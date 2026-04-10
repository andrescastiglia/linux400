#![no_std]
#![no_main]

use aya_ebpf::{macros::{lsm, map}, maps::HashMap, programs::LsmContext};
use aya_log_ebpf::{info, warn};
use l400_ebpf_common::VALID_OBJ_TYPES;

#[map(name = "L400_STATS")]
static STATS: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);

#[inline(always)]
fn inc_stat(key: u32) {
    if let Some(val) = unsafe { STATS.get_ptr_mut(&key) } {
        unsafe { *val += 1 };
    } else {
        let _ = STATS.insert(&key, &1, 0);
    }
}
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

#[lsm(hook = "file_open", sleepable)]
pub fn file_open(ctx: LsmContext) -> i32 {
    match try_file_open(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_file_open(ctx: LsmContext) -> Result<i32, i32> {
    let file: *const c_void = unsafe { ctx.arg(0) };
    let file = file as *mut c_void;
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
    
    let mut is_valid = false;
    for (i, obj_type) in VALID_OBJ_TYPES.iter().enumerate() {
        if prefix == &obj_type.prefix {
            inc_stat((i as u32) + 2); // Contadores por tipo empieza en 2
            info!(&ctx, "Valido: Accediendo a objeto L400");
            is_valid = true;
            break;
        }
    }

    if is_valid {
        inc_stat(0); // 0 = Permitidos
    } else {
        warn!(&ctx, "Invalido: Etiqueta L400 irreconocible. Bloqueando acceso!");
        inc_stat(1); // 1 = Denegados
        return Err(EACCES);
    }

    Ok(0)
}

#[lsm(hook = "bprm_check_security")]
pub fn bprm_check_security(ctx: LsmContext) -> i32 {
    info!(&ctx, "Auditoria de bprm_check_security: Ejecucion detectada (Stub de Fase 1). Validacion postergada via file_open.");
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
