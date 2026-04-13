use crate::zfs::{get_objtype, set_objtype, validate_objtype, ZfsError};
use std::env;
use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DEFAULT_L400_ROOT: &str = "/l400";
pub const L400_OBJATTR_ATTR: &str = "user.l400.objattr";
pub const L400_TEXT_ATTR: &str = "user.l400.text";
pub const L400_OWNER_ATTR: &str = "user.l400.owner";

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
    #[error("Invalid library path: {0}")]
    InvalidLibrary(String),
    #[error("Not Found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L400Object {
    pub path: PathBuf,
    pub library: Option<String>,
    pub name: String,
    pub objtype: String,
    pub attribute: Option<String>,
    pub text: Option<String>,
}

pub fn resolve_l400_root() -> PathBuf {
    env::var_os("L400_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_L400_ROOT))
}

fn read_l400_attr(path: &Path, attr: &str) -> Result<Option<String>, ObjectError> {
    let value = xattr::get(path, attr)?;
    value
        .map(|raw| {
            String::from_utf8(raw).map_err(|_| {
                ObjectError::Fs(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid UTF-8 in {}", attr),
                ))
            })
        })
        .transpose()
}

fn write_l400_attr(path: &Path, attr: &str, value: Option<&str>) -> Result<(), ObjectError> {
    match value {
        Some(value) => xattr::set(path, attr, value.as_bytes())?,
        None => {
            let _ = xattr::remove(path, attr);
        }
    }
    Ok(())
}

fn current_owner_name() -> Option<String> {
    env::var("SUDO_USER")
        .ok()
        .or_else(|| env::var("USER").ok())
        .filter(|value| !value.is_empty())
}

fn object_default_attribute(objtype: &str) -> Option<&'static str> {
    match objtype {
        "*LIB" => Some("LIB"),
        "*PGM" => Some("ELF"),
        "*USRPRF" => Some("USRPRF"),
        "*CMD" => Some("CMD"),
        "*SRVPGM" => Some("SRVPGM"),
        "*OUTQ" => Some("OUTQ"),
        _ => None,
    }
}

fn library_name_from_path(path: &Path) -> Option<String> {
    path.parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().to_string())
}

fn create_path_for_type(path: &Path, objtype: &str) -> Result<(), ObjectError> {
    match objtype {
        "*LIB" => fs::create_dir(path)?,
        _ => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::File::create(path)?;
        }
    }
    Ok(())
}

