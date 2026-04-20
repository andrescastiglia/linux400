use l400_ebpf_common::VALID_OBJ_TYPES;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

pub const L400_OBJTYPE_ATTR: &str = "user.l400.objtype";

#[derive(Error, Debug)]
pub enum ZfsError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid object type: {0}")]
    InvalidType(String),
    #[error("Command failed: {0}")]
    CommandFailed(String),
}

pub fn set_objtype(path: &Path, objtype: &str) -> Result<(), ZfsError> {
    if !validate_objtype(objtype) {
        return Err(ZfsError::InvalidType(objtype.to_string()));
    }
    xattr::set(path, L400_OBJTYPE_ATTR, objtype.as_bytes())?;
    Ok(())
}

pub fn get_objtype(path: &Path) -> Result<String, ZfsError> {
    match xattr::get(path, L400_OBJTYPE_ATTR)? {
        Some(val) => String::from_utf8(val).map_err(|_| {
            ZfsError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid UTF-8 in xattr",
            ))
        }),
        None => Err(ZfsError::InvalidType("No objtype attribute found".into())),
    }
}

pub fn validate_objtype(objtype: &str) -> bool {
    VALID_OBJ_TYPES.iter().any(|typ| typ.name == objtype)
}

pub fn path_is_on_zfs(path: &Path) -> bool {
    zfs_dataset_for_path(path).is_some()
}

pub fn zfs_dataset_for_path(path: &Path) -> Option<String> {
    let output = Command::new("df").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().next())
        .and_then(|value| {
            let dataset = value.to_string();
            let zfs_output = Command::new("zfs")
                .args(["list", "-H", "-o", "name", &dataset])
                .output()
                .ok()?;
            if !zfs_output.status.success() {
                return None;
            }

            let listed = String::from_utf8_lossy(&zfs_output.stdout);
            let listed = listed.lines().next()?.trim();
            if listed == dataset {
                Some(dataset)
            } else {
                None
            }
        })
}

pub fn zfs_xattr_mode(path: &Path) -> Option<String> {
    let dataset = zfs_dataset_for_path(path)?;
    let output = Command::new("zfs")
        .args(["get", "-H", "-o", "value", "xattr", &dataset])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn zfs_command_available() -> bool {
    Command::new("zfs")
        .arg("--help")
        .output()
        .map(|output| {
            output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty()
        })
        .unwrap_or(false)
}

pub fn create_dataset(dataset: &str) -> Result<(), ZfsError> {
    let output = Command::new("zfs").args(["create", dataset]).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(ZfsError::CommandFailed(
        String::from_utf8_lossy(&output.stderr).trim().to_string(),
    ))
}
