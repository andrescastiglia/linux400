use std::alloc::{alloc, dealloc, Layout};
use std::ops::{Deref, DerefMut};
use std::slice;

/// Un buffer alineado para operaciones de I/O directo (O_DIRECT).
/// Garantiza alineación a 4096 bytes para máxima compatibilidad con FS Linux y ZFS.
pub struct AlignedBuffer {
    ptr: *mut u8,
    layout: Layout,
    len: usize,
}

impl AlignedBuffer {
    pub fn new(len: usize) -> Self {
        // Redondear len al múltiplo de 4096 más cercano si es necesario para O_DIRECT
        let aligned_len = (len + 4095) & !4095;
        let layout = Layout::from_size_align(aligned_len, 4096)
            .expect("Falla al crear Layout para AlignedBuffer");

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                panic!("Memory allocation failed for AlignedBuffer");
            }
            // Inicializar con ceros para seguridad
            std::ptr::write_bytes(ptr, 0, aligned_len);

            Self {
                ptr,
                layout,
                len: aligned_len,
            }
        }
    }

    pub fn from_slice(src: &[u8]) -> Self {
        let mut buf = Self::new(src.len());
        buf[..src.len()].copy_from_slice(src);
        buf
    }
}

impl Deref for AlignedBuffer {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl DerefMut for AlignedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr, self.layout);
        }
    }
}

unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment() {
        let buf = AlignedBuffer::new(100);
        assert_eq!(buf.len(), 4096);
        assert_eq!(buf.as_ptr() as usize % 4096, 0);
    }

    #[test]
    fn test_from_slice() {
        let data = b"hello world";
        let buf = AlignedBuffer::from_slice(data);
        assert_eq!(&buf[..data.len()], data);
    }
}
