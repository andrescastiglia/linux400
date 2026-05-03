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

#[unsafe(no_mangle)]
/// # Safety
/// `code` must be either null or a valid pointer to a null-terminated C string.
pub unsafe extern "C" fn l400_set_cpf(code: *const c_char) {
    if code.is_null() {
        clear_last_cpf();
        return;
    }
    let code_str = unsafe { CStr::from_ptr(code) };
    let code = code_str.to_string_lossy();
    set_last_cpf(&code);
}

pub fn clear_last_cpf() {
    LAST_CPF_CODE.store(0, Ordering::SeqCst);
}

/// Envía un mensaje del programa (SNDPGMMSG) a la salida estándar/log del sistema.
///
/// # Safety
/// `msg` debe ser un puntero válido a una cadena C terminada en NUL durante toda la llamada.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn l400_sndpgmmsg(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(msg) };
    if let Ok(s) = c_str.to_str() {
        println!("[L400 SNDPGMMSG] {}", s);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_last_cpf_code() -> u32 {
    LAST_CPF_CODE.load(Ordering::SeqCst)
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_clear_status() {
    clear_last_cpf();
}

#[unsafe(no_mangle)]
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
