#![no_std]
#![no_main]

use aya_ebpf::{macros::lsm, programs::LsmContext};
use aya_log_ebpf::{info, warn};
use core::ffi::c_void;

#[repr(C)]
pub struct bpf_dynptr {
    val: [u64; 2],
}

extern "C" {
    // KFuncs kernel 6.11+
    pub fn bpf_dynptr_from_mem(data: *mut c_void, size: u32, flags: u64, ptr: *mut bpf_dynptr) -> i32;
    pub fn bpf_get_file_xattr(file: *mut c_void, name__str: *const u8, value_p: *mut bpf_dynptr) -> i32;
}

const EACCES: i32 = -13;

#[lsm(name = "file_open", sleepable)]
pub fn file_open(ctx: LsmContext) -> i32 {
    match try_file_open(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_file_open(ctx: LsmContext) -> Result<i32, i32> {
    let file: *mut c_void = unsafe { ctx.arg(0) };
    if file.is_null() {
        return Ok(0); // Ignore null cases safely
    }

    let attr_name = b"user.l400.objtype\0";
    let mut attr_value: [u8; 16] = [0; 16];
    let mut dynptr = bpf_dynptr { val: [0, 0] };

    // Construir dynptr en bpf
    let err = unsafe {
        bpf_dynptr_from_mem(
            attr_value.as_mut_ptr() as *mut c_void,
            attr_value.len() as u32,
            0,
            &mut dynptr as *mut bpf_dynptr,
        )
    };
    
    if err != 0 {
        return Ok(0); // dynptr fallback (si falla, permitir abrir)
    }

    // Extraer atributo a traves de la Kfunc
    let err = unsafe {
        bpf_get_file_xattr(
            file,
            attr_name.as_ptr(),
            &mut dynptr as *mut bpf_dynptr,
        )
    };

    if err < 0 {
        // No tiene el flag L400 (ej. un fichero nativo linux estandar), pasamos
        return Ok(0);
    }

    let prefix = &attr_value[0..4];
    
    if prefix == b"*PGM" {
        info!(&ctx, "Valido: Accediendo a objeto *PGM nativo");
    } else if prefix == b"*FIL" {
        info!(&ctx, "Valido: Accediendo a objeto *FILE nativo");
    } else if prefix == b"*USR" {
        info!(&ctx, "Valido: Accediendo a perfil de usuario");
    } else {
        warn!(&ctx, "Invalido: Etiqueta L400 irreconocible. Bloqueando acceso!");
        return Err(EACCES);
    }

    Ok(0)
}

#[lsm(name = "bprm_check_security")]
pub fn bprm_check_security(ctx: LsmContext) -> i32 {
    info!(&ctx, "Auditoria de bprm_check_security: Ejecucion detectada (Stub de Fase 1). Validacion postergada via file_open.");
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
