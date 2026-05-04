use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use thiserror::Error;

const DEFAULT_L400_RUN_DIR: &str = "/run/l400";
const LOADER_STATUS_FILE: &str = "loader-status";

#[derive(Error, Debug)]
pub enum RuntimeStatusError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid loader status entry: {0}")]
    InvalidEntry(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoaderStatus {
    pub mode: String,
    pub protection_active: bool,
    pub phase: String,
    pub bpf_path: Option<String>,
    pub attached_hooks: Option<String>,
    pub policy_version: Option<String>,
    pub runtime_version: Option<String>,
    pub ebpf_version: Option<String>,
    pub effective_mode: Option<String>,
    pub known_gaps: Option<String>,
    pub last_error: Option<String>,
    // Phase 9: Platform information
    pub btf_available: Option<bool>,
    pub kernel_version: Option<String>,
    pub cgroups_v2: Option<bool>,
    pub xattrs_supported: Option<bool>,
}

impl LoaderStatus {
    pub fn new(mode: impl Into<String>, protection_active: bool, phase: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            protection_active,
            phase: phase.into(),
            bpf_path: None,
            attached_hooks: None,
            policy_version: None,
            runtime_version: None,
            ebpf_version: None,
            effective_mode: None,
            known_gaps: None,
            last_error: None,
            // Phase 9: Platform information
            btf_available: None,
            kernel_version: None,
            cgroups_v2: None,
            xattrs_supported: None,
        }
    }

    fn to_lines(&self) -> String {
        let mut lines = vec![
            format!("mode={}", self.mode),
            format!(
                "protection_active={}",
                if self.protection_active { "1" } else { "0" }
            ),
            format!("phase={}", self.phase),
        ];
        if let Some(path) = &self.bpf_path {
            lines.push(format!("bpf_path={path}"));
        }
        if let Some(hooks) = &self.attached_hooks {
            lines.push(format!("attached_hooks={hooks}"));
        }
        if let Some(version) = &self.policy_version {
            lines.push(format!("policy_version={version}"));
        }
        if let Some(version) = &self.runtime_version {
            lines.push(format!("runtime_version={version}"));
        }
        if let Some(version) = &self.ebpf_version {
            lines.push(format!("ebpf_version={version}"));
        }
        if let Some(mode) = &self.effective_mode {
            lines.push(format!("effective_mode={mode}"));
        }
        if let Some(gaps) = &self.known_gaps {
            lines.push(format!("known_gaps={gaps}"));
        }
        if let Some(err) = &self.last_error {
            lines.push(format!("last_error={err}"));
        }
        // Phase 9: Platform information
        if let Some(btf) = &self.btf_available {
            lines.push(format!("btf_available={}", if *btf { "1" } else { "0" }));
        }
        if let Some(kernel) = &self.kernel_version {
            lines.push(format!("kernel_version={kernel}"));
        }
        if let Some(cgroups) = &self.cgroups_v2 {
            lines.push(format!("cgroups_v2={}", if *cgroups { "1" } else { "0" }));
        }
        if let Some(xattrs) = &self.xattrs_supported {
            lines.push(format!(
                "xattrs_supported={}",
                if *xattrs { "1" } else { "0" }
            ));
        }
        lines.push(String::new());
        lines.join("\n")
    }

    fn from_map(map: BTreeMap<String, String>) -> Result<Self, RuntimeStatusError> {
        let mode = map
            .get("mode")
            .cloned()
            .ok_or_else(|| RuntimeStatusError::InvalidEntry("missing mode".to_string()))?;
        let protection_active = match map.get("protection_active").map(String::as_str) {
            Some("1") => true,
            Some("0") => false,
            Some(value) => {
                return Err(RuntimeStatusError::InvalidEntry(format!(
                    "invalid protection_active={value}"
                )));
            }
            None => {
                return Err(RuntimeStatusError::InvalidEntry(
                    "missing protection_active".to_string(),
                ));
            }
        };
        let phase = map
            .get("phase")
            .cloned()
            .ok_or_else(|| RuntimeStatusError::InvalidEntry("missing phase".to_string()))?;

        let btf_available = map.get("btf_available").map(|v| v == "1");
        let kernel_version = map.get("kernel_version").cloned();
        let cgroups_v2 = map.get("cgroups_v2").map(|v| v == "1");
        let xattrs_supported = map.get("xattrs_supported").map(|v| v == "1");

        Ok(Self {
            mode,
            protection_active,
            phase,
            bpf_path: map.get("bpf_path").cloned(),
            attached_hooks: map.get("attached_hooks").cloned(),
            policy_version: map.get("policy_version").cloned(),
            runtime_version: map.get("runtime_version").cloned(),
            ebpf_version: map.get("ebpf_version").cloned(),
            effective_mode: map.get("effective_mode").cloned(),
            known_gaps: map.get("known_gaps").cloned(),
            last_error: map.get("last_error").cloned(),
            btf_available,
            kernel_version,
            cgroups_v2,
            xattrs_supported,
        })
    }
}

pub fn runtime_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn l400_run_dir() -> PathBuf {
    env::var_os("L400_RUN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_L400_RUN_DIR))
}

pub fn loader_status_path() -> PathBuf {
    l400_run_dir().join(LOADER_STATUS_FILE)
}

pub fn write_loader_status(status: &LoaderStatus) -> Result<(), RuntimeStatusError> {
    let run_dir = l400_run_dir();
    std::fs::create_dir_all(&run_dir)?;
    std::fs::write(loader_status_path(), status.to_lines())?;
    Ok(())
}

pub fn read_loader_status() -> Result<LoaderStatus, RuntimeStatusError> {
    let content = std::fs::read_to_string(loader_status_path())?;
    let mut map = BTreeMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.to_string(), value.to_string());
        }
    }
    LoaderStatus::from_map(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loader_status_round_trip() {
        let root = tempdir().unwrap();
        unsafe {
            env::set_var("L400_RUN_DIR", root.path());
        }

        let mut status = LoaderStatus::new("degraded", false, "fallback");
        status.bpf_path = Some("/opt/l400/hooks/l400-ebpf".to_string());
        status.attached_hooks = Some("file_open,bprm_creds_from_file,bprm_check_security".into());
        status.policy_version = Some("v1.0".into());
        status.runtime_version = Some(runtime_version().into());
        status.ebpf_version = Some("0.2.0".into());
        status.effective_mode = Some("degraded".into());
        status.known_gaps = Some("test-gap".into());
        status.last_error = Some("missing btf".to_string());
        // Phase 9: Platform information
        status.btf_available = Some(true);
        status.kernel_version = Some("6.11.0".to_string());
        status.cgroups_v2 = Some(true);
        status.xattrs_supported = Some(true);
        write_loader_status(&status).unwrap();

        let parsed = read_loader_status().unwrap();
        assert_eq!(parsed, status);

        unsafe {
            env::remove_var("L400_RUN_DIR");
        }
    }
}