fn validate_library_path(lib_path: &Path) -> Result<(), ObjectError> {
    if !lib_path.exists() {
        return Err(ObjectError::NotFound(lib_path.display().to_string()));
    }
    if !lib_path.is_dir() {
        return Err(ObjectError::InvalidLibrary(lib_path.display().to_string()));
    }
    let objtype = get_objtype(lib_path)?;
    if objtype != "*LIB" {
        return Err(ObjectError::InvalidLibrary(lib_path.display().to_string()));
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), ObjectError> {
    fs::create_dir(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub fn catalog_object(
    path: &Path,
    objtype: &str,
    attribute: Option<&str>,
    text: Option<&str>,
) -> Result<(), ObjectError> {
    if !path.exists() {
        return Err(ObjectError::NotFound(path.display().to_string()));
    }
    if !validate_objtype(objtype) {
        return Err(ObjectError::InvalidType(objtype.to_string()));
    }

    set_objtype(path, objtype)?;
    write_l400_attr(
        path,
        L400_OBJATTR_ATTR,
        attribute.or_else(|| object_default_attribute(objtype)),
    )?;
    write_l400_attr(path, L400_TEXT_ATTR, text)?;
    write_l400_attr(path, L400_OWNER_ATTR, current_owner_name().as_deref())?;
    Ok(())
}

pub fn create_library(root_path: &Path, name: &str) -> Result<PathBuf, ObjectError> {
    fs::create_dir_all(root_path)?;
    let target = root_path.join(name);
    if target.exists() {
        return Err(ObjectError::AlreadyExists);
    }
    create_path_for_type(&target, "*LIB")?;
    catalog_object(&target, "*LIB", Some("LIB"), Some("Linux/400 library"))?;
    Ok(target)
}

pub fn ensure_library(root_path: &Path, name: &str) -> Result<PathBuf, ObjectError> {
    let target = root_path.join(name);
    if target.exists() {
        validate_library_path(&target)?;
        return Ok(target);
    }
    create_library(root_path, name)
}

pub fn create_object(lib_path: &Path, name: &str, objtype: &str) -> Result<PathBuf, ObjectError> {
    create_object_with_metadata(
        lib_path,
        name,
        objtype,
        object_default_attribute(objtype),
        None,
    )
}

pub fn create_object_with_metadata(
    lib_path: &Path,
    name: &str,
    objtype: &str,
    attribute: Option<&str>,
    text: Option<&str>,
) -> Result<PathBuf, ObjectError> {
    validate_library_path(lib_path)?;
    if !validate_objtype(objtype) {
        return Err(ObjectError::InvalidType(objtype.to_string()));
    }

    let target = lib_path.join(name);
    if target.exists() {
        return Err(ObjectError::AlreadyExists);
    }

    create_path_for_type(&target, objtype)?;
    catalog_object(&target, objtype, attribute, text)?;
    Ok(target)
}

pub fn lookup_object(lib_path: &Path, name: &str) -> Result<L400Object, ObjectError> {
    validate_library_path(lib_path)?;
    let target = lib_path.join(name);
    if !target.exists() {
        return Err(ObjectError::NotFound(target.display().to_string()));
    }
    describe_object(&target)
}

pub fn describe_object(path: &Path) -> Result<L400Object, ObjectError> {
    let objtype = get_objtype(path)?;
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    Ok(L400Object {
        path: path.to_path_buf(),
        library: library_name_from_path(path),
        name,
        objtype,
        attribute: read_l400_attr(path, L400_OBJATTR_ATTR)?,
        text: read_l400_attr(path, L400_TEXT_ATTR)?,
    })
}

pub fn delete_object(path: &Path) -> Result<(), ObjectError> {
    let _ = get_objtype(path)?;
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn copy_object(src: &Path, dst: &Path) -> Result<(), ObjectError> {
    let source = describe_object(src)?;
    if dst.exists() {
        return Err(ObjectError::AlreadyExists);
    }

    if src.is_dir() {
        copy_dir_recursive(src, dst)?;
    } else {
        fs::copy(src, dst)?;
    }

    catalog_object(
        dst,
        &source.objtype,
        source.attribute.as_deref(),
        source.text.as_deref(),
    )?;
    Ok(())
}

pub fn list_objects(lib_path: &Path) -> Result<Vec<L400Object>, ObjectError> {
    validate_library_path(lib_path)?;
    let mut objects = Vec::new();
    for entry in fs::read_dir(lib_path)? {
        let entry = entry?;
        let path = entry.path();
        if let Ok(obj) = describe_object(&path) {
            objects.push(obj);
        }
    }
    objects.sort_by(|left, right| left.name.cmp(&right.name));
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_root() -> TempDir {
        tempfile::tempdir().expect("No se pudo crear root temporal")
    }

    #[test]
    fn create_and_list_library_objects() {
        let root = temp_root();
        let lib = create_library(root.path(), "QSYS").expect("create_library falló");

        create_object_with_metadata(&lib, "DEMOUSR", "*USRPRF", Some("USRPRF"), Some("Demo"))
            .expect("create_object_with_metadata falló");
        create_object_with_metadata(&lib, "HELLO", "*PGM", Some("C"), Some("Programa demo"))
            .expect("create_object_with_metadata falló");

        let objects = list_objects(&lib).expect("list_objects falló");
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].name, "DEMOUSR");
        assert_eq!(objects[1].attribute.as_deref(), Some("C"));
    }

    #[test]
    fn lookup_and_copy_preserve_metadata() {
        let root = temp_root();
        let lib = create_library(root.path(), "QGPL").expect("create_library falló");
        let source =
            create_object_with_metadata(&lib, "SOURCE", "*PGM", Some("CL"), Some("Programa CL"))
                .expect("create_object_with_metadata falló");

        let dest = lib.join("COPY");
        copy_object(&source, &dest).expect("copy_object falló");

        let copied = describe_object(&dest).expect("describe_object falló");
        assert_eq!(copied.objtype, "*PGM");
        assert_eq!(copied.attribute.as_deref(), Some("CL"));
        assert_eq!(copied.text.as_deref(), Some("Programa CL"));
    }

    #[test]
    fn delete_directory_object_works_for_libraries() {
        let root = temp_root();
        let lib = create_library(root.path(), "QTEMP").expect("create_library falló");
        assert!(lib.exists());
        delete_object(&lib).expect("delete_object falló");
        assert!(!lib.exists());
    }

    #[test]
    fn create_object_requires_l400_library() {
        let root = temp_root();
        let plain_dir = root.path().join("plain");
        fs::create_dir(&plain_dir).unwrap();

        let result = create_object(&plain_dir, "BADOBJ", "*PGM");
        assert!(matches!(
            result,
            Err(ObjectError::Zfs(_) | ObjectError::InvalidLibrary(_))
        ));
    }
}
