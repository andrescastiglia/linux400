use crate::dtaq::DataQueue;
use crate::object::resolve_l400_root;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuditError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub timestamp: String,
    pub event: String,
    pub user: String,
    pub object: String,
    pub message: String,
}

pub fn current_l400_user() -> String {
    std::env::var("L400_USER")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "QSECOFR".to_string())
        .trim()
        .to_uppercase()
}

pub fn qhst_path() -> PathBuf {
    resolve_l400_root().join("QSYS").join("QHST")
}

pub fn audit_event(
    event: &str,
    user: &str,
    object: impl AsRef<Path>,
    message: &str,
) -> Result<(), AuditError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    let object = object.as_ref().display().to_string();
    let line = format!(
        "ts={} event={} user={} object={} message={}\n",
        timestamp,
        event,
        user,
        object,
        message.split_whitespace().collect::<Vec<_>>().join("_")
    );

    let path = qhst_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(line.as_bytes())?;

    let dtaq_path = resolve_l400_root().join("QUSRSYS").join("QEZJOBLOG");
    if let Ok(queue) = DataQueue::open(&dtaq_path) {
        let _ = queue.snddtaq(line.trim_end().as_bytes());
    }

    Ok(())
}

pub fn read_audit_records(limit: usize) -> Result<Vec<AuditRecord>, AuditError> {
    let content = std::fs::read_to_string(qhst_path()).unwrap_or_default();
    let mut records = content
        .lines()
        .filter_map(parse_audit_line)
        .collect::<Vec<_>>();
    if limit > 0 && records.len() > limit {
        records = records.split_off(records.len() - limit);
    }
    Ok(records)
}

fn parse_audit_line(line: &str) -> Option<AuditRecord> {
    let mut timestamp = String::new();
    let mut event = String::new();
    let mut user = String::new();
    let mut object = String::new();
    let mut message = String::new();
    for part in line.split_whitespace() {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "ts" => timestamp = value.to_string(),
                "event" => event = value.to_string(),
                "user" => user = value.to_string(),
                "object" => object = value.to_string(),
                "message" => message = value.to_string(),
                _ => {}
            }
        }
    }
    (!event.is_empty()).then_some(AuditRecord {
        timestamp,
        event,
        user,
        object,
        message,
    })
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
    fn audit_event_writes_qhst() {
        let temp = tempfile::tempdir().expect("tempdir");
        let _root = EnvGuard::set("L400_ROOT", temp.path().to_str().unwrap());

        audit_event("DENIED", "QPGMR", temp.path().join("OBJ"), "no access").unwrap();
        let content = std::fs::read_to_string(qhst_path()).unwrap();
        assert!(content.contains("event=DENIED"));
        assert!(content.contains("user=QPGMR"));
    }
}
