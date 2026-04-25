use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU32, Ordering};

static LAST_CPF_CODE: AtomicU32 = AtomicU32::new(0);

pub fn set_last_cpf(code: &str) {
    let normalized = crate::status::normalize_cpf(code);
    let numeric = normalized
        .trim()
        .strip_prefix("CPF")
        .unwrap_or(normalized.trim())
        .parse::<u32>()
        .unwrap_or(0);
    LAST_CPF_CODE.store(numeric, Ordering::SeqCst);
}

pub fn clear_last_cpf() {
    LAST_CPF_CODE.store(0, Ordering::SeqCst);
}

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

#[no_mangle]
pub extern "C" fn l400_last_cpf_code() -> u32 {
    LAST_CPF_CODE.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn l400_clear_status() {
    clear_last_cpf();
}

#[no_mangle]
/// Sets the last CPF status code from a C string.
///
/// # Safety
/// `code` must be null or a valid NUL-terminated C string for the duration of the call.
pub unsafe extern "C" fn l400_set_status_cpf(code: *const c_char) {
    if code.is_null() {
        clear_last_cpf();
        return;
    }
    let code = unsafe { CStr::from_ptr(code) }.to_string_lossy();
    set_last_cpf(&code);
}
