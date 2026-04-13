use crate::object::{catalog_object, ObjectError};
use crate::zfs::{get_objtype, validate_objtype, ZfsError};
use sled::{Db, Tree};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DtaqError {
    #[error("ZFS Metadata Error: {0}")]
    Zfs(#[from] ZfsError),
    #[error("Sled Error: {0}")]
    Sled(#[from] sled::Error),
    #[error("Invalid Object Type: {0}")]
    InvalidType(String),
    #[error("Object Error: {0}")]
    Object(#[from] ObjectError),
    #[error("Already Exists")]
    AlreadyExists,
    #[error("Timeout")]
    Timeout,
    #[error("Queue Empty")]
    Empty,
}

pub struct DataQueue {
    pub name: String,
    db: Db,
    tree: Tree,
}

pub fn crtdtaq(lib_path: &Path, name: &str) -> Result<DataQueue, DtaqError> {
    if get_objtype(lib_path)? != "*LIB" {
        return Err(DtaqError::InvalidType("target library must be a *LIB".to_string()));
    }

    let target = lib_path.join(name);

    if target.exists() {
        return Err(DtaqError::AlreadyExists);
    }

    if !validate_objtype("*DTAQ") {
        return Err(DtaqError::InvalidType("*DTAQ".to_string()));
    }

    let db = sled::open(&target)?;
    let tree = db.open_tree("DTAQ")?;

    catalog_object(&target, "*DTAQ", Some("DTAQ"), Some("Data queue"))?;

    Ok(DataQueue {
        name: name.to_string(),
        db,
        tree,
    })
}

impl DataQueue {
    pub fn open(path: &Path) -> Result<Self, DtaqError> {
        let db = sled::open(path)?;
        let tree = db.open_tree("DTAQ")?;

        Ok(DataQueue {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            db,
            tree,
        })
    }

    pub fn snddtaq(&self, buffer: &[u8]) -> Result<(), DtaqError> {
        let id = self.db.generate_id()?;
        self.tree.insert(id.to_be_bytes(), buffer)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn rcvdtaq(&self, wait_time: i32) -> Result<Vec<u8>, DtaqError> {
        if wait_time == 0 {
            return match self.tree.pop_min()? {
                Some((_k, v)) => Ok(v.to_vec()),
                None => Err(DtaqError::Empty),
            };
        }

        let end_time = std::time::Instant::now() + std::time::Duration::from_secs(wait_time as u64);
        loop {
            if let Some((_k, v)) = self.tree.pop_min()? {
                return Ok(v.to_vec());
            }
            if std::time::Instant::now() >= end_time {
                return Err(DtaqError::Timeout);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    pub fn read_all(&self) -> Result<Vec<(u64, Vec<u8>)>, DtaqError> {
        let mut result = Vec::new();
        for item in self.tree.iter() {
            let (key, value) = item?;
            let id = u64::from_be_bytes(
                key.as_ref()
                    .try_into()
                    .map_err(|_| DtaqError::InvalidType("invalid DTAQ key".to_string()))?,
            );
            result.push((id, value.to_vec()));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::create_library;

    #[test]
    fn dtaq_round_trip_and_read_all() {
        let root = tempfile::tempdir().expect("No se pudo crear directorio temporal");
        let lib = create_library(root.path(), "QUSRSYS").expect("create_library falló");
        let dtaq = crtdtaq(&lib, "QEZJOBLOG").expect("crtdtaq falló");

        dtaq.snddtaq(b"MSG1").expect("snddtaq falló");
        dtaq.snddtaq(b"MSG2").expect("snddtaq falló");

        let messages = dtaq.read_all().expect("read_all falló");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].1, b"MSG1");
        assert_eq!(messages[1].1, b"MSG2");

        let received = dtaq.rcvdtaq(0).expect("rcvdtaq falló");
        assert_eq!(received, b"MSG1");
        assert_eq!(dtaq.read_all().expect("read_all falló").len(), 1);
    }
}
