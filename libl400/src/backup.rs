use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Backup/Restore module for Linux/400
/// Focus: *SAVF (Save File) option only - no tapes or optical support
/// Uses mega.io as backend device for *SAVF operations

const SAVF_DIR: &str = "/var/lib/l400/savf";
const MEGA_IO_MOUNT: &str = "/mnt/mega_io";
const MEGA_CREDENTIALS: &str = "/etc/l400/mega_credentials";

/// Represents a *SAVF (Save File) object
#[derive(Debug, Clone)]
pub struct SavfInfo {
    pub name: String,
    pub library: String,
    pub size: u64,
    pub created: String,
    pub description: String,
}

/// Result of backup/restore operations
pub type SavResult<T> = Result<T, String>;

/// Initialize mega.io device support
/// Prompts for user credentials and mounts the device
pub fn init_mega_io(username: &str, password: &str) -> SavResult<()> {
    // Store credentials securely
    let cred_dir = Path::new(MEGA_CREDENTIALS).parent().unwrap();
    if !cred_dir.exists() {
        fs::create_dir_all(cred_dir)
            .map_err(|e| format!("Error creating credentials dir: {}", e))?;
    }

    // Write credentials (in production, use proper encryption)
    let cred_content = format!("username={}\npassword={}\n", username, password);
    fs::write(MEGA_CREDENTIALS, cred_content)
        .map_err(|e| format!("Error writing credentials: {}", e))?;

    // Set restrictive permissions
    let mut perms = fs::metadata(MEGA_CREDENTIALS)
        .map_err(|e| format!("Error reading credentials: {}", e))?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(MEGA_CREDENTIALS, perms)
        .map_err(|e| format!("Error setting permissions: {}", e))?;

    // Mount mega.io (assuming mega.io tool is installed)
    if !Path::new(MEGA_IO_MOUNT).exists() {
        fs::create_dir_all(MEGA_IO_MOUNT)
            .map_err(|e| format!("Error creating mount point: {}", e))?;
    }

    let output = Command::new("mega-login")
        .args(&[username, password])
        .output()
        .map_err(|e| format!("Error running mega-login: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "mega-login failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Check if mega.io is mounted
pub fn is_mega_io_mounted() -> bool {
    Path::new(MEGA_IO_MOUNT).exists()
        && Command::new("mountpoint")
            .arg(MEGA_IO_MOUNT)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

/// Create a *SAVF (Save File) - backup target
pub fn create_savf(library: &str, name: &str, description: &str) -> SavResult<PathBuf> {
    let savf_dir = Path::new(SAVF_DIR);
    if !savf_dir.exists() {
        fs::create_dir_all(savf_dir).map_err(|e| format!("Error creating SAVF dir: {}", e))?;
    }

    let savf_path = savf_dir.join(format!("{}.savf", name));

    // Create empty SAVF (will be populated during backup)
    let manifest = format!(
        "[savf]\nname = \"{}\"\nlibrary = \"{}\"\ndescription = \"{}\"\ncreated = \"{}\"\n",
        name, library, description, "2026-05-03"
    );

    fs::write(&savf_path, manifest).map_err(|e| format!("Error creating SAVF: {}", e))?;

    Ok(savf_path)
}

/// Save a library to *SAVF (SAVLIB command implementation)
pub fn savlib(library: &str, savf_name: &str, target: &str) -> SavResult<String> {
    let l400_root = std::env::var("L400_ROOT").unwrap_or_else(|_| "/l400".to_string());
    let lib_path = Path::new(&l400_root).join(library);

    if !lib_path.exists() {
        return Err(format!("Library {} not found", library));
    }

    // Determine SAVF location
    let savf_path = if target == "MEGA" {
        if !is_mega_io_mounted() {
            return Err("mega.io not mounted. Run init_mega_io first.".to_string());
        }
        PathBuf::from(MEGA_IO_MOUNT).join(format!("{}.savf", savf_name))
    } else {
        PathBuf::from(SAVF_DIR).join(format!("{}.savf", savf_name))
    };

    // Create tar archive with xattrs for the library
    let tar_output = Command::new("tar")
        .args(&[
            "--xattrs",
            "--xattrs-include=*",
            "-czf",
            savf_path.to_str().unwrap(),
            lib_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("Error creating tar archive: {}", e))?;

    if !tar_output.status.success() {
        return Err(format!(
            "tar failed: {}",
            String::from_utf8_lossy(&tar_output.stderr)
        ));
    }

    // Update SAVF manifest
    let manifest = format!(
        "[savf]\nname = \"{}\"\nlibrary = \"{}\"\ncreated = \"{}\"\ntarget = \"{}\"\nsize = {}\n",
        savf_name,
        library,
        "2026-05-03",
        target,
        fs::metadata(&savf_path).map(|m| m.len()).unwrap_or(0)
    );
    fs::write(&savf_path, manifest).map_err(|e| format!("Error writing manifest: {}", e))?;

    Ok(format!("Library {} saved to {:?}", library, savf_path))
}

/// Restore a library from *SAVF (RSTLIB command implementation)
pub fn rstlib(savf_name: &str, target_library: &str, source: &str) -> SavResult<String> {
    // Determine SAVF location
    let savf_path = if source == "MEGA" {
        if !is_mega_io_mounted() {
            return Err("mega.io not mounted. Run init_mega_io first.".to_string());
        }
        PathBuf::from(MEGA_IO_MOUNT).join(format!("{}.savf", savf_name))
    } else {
        PathBuf::from(SAVF_DIR).join(format!("{}.savf", savf_name))
    };

    if !savf_path.exists() {
        return Err(format!("SAVF {} not found", savf_name));
    }

    let l400_root = std::env::var("L400_ROOT").unwrap_or_else(|_| "/l400".to_string());
    let target_path = Path::new(&l400_root).join(target_library);

    // Extract tar archive with xattrs
    let tar_output = Command::new("tar")
        .args(&[
            "--xattrs",
            "--xattrs-include=*",
            "-xzf",
            savf_path.to_str().unwrap(),
            "-C",
            target_path.parent().unwrap().to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("Error extracting tar archive: {}", e))?;

    if !tar_output.status.success() {
        return Err(format!(
            "tar failed: {}",
            String::from_utf8_lossy(&tar_output.stderr)
        ));
    }

    // Run CHKOBJINT to verify integrity
    let check_output = Command::new("l400")
        .args(&["CHKOBJINT", &format!("OBJ({}/{})", target_library, "*ALL")])
        .output()
        .map_err(|e| format!("Error running CHKOBJINT: {}", e))?;

    if !check_output.status.success() {
        return Err(format!(
            "CHKOBJINT failed: {}",
            String::from_utf8_lossy(&check_output.stderr)
        ));
    }

    Ok(format!(
        "Library {} restored from {:?}",
        target_library, savf_path
    ))
}

/// Save object to *SAVF (SAVOBJ command implementation)
pub fn savobj(object: &str, library: &str, savf_name: &str, target: &str) -> SavResult<String> {
    let l400_root = std::env::var("L400_ROOT").unwrap_or_else(|_| "/l400".to_string());
    let obj_path = Path::new(&l400_root).join(library).join(object);

    if !obj_path.exists() {
        return Err(format!("Object {}/{} not found", library, object));
    }

    let savf_path = if target == "MEGA" {
        PathBuf::from(MEGA_IO_MOUNT).join(format!("{}.savf", savf_name))
    } else {
        PathBuf::from(SAVF_DIR).join(format!("{}.savf", savf_name))
    };

    // Create tar archive with xattrs
    let tar_output = Command::new("tar")
        .args(&[
            "--xattrs",
            "--xattrs-include=*",
            "-czf",
            savf_path.to_str().unwrap(),
            obj_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("Error creating tar archive: {}", e))?;

    if !tar_output.status.success() {
        return Err(format!(
            "tar failed: {}",
            String::from_utf8_lossy(&tar_output.stderr)
        ));
    }

    Ok(format!(
        "Object {}/{} saved to {:?}",
        library, object, savf_path
    ))
}

/// List *SAVF files
pub fn list_savf() -> SavResult<Vec<SavfInfo>> {
    let mut savfs = Vec::new();
    let savf_dir = Path::new(SAVF_DIR);

    if savf_dir.exists() {
        if let Ok(entries) = fs::read_dir(savf_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "savf") {
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();

                    let metadata = fs::metadata(&path).ok();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

                    savfs.push(SavfInfo {
                        name: name.clone(),
                        library: "*ALL".to_string(),
                        size,
                        created: "2026-05-03".to_string(),
                        description: format!("SAVF: {}", name),
                    });
                }
            }
        }
    }

    // Also check mega.io if mounted
    if is_mega_io_mounted() {
        let mega_dir = Path::new(MEGA_IO_MOUNT);
        if let Ok(entries) = fs::read_dir(mega_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "savf") {
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();

                    savfs.push(SavfInfo {
                        name: name.clone(),
                        library: "*ALL".to_string(),
                        size: fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                        created: "2026-05-03".to_string(),
                        description: format!("SAVF (mega.io): {}", name),
                    });
                }
            }
        }
    }

    Ok(savfs)
}

/// Execute CHKOBJINT to verify object integrity after restore
pub fn chkobjint(object: &str) -> SavResult<String> {
    let output = Command::new("l400")
        .args(&["CHKOBJINT", &format!("OBJ({})", object)])
        .output()
        .map_err(|e| format!("Error running CHKOBJINT: {}", e))?;

    if output.status.success() {
        Ok("OK".to_string())
    } else {
        Err(format!(
            "CHKOBJINT failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
