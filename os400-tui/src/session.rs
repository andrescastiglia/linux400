use l400::l400_run_dir;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    pub user_profile: String,
    pub current_library: String,
    pub library_list: Vec<String>,
    pub last_message: Option<String>,
    pub job_id: u64,
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    inner: Arc<Mutex<SessionState>>,
    state_path: PathBuf,
}

impl SessionContext {
    pub fn new(job_id: u64) -> Self {
        let state = SessionState {
            user_profile: "QSECOFR".to_string(),
            current_library: "QGPL".to_string(),
            library_list: vec![
                "QGPL".to_string(),
                "QUSRSYS".to_string(),
                "QSYS".to_string(),
            ],
            last_message: None,
            job_id,
        };
        let state_path = l400_run_dir()
            .join("sessions")
            .join(format!("{job_id}.session"));
        let context = Self {
            inner: Arc::new(Mutex::new(state)),
            state_path,
        };
        #[cfg(not(test))]
        {
            context.apply_env();
            let _ = context.save();
        }
        context
    }

    pub fn snapshot(&self) -> SessionState {
        self.inner.lock().expect("session lock poisoned").clone()
    }

    pub fn sign_on(&self, user_profile: &str) {
        {
            let mut state = self.inner.lock().expect("session lock poisoned");
            state.user_profile = user_profile.trim().to_uppercase();
            state.current_library = "QGPL".to_string();
            state.library_list = vec![
                "QGPL".to_string(),
                "QUSRSYS".to_string(),
                "QSYS".to_string(),
            ];
            state.last_message = Some(format!("Signed on as {}", state.user_profile));
        }
        self.apply_env();
        let _ = self.save();
    }

    pub fn sign_off(&self) {
        {
            let mut state = self.inner.lock().expect("session lock poisoned");
            state.user_profile = "QSECOFR".to_string();
            state.current_library = "QGPL".to_string();
            state.library_list = vec![
                "QGPL".to_string(),
                "QUSRSYS".to_string(),
                "QSYS".to_string(),
            ];
            state.last_message = Some("Signed off".to_string());
        }
        let _ = std::fs::remove_file(&self.state_path);
        unsafe {
            std::env::remove_var("L400_USER");
            std::env::remove_var("L400_CURLIB");
            std::env::remove_var("L400_LIBLIST");
        }
    }

    pub fn set_current_library(&self, library: &str) {
        let library = library.trim().to_uppercase();
        {
            let mut state = self.inner.lock().expect("session lock poisoned");
            state.current_library = library.clone();
            if !state
                .library_list
                .iter()
                .any(|entry| entry.eq_ignore_ascii_case(&library))
            {
                state.library_list.insert(0, library.clone());
            }
            state.last_message = Some(format!("Current library changed to {library}"));
        }
        self.apply_env();
        let _ = self.save();
    }

    pub fn add_library(&self, library: &str) {
        let library = library.trim().to_uppercase();
        {
            let mut state = self.inner.lock().expect("session lock poisoned");
            if !state
                .library_list
                .iter()
                .any(|entry| entry.eq_ignore_ascii_case(&library))
            {
                state.library_list.insert(0, library.clone());
                state.last_message = Some(format!("{library} added to library list"));
            } else {
                state.last_message = Some(format!("{library} already in library list"));
            }
        }
        self.apply_env();
        let _ = self.save();
    }

    pub fn set_last_message(&self, message: impl Into<String>) {
        {
            let mut state = self.inner.lock().expect("session lock poisoned");
            state.last_message = Some(message.into());
        }
        let _ = self.save();
    }

    pub fn apply_env(&self) {
        let state = self.snapshot();
        unsafe {
            std::env::set_var("L400_USER", &state.user_profile);
            std::env::set_var("L400_CURLIB", &state.current_library);
            std::env::set_var("L400_LIBLIST", state.library_list.join(":"));
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let state = self.snapshot();
        let payload = format!(
            "job_id={}\nuser_profile={}\ncurrent_library={}\nlibrary_list={}\nlast_message={}\n",
            state.job_id,
            state.user_profile,
            state.current_library,
            state.library_list.join(":"),
            state.last_message.unwrap_or_default().replace('\n', " ")
        );
        std::fs::write(&self.state_path, payload)
    }

    pub fn state_path(&self) -> &PathBuf {
        &self.state_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => unsafe {
                    std::env::set_var(self.key, value);
                },
                None => unsafe {
                    std::env::remove_var(self.key);
                },
            }
        }
    }

    #[test]
    fn session_updates_env_and_persists_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let _guard = EnvGuard::set("L400_RUN_DIR", temp.path().to_str().unwrap());

        let session = SessionContext::new(42);
        session.sign_on("qpgmr");
        session.set_current_library("testlib");
        session.add_library("mylib");

        assert_eq!(std::env::var("L400_USER").as_deref(), Ok("QPGMR"));
        assert_eq!(std::env::var("L400_CURLIB").as_deref(), Ok("TESTLIB"));
        assert!(
            std::env::var("L400_LIBLIST")
                .expect("liblist")
                .contains("MYLIB")
        );
        assert!(session.state_path().exists());

        session.sign_off();
        assert!(!session.state_path().exists());
        assert_eq!(std::env::var("L400_USER").ok(), None);
        assert_eq!(session.snapshot().current_library, "QGPL");
    }
}
