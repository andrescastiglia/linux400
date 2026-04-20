use crate::lam::{tag_for_objtype, tag_pointer, untag_pointer_mut};
use crate::object::{describe_object, ObjectError};
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpaceError {
    #[error("Object error: {0}")]
    Object(#[from] ObjectError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Mmap failed: {0}")]
    MmapFailed(String),
}

pub struct ObjectMapping {
    pub ptr: *mut u8,
    pub size: usize,
    pub objtype: String,
}

impl Drop for ObjectMapping {
    fn drop(&mut self) {
        let _ = self.unmap();
    }
}

impl ObjectMapping {
    /// Deshace el mapeo y sincroniza los cambios a disco
    pub fn unmap(&mut self) -> Result<(), SpaceError> {
        if self.ptr.is_null() {
            return Ok(());
        }

        let untagged_ptr = untag_pointer_mut(self.ptr);
        
        // msync
        let ret = unsafe { libc::msync(untagged_ptr as *mut libc::c_void, self.size, libc::MS_SYNC) };
        if ret != 0 {
            return Err(SpaceError::MmapFailed(format!(
                "msync failed with error: {}",
                std::io::Error::last_os_error()
            )));
        }

        // munmap
        let ret = unsafe { libc::munmap(untagged_ptr as *mut libc::c_void, self.size) };
        if ret != 0 {
            return Err(SpaceError::MmapFailed(format!(
                "munmap failed with error: {}",
                std::io::Error::last_os_error()
            )));
        }

        self.ptr = ptr::null_mut();
        self.size = 0;
        Ok(())
    }

    /// Retorna un slice mutable al contenido mapeado
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        if self.ptr.is_null() {
            return &mut [];
        }
        let untagged_ptr = untag_pointer_mut(self.ptr);
        unsafe { std::slice::from_raw_parts_mut(untagged_ptr, self.size) }
    }
}

/// Mapea un objeto en memoria y devuelve un puntero etiquetado con su tipo
pub fn map_object(path: &Path) -> Result<ObjectMapping, SpaceError> {
    let obj = describe_object(path)?;
    let file = OpenOptions::new().read(true).write(true).open(path)?;
    
    let metadata = file.metadata()?;
    let size = metadata.len() as usize;

    if size == 0 {
        return Ok(ObjectMapping {
            ptr: ptr::null_mut(),
            size: 0,
            objtype: obj.objtype,
        });
    }

    let ptr = unsafe {
        libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.as_raw_fd(),
            0,
        )
    };

    if ptr == libc::MAP_FAILED {
        return Err(SpaceError::MmapFailed(format!(
            "mmap failed: {}",
            std::io::Error::last_os_error()
        )));
    }

    // Tagging
    let tag = tag_for_objtype(&obj.objtype);
    let tagged_ptr = tag_pointer(ptr as *const u8, tag) as *mut u8;

    Ok(ObjectMapping {
        ptr: tagged_ptr,
        size,
        objtype: obj.objtype,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lam::{get_space_bits, objtype_from_tag};
    use crate::object::create_object_with_metadata;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_map_and_tag_object() {
        let dir = tempdir().unwrap();
        let lib_path = crate::object::create_library(dir.path(), "TESTLIB").unwrap();
        let obj_path = create_object_with_metadata(
            &lib_path,
            "TESTFILE",
            "*FILE",
            Some("PF"),
            Some("Test mapped file"),
        ).unwrap();

        // Escribir algo inicial
        {
            let mut file = std::fs::File::create(&obj_path).unwrap();
            file.write_all(b"Hello World!").unwrap();
        }

        // Mapear
        let mut mapping = map_object(&obj_path).unwrap();
        
        assert!(!mapping.ptr.is_null());
        assert_eq!(mapping.size, 12);
        
        // Verificar el Tag (bits 48-63)
        let tag = get_space_bits(mapping.ptr).unwrap();
        assert_eq!(objtype_from_tag(tag), "*FILE");

        // Leer del slice (de-tagging automático)
        let slice = mapping.as_mut_slice();
        assert_eq!(slice, b"Hello World!");

        // Modificar
        slice[0] = b'h';

        // msync and munmap
        mapping.unmap().unwrap();

        // Verificar que persistió
        let content = std::fs::read_to_string(&obj_path).unwrap();
        assert_eq!(content, "hello World!");
    }
}
