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
    pub last_error: Option<String>,
}

impl LoaderStatus {
    pub fn new(mode: impl Into<String>, protection_active: bool, phase: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            protection_active,
            phase: phase.into(),
            bpf_path: None,
            last_error: None,
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
        if let Some(err) = &self.last_error {
            lines.push(format!("last_error={err}"));
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
                )))
            }
            None => {
                return Err(RuntimeStatusError::InvalidEntry(
                    "missing protection_active".to_string(),
                ))
            }
        };
        let phase = map
            .get("phase")
            .cloned()
            .ok_or_else(|| RuntimeStatusError::InvalidEntry("missing phase".to_string()))?;

        Ok(Self {
            mode,
            protection_active,
            phase,
            bpf_path: map.get("bpf_path").cloned(),
            last_error: map.get("last_error").cloned(),
        })
    }
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
        env::set_var("L400_RUN_DIR", root.path());

        let mut status = LoaderStatus::new("degraded", false, "fallback");
        status.bpf_path = Some("/opt/l400/hooks/l400-ebpf".to_string());
        status.last_error = Some("missing btf".to_string());
        write_loader_status(&status).unwrap();

        let parsed = read_loader_status().unwrap();
        assert_eq!(parsed, status);

        env::remove_var("L400_RUN_DIR");
    }
}
