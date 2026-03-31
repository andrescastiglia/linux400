use std::ffi::CStr;

pub mod db;

// Wrapper estandar compatible con ABI general C que LLVM vinculará y ejecutará para emular CL

#[no_mangle]
pub extern "C" fn l400_sndpgmmsg(msg: *const libc::c_char, _tousr: *const libc::c_char) -> i32 {
    unsafe {
        if !msg.is_null() {
            let rs_msg = CStr::from_ptr(msg);
            println!("[SNDPGMMSG]: {}", rs_msg.to_string_lossy());
            return 0; // Ok
        }
    }
    -1 // Error
}

#[no_mangle]
pub extern "C" fn l400_dltobj(_obj: *const libc::c_char) -> i32 {
    println!("[DLTOBJ] Operación delegada con puntero protegido.");
    0
}
