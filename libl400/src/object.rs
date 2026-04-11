use crate::zfs::{get_objtype, set_objtype, validate_objtype, ZfsError};
use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ObjectError {
    #[error("ZFS Metadata Error: {0}")]
    Zfs(#[from] ZfsError),
    #[error("File System Error: {0}")]
    Fs(#[from] std::io::Error),
    #[error("Target already exists")]
    AlreadyExists,
    #[error("Invalid Type: {0}")]
    InvalidType(String),
}

#[derive(Debug)]
pub struct L400Object {
    pub path: PathBuf,
    pub objtype: String,
}

pub fn create_object(lib_path: &Path, name: &str, objtype: &str) -> Result<PathBuf, ObjectError> {
    if !validate_objtype(objtype) {
        return Err(ObjectError::InvalidType(objtype.to_string()));
    }
    let target = lib_path.join(name);
    if target.exists() {
        return Err(ObjectError::AlreadyExists);
    }
    fs::File::create(&target)?;
    set_objtype(&target, objtype)?;
    Ok(target)
}

pub fn delete_object(path: &Path) -> Result<(), ObjectError> {
    // Valida que el archivo efectivamente tiene tipado de Linux/400
    let _ = get_objtype(path)?;
    fs::remove_file(path)?;
    Ok(())
}

pub fn copy_object(src: &Path, dst: &Path) -> Result<(), ObjectError> {
    let objtype = get_objtype(src)?;
    if dst.exists() {
        return Err(ObjectError::AlreadyExists);
    }
    fs::copy(src, dst)?;
    set_objtype(dst, &objtype)?;
    Ok(())
}

pub fn list_objects(lib_path: &Path) -> Result<Vec<L400Object>, ObjectError> {
    let mut objects = Vec::new();
    for entry in fs::read_dir(lib_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Ok(objtype) = get_objtype(&path) {
                objects.push(L400Object { path, objtype });
            }
        }
    }
    Ok(objects)
}

pub fn open_object_direct(path: &Path) -> Result<fs::File, ObjectError> {
    let _ = get_objtype(path)?;
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true);

    #[cfg(target_os = "linux")]
    options.custom_flags(rustix::fs::OFlags::DIRECT.bits() as i32);

    let file = options.open(path)?;
    Ok(file)
}

/// Valida que un buffer cumpla con los requisitos de alineación de O_DIRECT.
pub fn validate_alignment(buffer: &[u8], alignment: usize) -> bool {
    (buffer.as_ptr() as usize).is_multiple_of(alignment) && buffer.len().is_multiple_of(512)
}
