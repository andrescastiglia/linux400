use crate::bdb_native::{BdbError, BdbHandle};
use crate::object::{catalog_object, ObjectError};
use crate::storage::{
    default_storage_backend, read_storage_backend, read_string_attr, read_u32_attr,
    write_storage_backend, write_string_attr, write_u32_attr, StorageBackend, StorageError,
    L400_BASE_PF_ATTR, L400_RECORD_LEN_ATTR,
};
use crate::zfs::{get_objtype, validate_objtype, ZfsError};
use sled::{Db, Tree};
use std::path::Path;
use thiserror::Error;

pub type Record = Vec<u8>;
pub type RecordPair = (Record, Record);
pub type RecordSet = Vec<RecordPair>;

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

fn open_sled_pf(path: &Path) -> Result<PhysicalFileStorage, DbError> {
    let db = sled::open(path)?;
    let tree = db.open_tree("PF_MEMBER")?;
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

    Ok(PhysicalFile {
        name: name.to_string(),
        path: target.to_path_buf(),
        backend,
        record_len: record_len as u32,
        storage,
    })
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

    pub fn write_rcd(&self, key: &[u8], buffer: &[u8]) -> Result<(), DbError> {
        match &self.storage {
            PhysicalFileStorage::Sled { db, tree } => {
                tree.insert(key, buffer)?;
                db.flush()?;
            }
            PhysicalFileStorage::BerkeleyDb { db } => {
                db.put(key, buffer)?;
            }
        }
        Ok(())
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
        Ok(())
    }
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

fn open_sled_lf_from_db(name: &str, db: Db) -> Result<LogicalFileStorage, DbError> {
    let index_tree_name = format!("LF_IDX_{name}");
    let index = db.open_tree(index_tree_name.as_bytes())?;
    Ok(LogicalFileStorage::Sled { db, index })
}

fn open_sled_lf(path: &Path, name: &str, pf_path: &Path) -> Result<LogicalFileStorage, DbError> {
    let _ = path;
    let db = sled::open(pf_path)?;
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
            )))
        }
    };

    write_string_attr(&lf_path, L400_BASE_PF_ATTR, &over_pf.path.to_string_lossy())?;
    write_storage_backend(&lf_path, over_pf.backend)?;
    catalog_object(&lf_path, "*FILE", Some("LF"), Some("Logical file"))?;

    Ok(LogicalFile {
        name: name.to_string(),
        backend: over_pf.backend,
        base_pf: over_pf.path.to_string_lossy().to_string(),
        storage,
    })
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

// ─── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::create_library;
    use tempfile::TempDir;

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
}
