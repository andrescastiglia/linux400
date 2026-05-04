use crate::object::catalog_object;
use crate::storage::{read_string_attr, write_string_attr};
use std::os::unix::fs::MetadataExt;
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
    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
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

    // Log the creation
    crate::audit::audit_event(
        "USRPRF_CREATE",
        &crate::audit::current_l400_user(),
        &path,
        &format!("User profile {} created", upper_name),
    )
    .ok();

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

    // Log the deletion
    crate::audit::audit_event(
        "USRPRF_DELETE",
        &crate::audit::current_l400_user(),
        &path,
        &format!("User profile {} deleted", upper_name),
    )
    .ok();

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

/// User profile information for display
#[derive(Debug, Clone)]
pub struct UserProfileInfo {
    pub name: String,
    pub description: String,
    pub status: String, // "*ENABLED" or "*DISABLED"
    pub uid: u32,
    pub home_library: Option<String>,
    pub current_library: Option<String>,
    pub group_profiles: Vec<String>,
    pub owner: String,
    pub creation_date: String,
}

/// Change user profile attributes
pub fn change_user_profile(
    name: &str,
    description: Option<&str>,
    status: Option<&str>,
    password: Option<&str>,
    home_library: Option<&str>,
    current_library: Option<&str>,
    group_profiles: Option<&str>,
) -> Result<(), UsrPrfError> {
    let upper_name = name.to_uppercase();
    let path = get_usrprf_path(&upper_name);

    if !path.exists() {
        return Err(UsrPrfError::NotFound);
    }

    // Log the change attempt
    let mut changes = Vec::new();
    if description.is_some() {
        changes.push("TEXT");
    }
    if status.is_some() {
        changes.push("STATUS");
    }
    if password.is_some() {
        changes.push("PASSWORD");
    }
    if home_library.is_some() {
        changes.push("HOME_LIBRARY");
    }
    if current_library.is_some() {
        changes.push("CURRENT_LIBRARY");
    }
    if group_profiles.is_some() {
        changes.push("GROUP_PROFILES");
    }
    if !changes.is_empty() {
        crate::audit::audit_event(
            "USRPRF_CHANGE",
            &crate::audit::current_l400_user(),
            &path,
            &format!(
                "User profile {} changed: {}",
                upper_name,
                changes.join(", ")
            ),
        )
        .ok(); // Ignore audit errors for now
    }

    // Update description if provided
    if let Some(desc) = description {
        write_string_attr(&path, crate::object::L400_TEXT_ATTR, desc)
            .map_err(|e| UsrPrfError::System(format!("Failed to write description: {}", e)))?;
    }

    // Update status if provided
    if let Some(stat) = status {
        if stat != "*ENABLED" && stat != "*DISABLED" {
            return Err(UsrPrfError::InvalidParam(format!(
                "Invalid status: {}",
                stat
            )));
        }

        // Write status to xattr
        write_string_attr(&path, "user.l400.usrprf.status", stat)
            .map_err(|e| UsrPrfError::System(format!("Failed to write status: {}", e)))?;

        // If disabling, also lock the system user
        if stat == "*DISABLED" {
            let lower_name = name.to_lowercase();
            let output = Command::new("passwd").arg("-l").arg(&lower_name).output()?;
            if !output.status.success() {
                return Err(UsrPrfError::System(format!(
                    "Failed to disable user: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        } else if stat == "*ENABLED" {
            // Unlock the system user
            let lower_name = name.to_lowercase();
            let output = Command::new("passwd").arg("-u").arg(&lower_name).output()?;
            if !output.status.success() {
                return Err(UsrPrfError::System(format!(
                    "Failed to enable user: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }
    }

    // Update home library if provided
    if let Some(lib) = home_library {
        write_string_attr(&path, "user.l400.usrprf.home_library", lib)
            .map_err(|e| UsrPrfError::System(format!("Failed to write home library: {}", e)))?;
    }

    // Update current library if provided
    if let Some(lib) = current_library {
        write_string_attr(&path, "user.l400.usrprf.current_library", lib)
            .map_err(|e| UsrPrfError::System(format!("Failed to write current library: {}", e)))?;
    }

    // Update group profiles if provided (comma-separated)
    if let Some(groups) = group_profiles {
        write_string_attr(&path, "user.l400.usrprf.group_profiles", groups)
            .map_err(|e| UsrPrfError::System(format!("Failed to write group profiles: {}", e)))?;
    }

    // Change password if provided
    if let Some(pwd) = password {
        let lower_name = name.to_lowercase();
        // Use chpasswd for non-interactive password change
        use std::io::Write;
        let input = format!("{}:{}", lower_name, pwd);
        let mut child = Command::new("chpasswd")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| UsrPrfError::System(format!("Failed to spawn chpasswd: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .map_err(|e| UsrPrfError::System(format!("Failed to write password: {}", e)))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| UsrPrfError::System(format!("Failed to wait for chpasswd: {}", e)))?;

        if !output.status.success() {
            return Err(UsrPrfError::System(format!(
                "Failed to change password: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
    }

    Ok(())
}

/// Get user profile information for display
pub fn display_user_profile(name: &str) -> Result<UserProfileInfo, UsrPrfError> {
    let upper_name = name.to_uppercase();
    let path = get_usrprf_path(&upper_name);

    if !path.exists() {
        return Err(UsrPrfError::NotFound);
    }

    // Get metadata
    let metadata = std::fs::metadata(&path)?;
    let uid = metadata.uid();

    // Get description
    let description = read_string_attr(&path, crate::object::L400_TEXT_ATTR)
        .map_err(|e| UsrPrfError::System(format!("Failed to read description: {}", e)))?
        .unwrap_or_else(|| "User Profile".to_string());

    // Get status from xattr or default to enabled
    let status = read_string_attr(&path, "user.l400.usrprf.status")
        .map_err(|e| UsrPrfError::System(format!("Failed to read status: {}", e)))?
        .unwrap_or_else(|| "*ENABLED".to_string());

    // Get home library
    let home_library = read_string_attr(&path, "user.l400.usrprf.home_library")
        .map_err(|e| UsrPrfError::System(format!("Failed to read home library: {}", e)))?
        .filter(|s| !s.is_empty());

    // Get current library
    let current_library = read_string_attr(&path, "user.l400.usrprf.current_library")
        .map_err(|e| UsrPrfError::System(format!("Failed to read current library: {}", e)))?
        .filter(|s| !s.is_empty());

    // Get group profiles (comma-separated)
    let group_profiles = read_string_attr(&path, "user.l400.usrprf.group_profiles")
        .map_err(|e| UsrPrfError::System(format!("Failed to read group profiles: {}", e)))?
        .map(|s| s.split(',').map(|g| g.trim().to_string()).collect())
        .unwrap_or_else(Vec::new);

    // Get owner info (simplified - would need more logic for true owner tracking)
    let owner = "QSYS".to_string(); // Default owner

    // Get creation date from metadata
    let creation_date = metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(UserProfileInfo {
        name: upper_name,
        description,
        status,
        uid,
        home_library,
        current_library,
        group_profiles,
        owner,
        creation_date,
    })
}

/// List all user profiles
pub fn list_user_profiles() -> Result<Vec<String>, UsrPrfError> {
    let qsys = Path::new(QSYS_PATH);
    if !qsys.exists() {
        return Ok(vec![]);
    }

    let mut profiles = vec![];
    for entry in std::fs::read_dir(qsys)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "USRPRF" {
                if let Some(stem) = path.file_stem() {
                    profiles.push(stem.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(profiles)
}
