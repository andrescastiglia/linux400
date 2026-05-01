use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use thiserror::Error;

pub const L400_STORAGE_BACKEND_ATTR: &str = "user.l400.storage_backend";
pub const L400_RECORD_LEN_ATTR: &str = "user.l400.record_len";
pub const L400_BASE_PF_ATTR: &str = "user.l400.base_pf";
pub const L400_FIELD_SCHEMA_ATTR: &str = "user.l400.field_schema";
pub const L400_KEY_FIELDS_ATTR: &str = "user.l400.key_fields";
pub const L400_PF_MEMBERS_ATTR: &str = "user.l400.pf_members";
pub const L400_DATA_FORMAT_VERSION_ATTR: &str = "user.l400.data.version";
pub const L400_DATA_FORMAT_VERSION: u32 = 1;
pub const L400_OUTQ_RETENTION_DAYS_ATTR: &str = "user.l400.outq.retention_days";
pub const L400_OUTQ_ROUTING_ATTR: &str = "user.l400.outq.routing";
pub const L400_OUTQ_DEFAULT_STATUS_ATTR: &str = "user.l400.outq.default_status";
pub const L400_TOOLCHAIN_MANIFEST_ATTR: &str = "user.l400.toolchain.manifest";
pub const L400_TOOLCHAIN_MANIFEST_VERSION: u32 = 1;
static SLED_DB_CACHE: OnceLock<Mutex<HashMap<PathBuf, sled::Db>>> = OnceLock::new();

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
        .unwrap_or(StorageBackend::Sled)
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
                StorageBackend::parse(&value).ok_or(StorageError::InvalidBackend(value))?;
            Ok(Some(backend))
        }
        None => Ok(None),
    }
}

pub fn open_sled_db(path: &Path) -> Result<sled::Db, sled::Error> {
    let cache = SLED_DB_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().expect("sled db cache poisoned");
    if let Some(db) = cache.get(path) {
        return Ok(db.clone());
    }

    let db = sled::open(path)?;
    cache.insert(path.to_path_buf(), db.clone());
    Ok(db)
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

pub fn file_checksum(path: &Path) -> Result<String, StorageError> {
    let bytes = fs::read(path)?;
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:016x}"))
}

pub fn build_toolchain_manifest(
    path: &Path,
    toolchain: &str,
    version: &str,
    source: Option<&str>,
) -> Result<String, StorageError> {
    let metadata = fs::metadata(path)?;
    let checksum = file_checksum(path)?;
    Ok(format!(
        "manifest_version={}\ntoolchain={}\ntoolchain_version={}\nsize={}\nchecksum={}\nsource={}\n",
        L400_TOOLCHAIN_MANIFEST_VERSION,
        toolchain,
        version,
        metadata.len(),
        checksum,
        source.unwrap_or("-")
    ))
}

pub fn write_toolchain_manifest(
    path: &Path,
    toolchain: &str,
    version: &str,
    source: Option<&str>,
) -> Result<(), StorageError> {
    let manifest = build_toolchain_manifest(path, toolchain, version, source)?;
    write_string_attr(path, L400_TOOLCHAIN_MANIFEST_ATTR, &manifest)?;
    write_string_attr(path, "user.l400.toolchain", toolchain)?;
    write_string_attr(path, "user.l400.toolchain_version", version)?;
    Ok(())
}

pub fn verify_toolchain_manifest(path: &Path) -> Result<(), String> {
    let manifest = read_string_attr(path, L400_TOOLCHAIN_MANIFEST_ATTR)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "missing toolchain manifest".to_string())?;
    let fields = manifest
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect::<HashMap<_, _>>();
    if fields.get("manifest_version").map(String::as_str) != Some("1") {
        return Err("unsupported toolchain manifest version".to_string());
    }
    let toolchain = fields.get("toolchain").map(String::as_str).unwrap_or("");
    if !matches!(toolchain, "clc" | "c400c") {
        return Err(format!("unsupported toolchain '{toolchain}'"));
    }
    let expected = fields
        .get("checksum")
        .ok_or_else(|| "toolchain manifest missing checksum".to_string())?;
    let actual = file_checksum(path).map_err(|error| error.to_string())?;
    if expected != &actual {
        return Err("toolchain manifest checksum mismatch".to_string());
    }
    Ok(())
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

    #[test]
    fn default_backend_matches_current_target() {
        assert_eq!(default_storage_backend(), StorageBackend::Sled);
    }

    #[test]
    fn toolchain_manifest_verifies_and_detects_tampering() {
        let dir = tempfile::tempdir().expect("tempdir");
        let program = dir.path().join("HELLO");
        std::fs::write(&program, b"#!/bin/sh\nexit 0\n").expect("write program");

        write_toolchain_manifest(&program, "clc", "0.2.0", Some("HELLO.CLP"))
            .expect("write manifest");
        verify_toolchain_manifest(&program).expect("verify manifest");

        std::fs::write(&program, b"#!/bin/sh\nexit 1\n").expect("tamper");
        assert!(verify_toolchain_manifest(&program).is_err());
    }
}
