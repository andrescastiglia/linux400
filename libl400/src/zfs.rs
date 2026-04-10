use l400_ebpf_common::VALID_OBJ_TYPES;
use std::path::Path;
use thiserror::Error;

pub const L400_OBJTYPE_ATTR: &str = "user.l400.objtype";

#[derive(Error, Debug)]
pub enum ZfsError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid object type: {0}")]
    InvalidType(String),
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
