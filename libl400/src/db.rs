use crate::bdb_native::{BdbError, BdbHandle};
use crate::object::{ObjectError, catalog_object};
use crate::storage::{
    L400_BASE_PF_ATTR, L400_FIELD_SCHEMA_ATTR, L400_KEY_FIELDS_ATTR, L400_PF_MEMBERS_ATTR,
    L400_RECORD_LEN_ATTR, StorageBackend, StorageError, default_storage_backend, open_sled_db,
    read_storage_backend, read_string_attr, read_u32_attr, write_storage_backend,
    write_string_attr, write_u32_attr,
};
use crate::zfs::{ZfsError, get_objtype, validate_objtype};
use sled::{Db, Tree};
use std::path::Path;
use thiserror::Error;

pub type Record = Vec<u8>;
pub type RecordPair = (Record, Record);
pub type RecordSet = Vec<RecordPair>;
pub const DEFAULT_PF_MEMBER: &str = "PF_MEMBER";

#[derive(Error, Debug)]
pub enum DbError {
    #[error("ZFS Metadata Error: {0}")]
    Zfs(#[from] ZfsError),
    #[error("FS Error: {0}")]
    Fs(#[from] std::io::Error),
    #[error("Sled Error: {0}")]
    Sled(#[from] sled::Error),
    #[error("Berkeley DB Error: {0}")]
    Bdb(#[from] BdbError),
    #[error("Invalid Object Type: {0}")]
    InvalidType(String),
    #[error("Object Error: {0}")]
    Object(#[from] ObjectError),
    #[error("Already Exists")]
    AlreadyExists,
    #[error("Record out of bounds / Invalid Schema")]
    InvalidRecord,
    #[error("Not Found")]
    NotFound,
    #[error("Storage Error: {0}")]
    Storage(#[from] StorageError),
    #[error("Invalid Query: {0}")]
    InvalidQuery(String),
}

enum PhysicalFileStorage {
    Sled { db: Db, tree: Tree },
    BerkeleyDb { db: BdbHandle },
}

enum LogicalFileStorage {
    Sled { db: Db, index: Tree },
    BerkeleyDb { db: BdbHandle },
}

// ─── Physical File (*FILE PF) ─────────────────────────────────────────────────

pub struct PhysicalFile {
    pub name: String,
    pub path: std::path::PathBuf,
    pub backend: StorageBackend,
    pub record_len: u32,
    storage: PhysicalFileStorage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PfField {
    pub name: String,
    pub type_: String,
    pub length: u32,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PfSchema {
    pub record_len: u32,
    pub fields: Vec<PfField>,
    pub key_fields: Vec<String>,
}

impl PfSchema {
    pub fn minimal(record_len: u32) -> Self {
        Self {
            record_len,
            fields: Vec::new(),
            key_fields: vec!["KEY".to_string()],
        }
    }
}

fn open_sled_pf(path: &Path) -> Result<PhysicalFileStorage, DbError> {
    let db = open_sled_db(path)?;
    let tree = db.open_tree(DEFAULT_PF_MEMBER)?;
    Ok(PhysicalFileStorage::Sled { db, tree })
}

fn pf_member_tree_name(member: &str) -> String {
    let member = member.trim().to_uppercase();
    if member.is_empty() || member == DEFAULT_PF_MEMBER {
        DEFAULT_PF_MEMBER.to_string()
    } else {
        format!("PF_MEMBER_{member}")
    }
}

fn open_sled_pf_member(path: &Path, member: &str) -> Result<PhysicalFileStorage, DbError> {
    let db = open_sled_db(path)?;
    let tree = db.open_tree(pf_member_tree_name(member).as_bytes())?;
    Ok(PhysicalFileStorage::Sled { db, tree })
}

fn open_bdb_pf(path: &Path, create: bool) -> Result<PhysicalFileStorage, DbError> {
    let db = BdbHandle::open(path, create)?;
    Ok(PhysicalFileStorage::BerkeleyDb { db })
}

pub fn create_pf(lib_path: &Path, name: &str, record_len: usize) -> Result<PhysicalFile, DbError> {
    if get_objtype(lib_path)? != "*LIB" {
        return Err(DbError::InvalidType(
            "target library must be a *LIB".to_string(),
        ));
    }

    let target = lib_path.join(name);
    if target.exists() {
        return Err(DbError::AlreadyExists);
    }

    if !validate_objtype("*FILE") {
        return Err(DbError::InvalidType("*FILE".to_string()));
    }

    let backend = default_storage_backend();
    let storage = match backend {
        StorageBackend::Sled => open_sled_pf(&target)?,
        StorageBackend::BerkeleyDb => open_bdb_pf(&target, true)?,
    };

    catalog_object(&target, "*FILE", Some("PF"), Some("Physical file"))?;
    write_storage_backend(&target, backend)?;
    write_u32_attr(&target, L400_RECORD_LEN_ATTR, record_len as u32)?;
    write_string_attr(&target, L400_KEY_FIELDS_ATTR, "KEY")?;
    write_string_attr(&target, L400_PF_MEMBERS_ATTR, DEFAULT_PF_MEMBER)?;

    Ok(PhysicalFile {
        name: name.to_string(),
        path: target.to_path_buf(),
        backend,
        record_len: record_len as u32,
        storage,
    })
}

pub fn write_pf_schema(path: &Path, schema: &PfSchema) -> Result<(), DbError> {
    write_u32_attr(path, L400_RECORD_LEN_ATTR, schema.record_len)?;
    let fields = schema
        .fields
        .iter()
        .map(|field| {
            format!(
                "{}:{}:{}:{}",
                field.name,
                field.type_,
                field.length,
                field.text.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    write_string_attr(path, L400_FIELD_SCHEMA_ATTR, &fields)?;
    write_string_attr(path, L400_KEY_FIELDS_ATTR, &schema.key_fields.join(","))?;
    Ok(())
}

pub fn read_pf_schema(path: &Path) -> Result<PfSchema, DbError> {
    let record_len = read_u32_attr(path, L400_RECORD_LEN_ATTR)?.unwrap_or_default();
    let fields = read_string_attr(path, L400_FIELD_SCHEMA_ATTR)?
        .unwrap_or_default()
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| {
            let mut pieces = part.splitn(4, ':');
            let name = pieces.next()?.trim().to_uppercase();
            let type_ = pieces.next().unwrap_or("CHAR").trim().to_uppercase();
            let length = pieces
                .next()
                .and_then(|value| value.trim().parse::<u32>().ok())
                .unwrap_or_default();
            let text = pieces
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            Some(PfField {
                name,
                type_,
                length,
                text,
            })
        })
        .collect::<Vec<_>>();
    let key_fields = read_string_attr(path, L400_KEY_FIELDS_ATTR)?
        .unwrap_or_else(|| "KEY".to_string())
        .split(',')
        .map(|field| field.trim().to_uppercase())
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    Ok(PfSchema {
        record_len,
        fields,
        key_fields,
    })
}

pub fn list_pf_members(path: &Path) -> Result<Vec<String>, DbError> {
    Ok(read_string_attr(path, L400_PF_MEMBERS_ATTR)?
        .unwrap_or_else(|| DEFAULT_PF_MEMBER.to_string())
        .split(',')
        .map(|member| member.trim().to_uppercase())
        .filter(|member| !member.is_empty())
        .collect())
}

pub fn add_pf_member(path: &Path, member: &str) -> Result<(), DbError> {
    let member = member.trim().to_uppercase();
    if member.is_empty() {
        return Err(DbError::InvalidQuery("member name is empty".to_string()));
    }
    let backend = read_storage_backend(path)?.unwrap_or(default_storage_backend());
    if backend == StorageBackend::Sled {
        let _ = open_sled_pf_member(path, &member)?;
    }
    let mut members = list_pf_members(path)?;
    if !members
        .iter()
        .any(|entry| entry.eq_ignore_ascii_case(&member))
    {
        members.push(member);
    }
    write_string_attr(path, L400_PF_MEMBERS_ATTR, &members.join(","))?;
    Ok(())
}

impl PhysicalFile {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let backend = read_storage_backend(path)?.unwrap_or(default_storage_backend());
        let storage = match backend {
            StorageBackend::Sled => open_sled_pf(path)?,
            StorageBackend::BerkeleyDb => open_bdb_pf(path, false)?,
        };

        Ok(PhysicalFile {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.to_path_buf(),
            backend,
            record_len: read_u32_attr(path, L400_RECORD_LEN_ATTR)?.unwrap_or_default(),
            storage,
        })
    }

    pub fn open_member(path: &Path, member: &str) -> Result<Self, DbError> {
        let backend = read_storage_backend(path)?.unwrap_or(default_storage_backend());
        let storage = match backend {
            StorageBackend::Sled => open_sled_pf_member(path, member)?,
            StorageBackend::BerkeleyDb => open_bdb_pf(path, false)?,
        };

        Ok(PhysicalFile {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.to_path_buf(),
            backend,
            record_len: read_u32_attr(path, L400_RECORD_LEN_ATTR)?.unwrap_or_default(),
            storage,
        })
    }

    pub fn write_rcd(&self, key: &[u8], buffer: &[u8]) -> Result<(), DbError> {
        self.validate_write(key, buffer)?;
        let old = match self.chain_rcd(key) {
            Ok(old) => Some(old),
            Err(DbError::NotFound) => None,
            Err(error) => return Err(error),
        };
        match &self.storage {
            PhysicalFileStorage::Sled { db, tree } => {
                tree.insert(key, buffer)?;
                db.flush()?;
            }
            PhysicalFileStorage::BerkeleyDb { db } => {
                db.put(key, buffer)?;
            }
        }
        if let Some(old) = old {
            self.delete_dependent_lfs(key, &old)?;
        }
        self.update_dependent_lfs(key, buffer)?;
        Ok(())
    }

    fn validate_write(&self, key: &[u8], buffer: &[u8]) -> Result<(), DbError> {
        if key.is_empty() {
            return Err(DbError::InvalidRecord);
        }
        if self.record_len > 0 && buffer.len() > self.record_len as usize {
            return Err(DbError::InvalidRecord);
        }
        let schema =
            read_pf_schema(&self.path).unwrap_or_else(|_| PfSchema::minimal(self.record_len));
        for field in schema.fields {
            if field.name == "DATA"
                && field.type_ == "NUM"
                && !buffer
                    .iter()
                    .all(|byte| byte.is_ascii_digit() || *byte == b'.')
            {
                return Err(DbError::InvalidRecord);
            }
            if field.name == "KEY"
                && field.type_ == "NUM"
                && !key
                    .iter()
                    .all(|byte| byte.is_ascii_digit() || *byte == b'.')
            {
                return Err(DbError::InvalidRecord);
            }
        }
        Ok(())
    }

    pub fn append_rcd(&self, buffer: &[u8]) -> Result<u64, DbError> {
        let rrn = match &self.storage {
            PhysicalFileStorage::Sled { db, .. } => db.generate_id()? + 1,
            PhysicalFileStorage::BerkeleyDb { db } => match db.last_key()? {
                Some(raw) => String::from_utf8_lossy(&raw).parse::<u64>().unwrap_or(0) + 1,
                None => 1,
            },
        };
        self.write_rcd(rrn.to_string().as_bytes(), buffer)?;
        Ok(rrn)
    }

    pub fn chain_rcd(&self, key: &[u8]) -> Result<Vec<u8>, DbError> {
        match &self.storage {
            PhysicalFileStorage::Sled { tree, .. } => match tree.get(key)? {
                Some(ivec) => Ok(ivec.to_vec()),
                None => Err(DbError::NotFound),
            },
            PhysicalFileStorage::BerkeleyDb { db } => db.get(key).map_err(|err| match err {
                BdbError::NotFound => DbError::NotFound,
                other => DbError::Bdb(other),
            }),
        }
    }

    pub fn read_all(&self) -> Result<RecordSet, DbError> {
        match &self.storage {
            PhysicalFileStorage::Sled { tree, .. } => {
                let mut result = Vec::new();
                for item in tree.iter() {
                    let (k, v) = item?;
                    result.push((k.to_vec(), v.to_vec()));
                }
                Ok(result)
            }
            PhysicalFileStorage::BerkeleyDb { db } => Ok(db.read_all()?),
        }
    }

    pub fn delete_rcd(&self, key: &[u8]) -> Result<(), DbError> {
        let old = self.chain_rcd(key).ok();
        match &self.storage {
            PhysicalFileStorage::Sled { db, tree } => {
                tree.remove(key)?;
                db.flush()?;
            }
            PhysicalFileStorage::BerkeleyDb { db } => {
                db.delete(key).map_err(|err| match err {
                    BdbError::NotFound => DbError::NotFound,
                    other => DbError::Bdb(other),
                })?;
            }
        }
        if let Some(old) = old {
            self.delete_dependent_lfs(key, &old)?;
        }
        Ok(())
    }

    pub fn clear(&self) -> Result<(), DbError> {
        let keys = self
            .read_all()?
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();
        for key in keys {
            self.delete_rcd(&key)?;
        }
        Ok(())
    }

    fn update_dependent_lfs(&self, key: &[u8], buffer: &[u8]) -> Result<(), DbError> {
        for lf_path in dependent_lfs_for_pf(&self.path)? {
            let lf = LogicalFile::open(&lf_path)?;
            lf.insert_idx(buffer, key)?;
        }
        Ok(())
    }

    fn delete_dependent_lfs(&self, _key: &[u8], old_buffer: &[u8]) -> Result<(), DbError> {
        for lf_path in dependent_lfs_for_pf(&self.path)? {
            let lf = LogicalFile::open(&lf_path)?;
            let _ = lf.delete_idx(old_buffer);
        }
        Ok(())
    }
}

fn dependent_lfs_for_pf(pf_path: &Path) -> Result<Vec<std::path::PathBuf>, DbError> {
    let Some(parent) = pf_path.parent() else {
        return Ok(Vec::new());
    };
    let pf = pf_path.to_string_lossy().to_string();
    let mut result = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let entry = entry?;
        let path = entry.path();
        if path == pf_path {
            continue;
        }
        if let Ok(Some(base)) = read_string_attr(&path, L400_BASE_PF_ATTR) {
            if base == pf {
                result.push(path);
            }
        }
    }
    Ok(result)
}

// ─── Logical File (*FILE LF) ──────────────────────────────────────────────────
//
// Un Archivo Lógico (LF) es un índice secundario sobre un Physical File (PF).
// En OS/400, el LF reordena o filtra los registros del PF por un campo clave
// diferente. Aquí lo emulamos como:
//   key = campo_clave_secundario  →  value = clave_primaria_del_PF

pub struct LogicalFile {
    pub name: String,
    pub backend: StorageBackend,
    pub base_pf: String,
    storage: LogicalFileStorage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlStatementResult {
    Query(QueryResult),
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueryFilter {
    column: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectQuery {
    library: Option<String>,
    file: String,
    columns: Vec<String>,
    filter: Option<QueryFilter>,
    order_by: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InsertStatement {
    table: TableRef,
    columns: Option<Vec<String>>,
    values: Vec<String>,
}

fn open_sled_lf_from_db(name: &str, db: Db) -> Result<LogicalFileStorage, DbError> {
    let index_tree_name = format!("LF_IDX_{name}");
    let index = db.open_tree(index_tree_name.as_bytes())?;
    Ok(LogicalFileStorage::Sled { db, index })
}

fn open_sled_lf(path: &Path, name: &str, pf_path: &Path) -> Result<LogicalFileStorage, DbError> {
    let _ = path;
    let db = open_sled_db(pf_path)?;
    open_sled_lf_from_db(name, db)
}

fn open_bdb_lf(path: &Path, create: bool) -> Result<LogicalFileStorage, DbError> {
    let db = BdbHandle::open(path, create)?;
    Ok(LogicalFileStorage::BerkeleyDb { db })
}

pub fn create_lf(
    lib_path: &Path,
    name: &str,
    over_pf: &PhysicalFile,
) -> Result<LogicalFile, DbError> {
    create_lf_filtered(lib_path, name, over_pf, None, None)
}

pub fn create_lf_filtered(
    lib_path: &Path,
    name: &str,
    over_pf: &PhysicalFile,
    select_value: Option<&str>,
    omit_value: Option<&str>,
) -> Result<LogicalFile, DbError> {
    if get_objtype(lib_path)? != "*LIB" {
        return Err(DbError::InvalidType(
            "target library must be a *LIB".to_string(),
        ));
    }

    if !validate_objtype("*FILE") {
        return Err(DbError::InvalidType("*FILE".to_string()));
    }

    let lf_path = lib_path.join(name);
    if lf_path.exists() {
        return Err(DbError::AlreadyExists);
    }

    let storage = match (&over_pf.backend, &over_pf.storage) {
        (StorageBackend::Sled, PhysicalFileStorage::Sled { db, .. }) => {
            std::fs::create_dir_all(&lf_path)?;
            open_sled_lf_from_db(name, db.clone())?
        }
        (StorageBackend::BerkeleyDb, PhysicalFileStorage::BerkeleyDb { .. }) => {
            open_bdb_lf(&lf_path, true)?
        }
        _ => {
            return Err(DbError::Storage(StorageError::InvalidBackend(
                "physical file storage/backend mismatch".to_string(),
            )));
        }
    };

    write_string_attr(&lf_path, L400_BASE_PF_ATTR, &over_pf.path.to_string_lossy())?;
    if let Some(value) = select_value {
        write_string_attr(&lf_path, "user.l400.lf.select", value)?;
    }
    if let Some(value) = omit_value {
        write_string_attr(&lf_path, "user.l400.lf.omit", value)?;
    }
    write_storage_backend(&lf_path, over_pf.backend)?;
    catalog_object(&lf_path, "*FILE", Some("LF"), Some("Logical file"))?;

    let lf = LogicalFile {
        name: name.to_string(),
        backend: over_pf.backend,
        base_pf: over_pf.path.to_string_lossy().to_string(),
        storage,
    };

    for (primary_key, data) in over_pf.read_all()? {
        lf.insert_idx(&data, &primary_key)?;
    }

    Ok(lf)
}

impl LogicalFile {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let backend = read_storage_backend(path)?.unwrap_or(default_storage_backend());
        let pf_path_str = read_string_attr(path, L400_BASE_PF_ATTR)?
            .ok_or_else(|| DbError::InvalidType("LF object missing base_pf attribute".into()))?;
        let pf_path = Path::new(&pf_path_str);
        if !pf_path.exists() {
            return Err(DbError::NotFound);
        }

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let storage = match backend {
            StorageBackend::Sled => open_sled_lf(path, &name, pf_path)?,
            StorageBackend::BerkeleyDb => open_bdb_lf(path, false)?,
        };

        Ok(LogicalFile {
            name,
            backend,
            base_pf: pf_path_str,
            storage,
        })
    }

    pub fn insert_idx(&self, secondary_key: &[u8], primary_key: &[u8]) -> Result<(), DbError> {
        if !self.accepts_secondary_key(secondary_key)? {
            return Ok(());
        }
        match &self.storage {
            LogicalFileStorage::Sled { db, index } => {
                index.insert(secondary_key, primary_key)?;
                db.flush()?;
            }
            LogicalFileStorage::BerkeleyDb { db } => {
                db.put(secondary_key, primary_key)?;
            }
        }
        Ok(())
    }

    fn accepts_secondary_key(&self, secondary_key: &[u8]) -> Result<bool, DbError> {
        let value = String::from_utf8_lossy(secondary_key);
        let lf_path = Path::new(&self.base_pf)
            .parent()
            .map(|library| library.join(&self.name));
        let Some(lf_path) = lf_path else {
            return Ok(true);
        };
        if let Some(select) = read_string_attr(&lf_path, "user.l400.lf.select")? {
            return Ok(value == select);
        }
        if let Some(omit) = read_string_attr(&lf_path, "user.l400.lf.omit")? {
            return Ok(value != omit);
        }
        Ok(true)
    }

    pub fn setll(&self, secondary_key: &[u8]) -> Result<Vec<u8>, DbError> {
        match &self.storage {
            LogicalFileStorage::Sled { index, .. } => match index.get(secondary_key)? {
                Some(ivec) => Ok(ivec.to_vec()),
                None => Err(DbError::NotFound),
            },
            LogicalFileStorage::BerkeleyDb { db } => {
                db.get(secondary_key).map_err(|err| match err {
                    BdbError::NotFound => DbError::NotFound,
                    other => DbError::Bdb(other),
                })
            }
        }
    }

    pub fn read_all_idx(&self) -> Result<RecordSet, DbError> {
        match &self.storage {
            LogicalFileStorage::Sled { index, .. } => {
                let mut result = Vec::new();
                for item in index.iter() {
                    let (sk, pk) = item?;
                    result.push((sk.to_vec(), pk.to_vec()));
                }
                Ok(result)
            }
            LogicalFileStorage::BerkeleyDb { db } => Ok(db.read_all()?),
        }
    }

    pub fn delete_idx(&self, secondary_key: &[u8]) -> Result<(), DbError> {
        match &self.storage {
            LogicalFileStorage::Sled { db, index } => {
                index.remove(secondary_key)?;
                db.flush()?;
            }
            LogicalFileStorage::BerkeleyDb { db } => {
                db.delete(secondary_key).map_err(|err| match err {
                    BdbError::NotFound => DbError::NotFound,
                    other => DbError::Bdb(other),
                })?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableRef {
    library: Option<String>,
    file: String,
}

fn normalize_sql_statement(statement: &str) -> &str {
    statement.trim().trim_end_matches(';').trim()
}

fn find_keyword(statement: &str, keyword: &str) -> Option<usize> {
    let upper = statement.to_uppercase();
    let keyword = keyword.to_uppercase();
    let bytes = statement.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut depth = 0usize;
    let max = upper.len().saturating_sub(keyword.len());
    for index in 0..=max {
        let byte = bytes[index];
        match byte {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'(' if !in_single && !in_double => depth += 1,
            b')' if !in_single && !in_double => depth = depth.saturating_sub(1),
            _ => {}
        }
        if !in_single && !in_double && depth == 0 && upper[index..].starts_with(&keyword) {
            return Some(index);
        }
    }
    None
}

fn split_csv(part: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut depth = 0usize;
    for ch in part.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            '(' if !in_single && !in_double => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_single && !in_double => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if !in_single && !in_double && depth == 0 => {
                result.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }
    result
}

fn strip_sql_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .to_string()
}

fn parse_table_ref(token: &str) -> Result<TableRef, DbError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(DbError::InvalidQuery("table name is required".to_string()));
    }
    let (library, file) = if let Some((library, file)) = token.split_once('/') {
        (
            Some(library.trim().to_uppercase()),
            file.trim().to_uppercase(),
        )
    } else if let Some((library, file)) = token.split_once('.') {
        (
            Some(library.trim().to_uppercase()),
            file.trim().to_uppercase(),
        )
    } else {
        (None, token.to_uppercase())
    };
    if file.is_empty() {
        return Err(DbError::InvalidQuery("table name is required".to_string()));
    }
    Ok(TableRef { library, file })
}

fn resolve_library(library: Option<String>, default_library: Option<&str>) -> String {
    library.unwrap_or_else(|| {
        default_library
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_uppercase())
            .or_else(|| {
                std::env::var("L400_CURLIB")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| value.trim().to_uppercase())
            })
            .unwrap_or_else(|| "QGPL".to_string())
    })
}

fn resolve_table_path(
    table: &TableRef,
    default_library: Option<&str>,
) -> (String, std::path::PathBuf) {
    let library = resolve_library(table.library.clone(), default_library);
    let path = crate::object::resolve_l400_root()
        .join(&library)
        .join(&table.file);
    (library, path)
}

fn parse_where_filter(where_part: &str) -> Result<QueryFilter, DbError> {
    let (column, value) = where_part.split_once('=').ok_or_else(|| {
        DbError::InvalidQuery("WHERE only supports <column> = <value>".to_string())
    })?;
    Ok(QueryFilter {
        column: column.trim().to_uppercase(),
        value: strip_sql_value(value),
    })
}

fn parse_select_query(statement: &str) -> Result<SelectQuery, DbError> {
    let statement = normalize_sql_statement(statement);
    if statement.is_empty() {
        return Err(DbError::InvalidQuery("statement is empty".to_string()));
    }

    let upper = statement.to_uppercase();
    if !upper.starts_with("SELECT ") {
        return Err(DbError::InvalidQuery(
            "only SELECT statements are supported".to_string(),
        ));
    }

    let from_pos = upper.find(" FROM ").ok_or_else(|| {
        DbError::InvalidQuery("expected FROM clause: SELECT <cols> FROM <file>".to_string())
    })?;
    let select_part = statement[7..from_pos].trim();
    if select_part.is_empty() {
        return Err(DbError::InvalidQuery(
            "SELECT clause must include at least one column".to_string(),
        ));
    }

    let remainder = &statement[from_pos + 6..];
    let clause_positions = [
        find_keyword(remainder, " WHERE ").map(|pos| (pos, "WHERE")),
        find_keyword(remainder, " ORDER BY ").map(|pos| (pos, "ORDER BY")),
        find_keyword(remainder, " LIMIT ").map(|pos| (pos, "LIMIT")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let from_end = clause_positions
        .iter()
        .map(|(pos, _)| *pos)
        .min()
        .unwrap_or(remainder.len());
    let from_part = &remainder[..from_end];
    let where_part = clause_positions
        .iter()
        .find(|(_, keyword)| *keyword == "WHERE")
        .map(|(pos, _)| {
            let start = pos + " WHERE ".len();
            let end = clause_positions
                .iter()
                .filter_map(|(other_pos, _)| (*other_pos > *pos).then_some(*other_pos))
                .min()
                .unwrap_or(remainder.len());
            remainder[start..end].trim()
        });
    let order_by = clause_positions
        .iter()
        .find(|(_, keyword)| *keyword == "ORDER BY")
        .map(|(pos, _)| {
            let start = pos + " ORDER BY ".len();
            let end = clause_positions
                .iter()
                .filter_map(|(other_pos, _)| (*other_pos > *pos).then_some(*other_pos))
                .min()
                .unwrap_or(remainder.len());
            remainder[start..end].trim().to_uppercase()
        })
        .filter(|value| !value.is_empty());
    let limit = clause_positions
        .iter()
        .find(|(_, keyword)| *keyword == "LIMIT")
        .map(|(pos, _)| {
            let start = pos + " LIMIT ".len();
            let end = clause_positions
                .iter()
                .filter_map(|(other_pos, _)| (*other_pos > *pos).then_some(*other_pos))
                .min()
                .unwrap_or(remainder.len());
            remainder[start..end]
                .trim()
                .parse::<usize>()
                .map_err(|_| DbError::InvalidQuery("LIMIT must be a positive integer".to_string()))
        })
        .transpose()?;

    let table_token = from_part.trim();
    if table_token.is_empty() {
        return Err(DbError::InvalidQuery(
            "FROM clause must include a file name".to_string(),
        ));
    }

    let table = parse_table_ref(table_token)?;

    let columns = select_part
        .split(',')
        .map(|column| column.trim().to_uppercase())
        .filter(|column| !column.is_empty())
        .collect::<Vec<_>>();
    if columns.is_empty() {
        return Err(DbError::InvalidQuery(
            "SELECT clause must include at least one column".to_string(),
        ));
    }

    let filter = where_part.map(parse_where_filter).transpose()?;

    Ok(SelectQuery {
        library: table.library,
        file: table.file,
        columns,
        filter,
        order_by,
        limit,
    })
}

fn project_row(
    all_columns: &[String],
    row: &[String],
    requested_columns: &[String],
) -> Result<Vec<String>, DbError> {
    requested_columns
        .iter()
        .map(|requested| {
            let index = all_columns
                .iter()
                .position(|column| column == requested)
                .ok_or_else(|| DbError::InvalidQuery(format!("unknown column {requested}")))?;
            Ok(row[index].clone())
        })
        .collect()
}

pub fn run_select_query(
    statement: &str,
    default_library: Option<&str>,
) -> Result<QueryResult, DbError> {
    let query = parse_select_query(statement)?;
    let table = TableRef {
        library: query.library.clone(),
        file: query.file.clone(),
    };
    let (_library, path) = resolve_table_path(&table, default_library);
    let object = crate::object::describe_object(&path)?;
    let (all_columns, all_rows) = if object.attribute.as_deref() == Some("LF") {
        let file = LogicalFile::open(&path)?;
        let rows = file
            .read_all_idx()?
            .into_iter()
            .map(|(secondary_key, primary_key)| {
                vec![
                    String::from_utf8_lossy(&secondary_key).to_string(),
                    String::from_utf8_lossy(&primary_key).to_string(),
                ]
            })
            .collect::<Vec<_>>();
        (vec!["KEY".to_string(), "PRIMARY_KEY".to_string()], rows)
    } else {
        let file = PhysicalFile::open(&path)?;
        let rows = file
            .read_all()?
            .into_iter()
            .map(|(key, data)| {
                vec![
                    String::from_utf8_lossy(&key).to_string(),
                    String::from_utf8_lossy(&data).to_string(),
                ]
            })
            .collect::<Vec<_>>();
        (vec!["KEY".to_string(), "DATA".to_string()], rows)
    };

    let filtered_rows = if let Some(filter) = &query.filter {
        let filter_index = all_columns
            .iter()
            .position(|column| column == &filter.column)
            .ok_or_else(|| DbError::InvalidQuery(format!("unknown column {}", filter.column)))?;
        all_rows
            .into_iter()
            .filter(|row| {
                row.get(filter_index)
                    .is_some_and(|value| value == &filter.value)
            })
            .collect::<Vec<_>>()
    } else {
        all_rows
    };

    let mut filtered_rows = filtered_rows;
    if let Some(order_by) = &query.order_by {
        let order_column = order_by
            .split_whitespace()
            .next()
            .ok_or_else(|| DbError::InvalidQuery("ORDER BY requires a column".to_string()))?;
        let descending = order_by
            .split_whitespace()
            .nth(1)
            .is_some_and(|direction| direction.eq_ignore_ascii_case("DESC"));
        let order_index = all_columns
            .iter()
            .position(|column| column == order_column)
            .ok_or_else(|| DbError::InvalidQuery(format!("unknown column {order_column}")))?;
        filtered_rows.sort_by(|left, right| {
            let ordering = left[order_index].cmp(&right[order_index]);
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }
    if let Some(limit) = query.limit {
        filtered_rows.truncate(limit);
    }

    let requested_columns = if query.columns.len() == 1 && query.columns[0] == "*" {
        all_columns.clone()
    } else {
        query.columns.clone()
    };
    for column in &requested_columns {
        if !all_columns.iter().any(|candidate| candidate == column) {
            return Err(DbError::InvalidQuery(format!("unknown column {column}")));
        }
    }
    let projected_rows = filtered_rows
        .iter()
        .map(|row| project_row(&all_columns, row, &requested_columns))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(QueryResult {
        columns: requested_columns,
        rows: projected_rows,
    })
}

fn parse_insert_statement(statement: &str) -> Result<InsertStatement, DbError> {
    let rest = statement
        .get("INSERT INTO ".len()..)
        .ok_or_else(|| DbError::InvalidQuery("expected INSERT INTO".to_string()))?
        .trim();
    let values_pos = find_keyword(rest, " VALUES ")
        .ok_or_else(|| DbError::InvalidQuery("INSERT requires VALUES".to_string()))?;
    let target = rest[..values_pos].trim();
    let values_part = rest[values_pos + " VALUES ".len()..].trim();
    let (table_token, columns) = if let Some(open_pos) = target.find('(') {
        let close_pos = target.rfind(')').ok_or_else(|| {
            DbError::InvalidQuery("INSERT columns must close with ')'".to_string())
        })?;
        (
            target[..open_pos].trim(),
            Some(
                split_csv(&target[open_pos + 1..close_pos])
                    .into_iter()
                    .map(|column| column.to_uppercase())
                    .collect::<Vec<_>>(),
            ),
        )
    } else {
        (target, None)
    };
    let values = values_part
        .strip_prefix('(')
        .and_then(|part| part.strip_suffix(')'))
        .ok_or_else(|| {
            DbError::InvalidQuery("VALUES must be enclosed in parentheses".to_string())
        })?;
    Ok(InsertStatement {
        table: parse_table_ref(table_token)?,
        columns,
        values: split_csv(values)
            .into_iter()
            .map(|value| strip_sql_value(&value))
            .collect(),
    })
}

fn parse_assignments(part: &str) -> Result<Vec<(String, String)>, DbError> {
    split_csv(part)
        .into_iter()
        .map(|assignment| {
            let (column, value) = assignment.split_once('=').ok_or_else(|| {
                DbError::InvalidQuery("assignment must use <column> = <value>".to_string())
            })?;
            Ok((column.trim().to_uppercase(), strip_sql_value(value)))
        })
        .collect()
}

fn load_pf_for_sql(
    table: &TableRef,
    default_library: Option<&str>,
) -> Result<PhysicalFile, DbError> {
    let (_library, path) = resolve_table_path(table, default_library);
    let object = crate::object::describe_object(&path)?;
    if object.attribute.as_deref() == Some("LF") {
        return Err(DbError::InvalidQuery(
            "DML statements require a physical file".to_string(),
        ));
    }
    PhysicalFile::open(&path)
}

fn run_insert_statement(
    statement: &str,
    default_library: Option<&str>,
) -> Result<SqlStatementResult, DbError> {
    let insert = parse_insert_statement(statement)?;
    let pf = load_pf_for_sql(&insert.table, default_library)?;
    let columns = insert.columns.unwrap_or_else(|| {
        if insert.values.len() == 1 {
            vec!["DATA".to_string()]
        } else {
            vec!["KEY".to_string(), "DATA".to_string()]
        }
    });
    if columns.len() != insert.values.len() {
        return Err(DbError::InvalidQuery(
            "INSERT column count does not match VALUES count".to_string(),
        ));
    }
    let mut key = None;
    let mut data = None;
    let mut provided_values = std::collections::HashMap::new();
    for (column, value) in columns.into_iter().zip(insert.values) {
        provided_values.insert(column.clone(), value.clone());
        match column.as_str() {
            "KEY" | "RRN" => key = Some(value),
            "DATA" => data = Some(value),
            other if pf_schema_has_field(&pf.path, other)? => {}
            other => {
                return Err(DbError::InvalidQuery(format!(
                    "INSERT does not support column {other}"
                )));
            }
        }
    }
    let schema = read_pf_schema(&pf.path).unwrap_or_else(|_| PfSchema::minimal(pf.record_len));
    if schema.key_fields.len() > 1 {
        let mut parts = Vec::new();
        for field in &schema.key_fields {
            let Some(value) = provided_values.get(field) else {
                return Err(DbError::InvalidQuery(format!(
                    "missing composite key field {field}"
                )));
            };
            parts.push(value.clone());
        }
        key = Some(parts.join("|"));
    }
    let data = data.unwrap_or_default();
    if let Some(key) = key {
        pf.write_rcd(key.as_bytes(), data.as_bytes())?;
        Ok(SqlStatementResult::Message("1 row inserted".to_string()))
    } else {
        let rrn = pf.append_rcd(data.as_bytes())?;
        Ok(SqlStatementResult::Message(format!(
            "1 row inserted RRN({rrn})"
        )))
    }
}

fn pf_schema_has_field(path: &Path, column: &str) -> Result<bool, DbError> {
    let schema = read_pf_schema(path)?;
    Ok(schema.fields.iter().any(|field| field.name == column))
}

fn matching_keys(pf: &PhysicalFile, filter: Option<&QueryFilter>) -> Result<RecordSet, DbError> {
    let rows = pf.read_all()?;
    let Some(filter) = filter else {
        return Ok(rows);
    };
    match filter.column.as_str() {
        "KEY" | "RRN" => Ok(rows
            .into_iter()
            .filter(|(key, _)| String::from_utf8_lossy(key) == filter.value)
            .collect()),
        "DATA" => Ok(rows
            .into_iter()
            .filter(|(_, data)| String::from_utf8_lossy(data) == filter.value)
            .collect()),
        other => Err(DbError::InvalidQuery(format!("unknown column {other}"))),
    }
}

fn run_update_statement(
    statement: &str,
    default_library: Option<&str>,
) -> Result<SqlStatementResult, DbError> {
    let rest = statement
        .get("UPDATE ".len()..)
        .ok_or_else(|| DbError::InvalidQuery("expected UPDATE".to_string()))?
        .trim();
    let set_pos = find_keyword(rest, " SET ")
        .ok_or_else(|| DbError::InvalidQuery("UPDATE requires SET".to_string()))?;
    let where_pos = find_keyword(rest, " WHERE ")
        .ok_or_else(|| DbError::InvalidQuery("UPDATE requires WHERE".to_string()))?;
    if where_pos <= set_pos {
        return Err(DbError::InvalidQuery(
            "WHERE must appear after SET".to_string(),
        ));
    }
    let table = parse_table_ref(&rest[..set_pos])?;
    let assignments = parse_assignments(&rest[set_pos + " SET ".len()..where_pos])?;
    let filter = parse_where_filter(&rest[where_pos + " WHERE ".len()..])?;
    let pf = load_pf_for_sql(&table, default_library)?;
    let rows = matching_keys(&pf, Some(&filter))?;
    let mut count = 0usize;
    for (old_key, old_data) in rows {
        let mut new_key = old_key.clone();
        let mut new_data = old_data.clone();
        for (column, value) in &assignments {
            match column.as_str() {
                "KEY" | "RRN" => new_key = value.as_bytes().to_vec(),
                "DATA" => new_data = value.as_bytes().to_vec(),
                other => return Err(DbError::InvalidQuery(format!("unknown column {other}"))),
            }
        }
        if new_key != old_key {
            pf.delete_rcd(&old_key)?;
        }
        pf.write_rcd(&new_key, &new_data)?;
        count += 1;
    }
    Ok(SqlStatementResult::Message(format!(
        "{count} row(s) updated"
    )))
}

fn run_delete_statement(
    statement: &str,
    default_library: Option<&str>,
) -> Result<SqlStatementResult, DbError> {
    let rest = statement
        .get("DELETE FROM ".len()..)
        .ok_or_else(|| DbError::InvalidQuery("expected DELETE FROM".to_string()))?
        .trim();
    let where_pos = find_keyword(rest, " WHERE ")
        .ok_or_else(|| DbError::InvalidQuery("DELETE requires WHERE".to_string()))?;
    let table = parse_table_ref(&rest[..where_pos])?;
    let filter = parse_where_filter(&rest[where_pos + " WHERE ".len()..])?;
    let pf = load_pf_for_sql(&table, default_library)?;
    let rows = matching_keys(&pf, Some(&filter))?;
    let count = rows.len();
    for (key, _) in rows {
        pf.delete_rcd(&key)?;
    }
    Ok(SqlStatementResult::Message(format!(
        "{count} row(s) deleted"
    )))
}

fn run_create_table_statement(
    statement: &str,
    default_library: Option<&str>,
) -> Result<SqlStatementResult, DbError> {
    let rest = statement
        .get("CREATE TABLE ".len()..)
        .ok_or_else(|| DbError::InvalidQuery("expected CREATE TABLE".to_string()))?
        .trim();
    let open_pos = rest
        .find('(')
        .ok_or_else(|| DbError::InvalidQuery("CREATE TABLE requires columns".to_string()))?;
    let close_pos = rest.rfind(')').ok_or_else(|| {
        DbError::InvalidQuery("CREATE TABLE columns must close with ')'".to_string())
    })?;
    let table = parse_table_ref(&rest[..open_pos])?;
    let columns = split_csv(&rest[open_pos + 1..close_pos]);
    let fields = columns
        .into_iter()
        .filter_map(|column| {
            let mut parts = column.split_whitespace();
            let name = parts.next()?.trim().to_uppercase();
            let type_part = parts.next().unwrap_or("CHAR").trim().to_uppercase();
            let (type_, length) = if let Some(open) = type_part.find('(') {
                let close = type_part.find(')').unwrap_or(type_part.len());
                (
                    type_part[..open].to_string(),
                    type_part[open + 1..close].parse::<u32>().unwrap_or(0),
                )
            } else {
                let length = match type_part.as_str() {
                    "INT" | "INTEGER" => 10,
                    _ => 32,
                };
                (type_part, length)
            };
            Some(PfField {
                name,
                type_,
                length,
                text: None,
            })
        })
        .collect::<Vec<_>>();
    if fields.is_empty() {
        return Err(DbError::InvalidQuery(
            "CREATE TABLE requires at least one column".to_string(),
        ));
    }
    let record_len = fields.iter().map(|field| field.length).sum::<u32>().max(1);
    let library = resolve_library(table.library.clone(), default_library);
    let lib_path = crate::object::resolve_l400_root().join(&library);
    let pf = create_pf(&lib_path, &table.file, record_len as usize)?;
    let key_fields = fields
        .iter()
        .find(|field| field.name == "KEY")
        .map(|field| vec![field.name.clone()])
        .unwrap_or_else(|| vec!["KEY".to_string()]);
    write_pf_schema(
        &pf.path,
        &PfSchema {
            record_len,
            fields,
            key_fields,
        },
    )?;
    Ok(SqlStatementResult::Message(format!(
        "table {}/{} created",
        library, table.file
    )))
}

fn run_create_index_statement(
    statement: &str,
    default_library: Option<&str>,
) -> Result<SqlStatementResult, DbError> {
    let rest = statement
        .get("CREATE INDEX ".len()..)
        .ok_or_else(|| DbError::InvalidQuery("expected CREATE INDEX".to_string()))?
        .trim();
    let on_pos = find_keyword(rest, " ON ")
        .ok_or_else(|| DbError::InvalidQuery("CREATE INDEX requires ON".to_string()))?;
    let index_ref = parse_table_ref(&rest[..on_pos])?;
    let source_part = rest[on_pos + " ON ".len()..].trim();
    let table_token = source_part
        .split_once('(')
        .map(|(table, _)| table.trim())
        .unwrap_or(source_part);
    let source_ref = parse_table_ref(table_token)?;
    let library = resolve_library(
        index_ref.library.clone().or(source_ref.library.clone()),
        default_library,
    );
    let lib_path = crate::object::resolve_l400_root().join(&library);
    let (_src_library, source_path) = resolve_table_path(&source_ref, Some(&library));
    let pf = PhysicalFile::open(&source_path)?;
    create_lf(&lib_path, &index_ref.file, &pf)?;
    Ok(SqlStatementResult::Message(format!(
        "index {}/{} created",
        library, index_ref.file
    )))
}

pub fn run_sql_statement(
    statement: &str,
    default_library: Option<&str>,
) -> Result<SqlStatementResult, DbError> {
    let statement = normalize_sql_statement(statement);
    let upper = statement.to_uppercase();
    if upper.starts_with("SELECT ") {
        return run_select_query(statement, default_library).map(SqlStatementResult::Query);
    }
    if upper.starts_with("INSERT INTO ") {
        return run_insert_statement(statement, default_library);
    }
    if upper.starts_with("UPDATE ") {
        return run_update_statement(statement, default_library);
    }
    if upper.starts_with("DELETE FROM ") {
        return run_delete_statement(statement, default_library);
    }
    if upper.starts_with("CREATE TABLE ") {
        return run_create_table_statement(statement, default_library);
    }
    if upper.starts_with("CREATE INDEX ") {
        return run_create_index_statement(statement, default_library);
    }
    Err(DbError::InvalidQuery(
        "supported statements: SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, CREATE INDEX"
            .to_string(),
    ))
}

// ─── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::create_library;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn tmp_lib() -> TempDir {
        tempfile::tempdir().expect("No se pudo crear directorio temporal")
    }

    fn l400_library(root: &TempDir, name: &str) -> std::path::PathBuf {
        create_library(root.path(), name).expect("No se pudo crear biblioteca L400")
    }

    #[test]
    fn test_create_pf_and_round_trip() {
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let pf = create_pf(&lib_path, "CLIENTES", 100).expect("create_pf falló");

        let key = b"CLIENTE001";
        let valor = b"Juan Perez,Buenos Aires,2000";

        pf.write_rcd(key, valor).expect("write_rcd falló");
        let leido = pf.chain_rcd(key).expect("chain_rcd falló");

        assert_eq!(leido, valor, "Round-trip de datos fallido");
    }

    #[test]
    fn test_pf_not_found() {
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let pf = create_pf(&lib_path, "PEDIDOS", 50).expect("create_pf falló");
        let result = pf.chain_rcd(b"INEXISTENTE");
        assert!(matches!(result, Err(DbError::NotFound)));
    }

    #[test]
    fn test_pf_delete_rcd() {
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let pf = create_pf(&lib_path, "VENTAS", 50).expect("create_pf falló");
        pf.write_rcd(b"V001", b"100.00").expect("write_rcd falló");
        pf.delete_rcd(b"V001").expect("delete_rcd falló");
        assert!(matches!(pf.chain_rcd(b"V001"), Err(DbError::NotFound)));
    }

    #[test]
    fn test_create_lf_and_setll() {
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let pf = create_pf(&lib_path, "CLXPF", 100).expect("create_pf falló");

        pf.write_rcd(b"C001", b"Ana,CABA").unwrap();
        pf.write_rcd(b"C002", b"Luis,Rosario").unwrap();

        let lf = create_lf(&lib_path, "CLXLF", &pf).expect("create_lf falló");
        lf.insert_idx(b"Ana", b"C001").unwrap();
        lf.insert_idx(b"Luis", b"C002").unwrap();

        let pk = lf.setll(b"Ana").expect("setll falló");
        let registro = pf
            .chain_rcd(&pk)
            .expect("chain_rcd sobre primary key falló");
        assert_eq!(registro, b"Ana,CABA");
    }

    #[test]
    fn test_pf_schema_members_and_auto_lf_update() {
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let pf = create_pf(&lib_path, "CUSTOMERS", 64).expect("create_pf falló");
        let schema = PfSchema {
            record_len: 64,
            fields: vec![
                PfField {
                    name: "ID".to_string(),
                    type_: "CHAR".to_string(),
                    length: 10,
                    text: Some("Customer id".to_string()),
                },
                PfField {
                    name: "NAME".to_string(),
                    type_: "CHAR".to_string(),
                    length: 30,
                    text: None,
                },
            ],
            key_fields: vec!["ID".to_string()],
        };
        write_pf_schema(&pf.path, &schema).expect("write_pf_schema falló");
        assert_eq!(
            read_pf_schema(&pf.path).expect("read_pf_schema falló"),
            schema
        );

        add_pf_member(&pf.path, "JAN2026").expect("add_pf_member falló");
        assert!(
            list_pf_members(&pf.path)
                .expect("list_pf_members falló")
                .contains(&"JAN2026".to_string())
        );

        let lf = create_lf(&lib_path, "CUSTBYNAME", &pf).expect("create_lf falló");
        pf.write_rcd(b"C001", b"ALICE").expect("write_rcd falló");
        assert_eq!(lf.setll(b"ALICE").expect("LF auto update falló"), b"C001");

        let rrn = pf.append_rcd(b"BOB").expect("append_rcd falló");
        assert!(rrn > 0);
    }

    #[test]
    fn test_logical_file_open() {
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let lf_path = lib_path.join("EXTLF");

        {
            let pf = create_pf(&lib_path, "BASEPF", 100).unwrap();
            pf.write_rcd(b"K1", b"Data1").unwrap();

            let lf = create_lf(&lib_path, "EXTLF", &pf).unwrap();
            lf.insert_idx(b"S1", b"K1").unwrap();
        }

        let lf_opened = LogicalFile::open(&lf_path).expect("LogicalFile::open falló");
        let pk = lf_opened.setll(b"S1").unwrap();
        assert_eq!(pk, b"K1");
    }

    #[test]
    fn test_lf_read_all_idx_ordered() {
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let pf = create_pf(&lib_path, "ARTPF", 50).expect("create_pf falló");
        pf.write_rcd(b"P001", b"Teclado").unwrap();
        pf.write_rcd(b"P002", b"Monitor").unwrap();

        let lf = create_lf(&lib_path, "ARTLF", &pf).expect("create_lf falló");
        lf.insert_idx(b"Monitor", b"P002").unwrap();
        lf.insert_idx(b"Teclado", b"P001").unwrap();

        let all = lf.read_all_idx().expect("read_all_idx falló");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].0, b"Monitor");
        assert_eq!(all[1].0, b"Teclado");
    }

    #[test]
    fn test_odirect_buffer_alignment_util() {
        use crate::util::AlignedBuffer;
        let aligned = AlignedBuffer::new(1024);
        assert_eq!(
            aligned.as_ptr() as usize % 4096,
            0,
            "AlignedBuffer debe estar alineado a 4096 bytes"
        );
        assert_eq!(
            aligned.len() % 512,
            0,
            "El tamaño del buffer debe ser múltiplo de 512"
        );
    }

    #[test]
    fn test_odirect_buffer_size_is_multiple_of_512() {
        let bad_sizes = [1, 100, 511, 1023];
        for sz in bad_sizes {
            assert_ne!(
                sz % 512,
                0,
                "Tamaño {} no debe ser válido para O_DIRECT",
                sz
            );
        }
        let good_sizes = [512, 1024, 4096, 8192, 65536];
        for sz in good_sizes {
            assert_eq!(
                sz % 512,
                0,
                "Tamaño {} sí debe ser válido para O_DIRECT",
                sz
            );
        }
    }

    #[test]
    fn test_zfs_e2e_lf() {
        let pool_path = Path::new("/linux400pool");
        if !pool_path.exists() {
            println!("SKIPPING ZFS E2E TEST: /linux400pool not found or not mounted");
            return;
        }

        if std::fs::create_dir(pool_path.join(".l400_test_probe")).is_err() {
            println!("SKIPPING ZFS E2E TEST: No write permission on /linux400pool");
            return;
        }
        let _ = std::fs::remove_dir(pool_path.join(".l400_test_probe"));

        let test_dir = pool_path.join("test_fase3_debt");
        std::fs::create_dir_all(&test_dir).ok();
        let lib_path = create_library(&test_dir, "TESTLIB").expect("Fallo crear biblioteca L400");

        let pf_name = "E2EPF";
        let lf_name = "E2ELF";

        let pf = create_pf(&lib_path, pf_name, 100).expect("Fallo crear PF en ZFS");
        pf.write_rcd(b"KEY1", b"ZFS DATA").unwrap();

        let lf = create_lf(&lib_path, lf_name, &pf).expect("Fallo crear LF en ZFS");
        lf.insert_idx(b"IDX1", b"KEY1").unwrap();

        use crate::zfs::get_objtype;
        assert_eq!(get_objtype(&lib_path).unwrap(), "*LIB");
        assert_eq!(get_objtype(&lib_path.join(pf_name)).unwrap(), "*FILE");
        assert_eq!(get_objtype(&lib_path.join(lf_name)).unwrap(), "*FILE");

        std::fs::remove_dir_all(&test_dir).ok();
    }

    #[test]
    fn test_run_select_query_for_pf() {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env test lock poisoned");
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let pf = create_pf(&lib_path, "CLIENTES", 100).expect("create_pf falló");
        pf.write_rcd(b"C001", b"Ana").expect("write_rcd falló");
        pf.write_rcd(b"C002", b"Luis").expect("write_rcd falló");
        drop(pf);

        let original = std::env::var_os("L400_ROOT");
        unsafe {
            std::env::set_var("L400_ROOT", lib.path());
        }

        let result = run_select_query("SELECT * FROM CLIENTES WHERE KEY='C002'", Some("QGPL"))
            .expect("run_select_query falló");

        match original {
            Some(value) => unsafe {
                std::env::set_var("L400_ROOT", value);
            },
            None => unsafe {
                std::env::remove_var("L400_ROOT");
            },
        }

        assert_eq!(result.columns, vec!["KEY".to_string(), "DATA".to_string()]);
        assert_eq!(
            result.rows,
            vec![vec!["C002".to_string(), "Luis".to_string()]]
        );
    }

    #[test]
    fn test_run_select_query_rejects_unknown_columns() {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env test lock poisoned");
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");
        let pf = create_pf(&lib_path, "CLIENTES", 100).expect("create_pf falló");
        pf.write_rcd(b"C001", b"Ana").expect("write_rcd falló");
        drop(pf);

        let original = std::env::var_os("L400_ROOT");
        unsafe {
            std::env::set_var("L400_ROOT", lib.path());
        }

        let error = run_select_query("SELECT FOO FROM CLIENTES", Some("QGPL"))
            .expect_err("run_select_query debía fallar");

        match original {
            Some(value) => unsafe {
                std::env::set_var("L400_ROOT", value);
            },
            None => unsafe {
                std::env::remove_var("L400_ROOT");
            },
        }

        assert!(matches!(error, DbError::InvalidQuery(_)));
    }

    #[test]
    fn test_run_sql_statement_crud_and_order_limit() {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env test lock poisoned");
        let lib = tmp_lib();
        let lib_path = l400_library(&lib, "QGPL");

        let original = std::env::var_os("L400_ROOT");
        unsafe {
            std::env::set_var("L400_ROOT", lib.path());
        }

        run_sql_statement(
            "CREATE TABLE QGPL/CUSTOMERS (KEY CHAR(10), DATA CHAR(30))",
            Some("QGPL"),
        )
        .expect("CREATE TABLE falló");
        run_sql_statement(
            "INSERT INTO CUSTOMERS (KEY, DATA) VALUES ('C002', 'Luis')",
            Some("QGPL"),
        )
        .expect("INSERT C002 falló");
        run_sql_statement(
            "INSERT INTO CUSTOMERS (KEY, DATA) VALUES ('C001', 'Ana')",
            Some("QGPL"),
        )
        .expect("INSERT C001 falló");

        let pf = PhysicalFile::open(&lib_path.join("CUSTOMERS")).expect("open PF falló");
        let lf = create_lf(&lib_path, "CUSTBYDATA", &pf).expect("create LF falló");
        assert_eq!(
            lf.setll(b"Ana").expect("LF backfill falló"),
            b"C001".to_vec()
        );

        let result = match run_sql_statement(
            "SELECT KEY, DATA FROM CUSTOMERS ORDER BY DATA DESC LIMIT 1",
            Some("QGPL"),
        )
        .expect("SELECT ORDER BY LIMIT falló")
        {
            SqlStatementResult::Query(result) => result,
            other => panic!("resultado inesperado: {other:?}"),
        };
        assert_eq!(
            result.rows,
            vec![vec!["C002".to_string(), "Luis".to_string()]]
        );

        run_sql_statement(
            "UPDATE CUSTOMERS SET DATA='Carla' WHERE KEY='C001'",
            Some("QGPL"),
        )
        .expect("UPDATE falló");
        assert!(matches!(lf.setll(b"Ana"), Err(DbError::NotFound)));
        assert_eq!(
            lf.setll(b"Carla").expect("LF update falló"),
            b"C001".to_vec()
        );

        run_sql_statement("DELETE FROM CUSTOMERS WHERE KEY='C002'", Some("QGPL"))
            .expect("DELETE falló");
        let result =
            run_select_query("SELECT * FROM CUSTOMERS", Some("QGPL")).expect("SELECT final falló");

        match original {
            Some(value) => unsafe {
                std::env::set_var("L400_ROOT", value);
            },
            None => unsafe {
                std::env::remove_var("L400_ROOT");
            },
        }

        assert_eq!(
            result.rows,
            vec![vec!["C001".to_string(), "Carla".to_string()]]
        );
    }
}
