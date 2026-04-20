use std::env;
use std::path::Path;
use thiserror::Error;

pub const L400_STORAGE_BACKEND_ATTR: &str = "user.l400.storage_backend";
pub const L400_RECORD_LEN_ATTR: &str = "user.l400.record_len";
pub const L400_BASE_PF_ATTR: &str = "user.l400.base_pf";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageBackend {
    Sled,
    BerkeleyDb,
}

impl StorageBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            StorageBackend::Sled => "sled",
            StorageBackend::BerkeleyDb => "berkeleydb",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "sled" => Some(StorageBackend::Sled),
            "berkeleydb" | "bdb" | "libdb" => Some(StorageBackend::BerkeleyDb),
            _ => None,
        }
    }
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid storage backend: {0}")]
    InvalidBackend(String),
    #[error("Unsupported storage backend in this build: {0}")]
    UnsupportedBackend(String),
}

pub fn default_storage_backend() -> StorageBackend {
    env::var("L400_STORAGE_BACKEND")
        .ok()
        .and_then(|value| StorageBackend::parse(&value))
        .unwrap_or(StorageBackend::BerkeleyDb)
}

pub fn write_storage_backend(path: &Path, backend: StorageBackend) -> Result<(), StorageError> {
    xattr::set(path, L400_STORAGE_BACKEND_ATTR, backend.as_str().as_bytes())?;
    Ok(())
}

pub fn read_storage_backend(path: &Path) -> Result<Option<StorageBackend>, StorageError> {
    let raw = xattr::get(path, L400_STORAGE_BACKEND_ATTR)?;
    match raw {
        Some(raw) => {
            let value = String::from_utf8(raw)
                .map_err(|_| StorageError::InvalidBackend("invalid UTF-8".to_string()))?;
            let backend =
                StorageBackend::parse(&value).ok_or_else(|| StorageError::InvalidBackend(value))?;
            Ok(Some(backend))
        }
        None => Ok(None),
    }
}

pub fn write_string_attr(path: &Path, attr: &str, value: &str) -> Result<(), StorageError> {
    xattr::set(path, attr, value.as_bytes())?;
    Ok(())
}

pub fn read_string_attr(path: &Path, attr: &str) -> Result<Option<String>, StorageError> {
    let raw = xattr::get(path, attr)?;
    match raw {
        Some(raw) => Ok(Some(String::from_utf8(raw).map_err(|_| {
            StorageError::InvalidBackend(format!("invalid UTF-8 in {attr}"))
        })?)),
        None => Ok(None),
    }
}

pub fn write_u32_attr(path: &Path, attr: &str, value: u32) -> Result<(), StorageError> {
    write_string_attr(path, attr, &value.to_string())
}

pub fn read_u32_attr(path: &Path, attr: &str) -> Result<Option<u32>, StorageError> {
    match read_string_attr(path, attr)? {
        Some(value) => Ok(Some(value.parse::<u32>().map_err(|_| {
            StorageError::InvalidBackend(format!("invalid integer in {attr}"))
        })?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_parser_accepts_aliases() {
        assert_eq!(StorageBackend::parse("sled"), Some(StorageBackend::Sled));
        assert_eq!(
            StorageBackend::parse("berkeleydb"),
            Some(StorageBackend::BerkeleyDb)
        );
        assert_eq!(
            StorageBackend::parse("bdb"),
            Some(StorageBackend::BerkeleyDb)
        );
        assert_eq!(StorageBackend::parse("nope"), None);
    }
}
