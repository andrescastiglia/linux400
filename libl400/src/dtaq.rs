use crate::zfs::{set_objtype, validate_objtype, ZfsError};
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
    let target = lib_path.join(name);

    if target.exists() {
        return Err(DtaqError::AlreadyExists);
    }

    if !validate_objtype("*DTAQ") {
        return Err(DtaqError::InvalidType("*DTAQ".to_string()));
    }

    let db = sled::open(&target)?;
    let tree = db.open_tree("DTAQ")?;

    set_objtype(&target, "*DTAQ")?;

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
}
