use crate::object::catalog_object;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UsrPrfError {
    #[error("Object error: {0}")]
    Object(#[from] crate::object::ObjectError),
    #[error("System user error: {0}")]
    System(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Profile already exists")]
    AlreadyExists,
    #[error("Profile not found")]
    NotFound,
}

const USRPRF_OBJTYPE: &str = "*USRPRF";
const QSYS_PATH: &str = "/l400/QSYS";

pub fn get_usrprf_path(name: &str) -> PathBuf {
    Path::new(QSYS_PATH).join(format!("{}.USRPRF", name.to_uppercase()))
}

fn user_exists(name: &str) -> bool {
    Command::new("id")
        .arg("-u")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn create_user_profile(name: &str, description: Option<&str>) -> Result<(), UsrPrfError> {
    let lower_name = name.to_lowercase();
    let upper_name = name.to_uppercase();

    // 1. Create system user if it doesn't exist
    if !user_exists(&lower_name) {
        let mut cmd = Command::new("useradd");
        cmd.arg("-r") // System account
           .arg("-s")
           .arg("/bin/false") // No shell by default unless specified otherwise
           .arg(&lower_name);
        
        let output = cmd.output()?;
        if !output.status.success() {
            return Err(UsrPrfError::System(format!(
                "Failed to create system user: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
    }

    // 2. Create the L400 *USRPRF object
    let path = get_usrprf_path(&upper_name);
    if path.exists() {
        return Err(UsrPrfError::AlreadyExists);
    }

    if !Path::new(QSYS_PATH).exists() {
        std::fs::create_dir_all(QSYS_PATH)?;
    }

    std::fs::File::create(&path)?;

    catalog_object(
        &path,
        USRPRF_OBJTYPE,
        Some("OS400"),
        description.or(Some("User Profile")),
    )?;

    Ok(())
}

pub fn delete_user_profile(name: &str, keep_system_user: bool) -> Result<(), UsrPrfError> {
    let lower_name = name.to_lowercase();
    let upper_name = name.to_uppercase();
    let path = get_usrprf_path(&upper_name);

    if !path.exists() {
        return Err(UsrPrfError::NotFound);
    }

    std::fs::remove_file(&path)?;

    if !keep_system_user && user_exists(&lower_name) {
        let output = Command::new("userdel").arg(&lower_name).output()?;
        if !output.status.success() {
            return Err(UsrPrfError::System(format!(
                "Failed to delete system user: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
    }

    Ok(())
}
