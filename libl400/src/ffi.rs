use std::ffi::CStr;
use std::os::raw::c_char;

/// Envía un mensaje del programa (SNDPGMMSG) a la salida estándar/log del sistema.
///
/// # Safety
/// `msg` debe ser un puntero válido a una cadena C terminada en NUL durante toda la llamada.
#[no_mangle]
pub unsafe extern "C" fn l400_sndpgmmsg(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(msg) };
    if let Ok(s) = c_str.to_str() {
        println!("[L400 SNDPGMMSG] {}", s);
    }
}
