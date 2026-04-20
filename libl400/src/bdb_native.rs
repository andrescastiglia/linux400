use std::ffi::{c_int, c_uint, c_void, CString};
use std::path::Path;
use thiserror::Error;

const DB_BTREE: c_int = 1;
const DB_CREATE: c_uint = 0x00000001;
const DB_RDONLY: c_uint = 0x00000400;
const DB_NOTFOUND: c_int = -30988;
const DB_FIRST: c_uint = 7;
const DB_NEXT: c_uint = 16;
const DB_LAST: c_uint = 15;

#[repr(C)]
pub struct DB {
    _private: [u8; 0],
}

#[repr(C)]
pub struct DBC {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn l400_bdb_open(
        path: *const i8,
        db_type: c_int,
        open_flags: c_uint,
        out_db: *mut *mut DB,
    ) -> c_int;
    fn l400_bdb_close(db: *mut DB) -> c_int;
    fn l400_bdb_put(
        db: *mut DB,
        key: *const c_void,
        key_len: c_uint,
        data: *const c_void,
        data_len: c_uint,
    ) -> c_int;
    fn l400_bdb_get(
        db: *mut DB,
        key: *const c_void,
        key_len: c_uint,
        out_data: *mut *mut c_void,
        out_len: *mut c_uint,
    ) -> c_int;
    fn l400_bdb_del(db: *mut DB, key: *const c_void, key_len: c_uint) -> c_int;
    fn l400_bdb_cursor_open(db: *mut DB, out_cursor: *mut *mut DBC) -> c_int;
    fn l400_bdb_cursor_get(
        cursor: *mut DBC,
        out_key: *mut *mut c_void,
        out_key_len: *mut c_uint,
        out_data: *mut *mut c_void,
        out_data_len: *mut c_uint,
        flags: c_uint,
    ) -> c_int;
    fn l400_bdb_cursor_close(cursor: *mut DBC) -> c_int;
    fn l400_bdb_free(ptr: *mut c_void);
}

#[derive(Error, Debug)]
pub enum BdbError {
    #[error("invalid path for Berkeley DB")]
    InvalidPath,
    #[error("Berkeley DB error code {0}")]
    DbCode(i32),
    #[error("record not found")]
    NotFound,
}

fn map_db_error(code: i32) -> BdbError {
    if code == DB_NOTFOUND {
        BdbError::NotFound
    } else {
        BdbError::DbCode(code)
    }
}

pub struct BdbHandle {
    raw: *mut DB,
}

impl BdbHandle {
    pub fn open(path: &Path, create: bool) -> Result<Self, BdbError> {
        let path =
            CString::new(path.to_string_lossy().as_bytes()).map_err(|_| BdbError::InvalidPath)?;
        let mut db = std::ptr::null_mut();
        let flags = if create { DB_CREATE } else { DB_RDONLY };
        let ret = unsafe { l400_bdb_open(path.as_ptr(), DB_BTREE, flags, &mut db) };
        if ret != 0 {
            if !create {
                let mut db_rw = std::ptr::null_mut();
                let ret_rw = unsafe { l400_bdb_open(path.as_ptr(), DB_BTREE, 0, &mut db_rw) };
                if ret_rw == 0 {
                    return Ok(Self { raw: db_rw });
                }
                return Err(map_db_error(ret_rw));
            }
            return Err(map_db_error(ret));
        }
        Ok(Self { raw: db })
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), BdbError> {
        let ret = unsafe {
            l400_bdb_put(
                self.raw,
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
                value.as_ptr() as *const c_void,
                value.len() as c_uint,
            )
        };
        if ret != 0 {
            return Err(map_db_error(ret));
        }
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Vec<u8>, BdbError> {
        let mut data = std::ptr::null_mut();
        let mut len = 0;
        let ret = unsafe {
            l400_bdb_get(
                self.raw,
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
                &mut data,
                &mut len,
            )
        };
        if ret != 0 {
            return Err(map_db_error(ret));
        }
        let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, len as usize).to_vec() };
        unsafe { l400_bdb_free(data) };
        Ok(bytes)
    }

    pub fn delete(&self, key: &[u8]) -> Result<(), BdbError> {
        let ret =
            unsafe { l400_bdb_del(self.raw, key.as_ptr() as *const c_void, key.len() as c_uint) };
        if ret != 0 {
            return Err(map_db_error(ret));
        }
        Ok(())
    }

    pub fn read_all(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>, BdbError> {
        let mut cursor = std::ptr::null_mut();
        let ret = unsafe { l400_bdb_cursor_open(self.raw, &mut cursor) };
        if ret != 0 {
            return Err(map_db_error(ret));
        }

        let mut rows = Vec::new();
        loop {
            let mut key = std::ptr::null_mut();
            let mut key_len = 0;
            let mut data = std::ptr::null_mut();
            let mut data_len = 0;
            let ret = unsafe {
                l400_bdb_cursor_get(
                    cursor,
                    &mut key,
                    &mut key_len,
                    &mut data,
                    &mut data_len,
                    if rows.is_empty() { DB_FIRST } else { DB_NEXT },
                )
            };
            if ret != 0 {
                if ret == DB_NOTFOUND {
                    break;
                }
                unsafe { l400_bdb_cursor_close(cursor) };
                return Err(map_db_error(ret));
            }
            let key_vec =
                unsafe { std::slice::from_raw_parts(key as *const u8, key_len as usize).to_vec() };
            let data_vec = unsafe {
                std::slice::from_raw_parts(data as *const u8, data_len as usize).to_vec()
            };
            unsafe {
                l400_bdb_free(key);
                l400_bdb_free(data);
            }
            rows.push((key_vec, data_vec));
        }

        unsafe { l400_bdb_cursor_close(cursor) };
        Ok(rows)
    }

    pub fn last_key(&self) -> Result<Option<Vec<u8>>, BdbError> {
        let mut cursor = std::ptr::null_mut();
        let ret = unsafe { l400_bdb_cursor_open(self.raw, &mut cursor) };
        if ret != 0 {
            return Err(map_db_error(ret));
        }

        let mut key = std::ptr::null_mut();
        let mut key_len = 0;
        let mut data = std::ptr::null_mut();
        let mut data_len = 0;
        let ret = unsafe {
            l400_bdb_cursor_get(
                cursor,
                &mut key,
                &mut key_len,
                &mut data,
                &mut data_len,
                DB_LAST,
            )
        };
        if ret == DB_NOTFOUND {
            unsafe { l400_bdb_cursor_close(cursor) };
            return Ok(None);
        }
        if ret != 0 {
            unsafe { l400_bdb_cursor_close(cursor) };
            return Err(map_db_error(ret));
        }
        let key_vec =
            unsafe { std::slice::from_raw_parts(key as *const u8, key_len as usize).to_vec() };
        unsafe {
            l400_bdb_free(key);
            l400_bdb_free(data);
            l400_bdb_cursor_close(cursor);
        }
        Ok(Some(key_vec))
    }
}

impl Drop for BdbHandle {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                let _ = l400_bdb_close(self.raw);
            }
        }
    }
}
