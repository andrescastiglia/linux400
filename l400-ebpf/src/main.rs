#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_get_current_uid_gid},
    macros::{lsm, map},
    maps::HashMap,
    programs::LsmContext,
};
use aya_log_ebpf::{info, warn};
use core::ffi::c_void;
use l400_ebpf_common::{
    L400_POLICY_VERSION, STAT_DENIED_INVALID_TAG, STAT_EXEC_ALLOWED_NATIVE,
    STAT_EXEC_ALLOWED_OWNER, STAT_EXEC_ALLOWED_PGM, STAT_EXEC_ALLOWED_USER_AUTH,
    STAT_EXEC_CHECK_ALLOWED, STAT_EXEC_CHECK_DENIED, STAT_EXEC_DECISION_MISSING,
    STAT_EXEC_DENIED_EXCLUDE, STAT_EXEC_DENIED_INVALID_FORMAT, STAT_EXEC_DENIED_WRONG_TYPE,
    STAT_OBJTYPE_BASE, STAT_OPEN_ALLOWED, STAT_OPEN_ALLOWED_OWNER, STAT_OPEN_ALLOWED_USER_AUTH,
    STAT_OPEN_DENIED_EXCLUDE, VALID_OBJ_TYPES,
};

#[map(name = "L400_STATS")]
static STATS: HashMap<u32, u64> = HashMap::with_max_entries(64, 0);

#[map(name = "L400_EXEC_GUARD")]
static EXEC_GUARD: HashMap<u32, u32> = HashMap::with_max_entries(1024, 0);

#[inline(always)]
fn inc_stat(key: u32) {
    if let Some(val) = STATS.get_ptr_mut(&key) {
        unsafe { *val += 1 };
    } else {
        let _ = STATS.insert(&key, &1, 0);
    }
}

#[repr(C)]
pub struct bpf_dynptr {
    val: [u64; 2],
}

unsafe extern "C" {
    pub fn bpf_dynptr_from_mem(
        data: *mut c_void,
        size: u32,
        flags: u64,
        ptr: *mut bpf_dynptr,
    ) -> i32;
    pub fn bpf_get_file_xattr(
        file: *mut c_void,
        name__str: *const u8,
        value_p: *mut bpf_dynptr,
    ) -> i32;
}

const EACCES: i32 = -13;
const EXEC_ALLOW_NATIVE: u32 = 1;
const EXEC_ALLOW_PGM: u32 = 2;
const EXEC_DENY_INVALID_TAG: u32 = 3;
const EXEC_DENY_WRONG_TYPE: u32 = 4;
const EXEC_DENY_INVALID_FORMAT: u32 = 5;
const EXEC_DENY_EXCLUDE: u32 = 6;

enum ObjTypeLookup {
    Untagged,
    Known([u8; 4], usize),
    Invalid,
}

fn lookup_file_objtype(file: *mut c_void) -> ObjTypeLookup {
    let attr_name = b"user.l400.objtype\0";
    let mut attr_value: [u8; 16] = [0; 16];
    let mut dynptr = bpf_dynptr { val: [0, 0] };

    let err = unsafe {
        bpf_dynptr_from_mem(
            attr_value.as_mut_ptr() as *mut c_void,
            attr_value.len() as u32,
            0,
            &mut dynptr as *mut bpf_dynptr,
        )
    };
    if err != 0 {
        return ObjTypeLookup::Untagged;
    }

    let err =
        unsafe { bpf_get_file_xattr(file, attr_name.as_ptr(), &mut dynptr as *mut bpf_dynptr) };
    if err < 0 {
        return ObjTypeLookup::Untagged;
    }

    let prefix = [attr_value[0], attr_value[1], attr_value[2], attr_value[3]];
    for (i, obj_type) in VALID_OBJ_TYPES.iter().enumerate() {
        if prefix == obj_type.prefix {
            return ObjTypeLookup::Known(prefix, i);
        }
    }

    ObjTypeLookup::Invalid
}

fn lookup_file_objattr(file: *mut c_void) -> bool {
    let attr_name = b"user.l400.objattr\0";
    let mut attr_value: [u8; 16] = [0; 16];
    let mut dynptr = bpf_dynptr { val: [0, 0] };

    let err = unsafe {
        bpf_dynptr_from_mem(
            attr_value.as_mut_ptr() as *mut c_void,
            attr_value.len() as u32,
            0,
            &mut dynptr as *mut bpf_dynptr,
        )
    };
    if err != 0 {
        return false;
    }

    let err =
        unsafe { bpf_get_file_xattr(file, attr_name.as_ptr(), &mut dynptr as *mut bpf_dynptr) };
    if err < 0 {
        return false;
    }

    if err == 1 && attr_value[0] == b'C' {
        return true;
    }
    if err == 2 && attr_value[0] == b'C' && attr_value[1] == b'L' {
        return true;
    }

    // Also handle null-terminated strings just in case
    if attr_value[0] == b'C' && attr_value[1] == 0 {
        return true;
    }
    if attr_value[0] == b'C' && attr_value[1] == b'L' && attr_value[2] == 0 {
        return true;
    }

    false
}

fn lookup_file_public_auth_exclude(file: *mut c_void) -> bool {
    let attr_name = b"user.l400.auth\0";
    let mut attr_value: [u8; 128] = [0; 128];
    let mut dynptr = bpf_dynptr { val: [0, 0] };

    let err = unsafe {
        bpf_dynptr_from_mem(
            attr_value.as_mut_ptr() as *mut c_void,
            attr_value.len() as u32,
            0,
            &mut dynptr as *mut bpf_dynptr,
        )
    };
    if err != 0 {
        return false;
    }

    let err =
        unsafe { bpf_get_file_xattr(file, attr_name.as_ptr(), &mut dynptr as *mut bpf_dynptr) };
    if err < 0 {
        return false;
    }

    let len = err as usize;
    if len > 128 {
        return false;
    }

    let target = b"*PUBLIC:*EXCLUDE";
    let target2 = b"*PUBLIC:EXCLUDE";

    // Substring search
    for i in 0..len {
        if i + target.len() <= len && &attr_value[i..i + target.len()] == target {
            return true;
        }
        if i + target2.len() <= len && &attr_value[i..i + target2.len()] == target2 {
            return true;
        }
    }

    false
}

fn lookup_file_owner_uid_matches(file: *mut c_void, uid: u32) -> bool {
    let attr_name = b"user.l400.owner_uid\0";
    let mut attr_value: [u8; 16] = [0; 16];
    let mut dynptr = bpf_dynptr { val: [0, 0] };

    let err = unsafe {
        bpf_dynptr_from_mem(
            attr_value.as_mut_ptr() as *mut c_void,
            attr_value.len() as u32,
            0,
            &mut dynptr as *mut bpf_dynptr,
        )
    };
    if err != 0 {
        return false;
    }

    let err =
        unsafe { bpf_get_file_xattr(file, attr_name.as_ptr(), &mut dynptr as *mut bpf_dynptr) };
    if err <= 0 || err > 16 {
        return false;
    }

    let mut parsed: u32 = 0;
    let mut i = 0usize;
    while i < err as usize {
        let ch = attr_value[i];
        if !ch.is_ascii_digit() {
            return false;
        }
        parsed = parsed.saturating_mul(10).saturating_add((ch - b'0') as u32);
        i += 1;
    }

    parsed == uid
}

fn lookup_file_uid_auth_allows(file: *mut c_void, uid: u32) -> bool {
    let attr_name = b"user.l400.auth\0";
    let mut attr_value: [u8; 128] = [0; 128];
    let mut dynptr = bpf_dynptr { val: [0, 0] };

    let err = unsafe {
        bpf_dynptr_from_mem(
            attr_value.as_mut_ptr() as *mut c_void,
            attr_value.len() as u32,
            0,
            &mut dynptr as *mut bpf_dynptr,
        )
    };
    if err != 0 {
        return false;
    }

    let err =
        unsafe { bpf_get_file_xattr(file, attr_name.as_ptr(), &mut dynptr as *mut bpf_dynptr) };
    if err <= 0 || err > 128 {
        return false;
    }

    let mut uid_buf: [u8; 10] = [0; 10];
    let mut n = uid;
    let mut digits = 0usize;
    if n == 0 {
        uid_buf[0] = b'0';
        digits = 1;
    } else {
        let mut rev: [u8; 10] = [0; 10];
        while n > 0 && digits < 10 {
            rev[digits] = b'0' + (n % 10) as u8;
            n /= 10;
            digits += 1;
        }
        let mut j = 0usize;
        while j < digits {
            uid_buf[j] = rev[digits - j - 1];
            j += 1;
        }
    }

    let len = err as usize;
    let mut i = 0usize;
    while i < len {
        if i + 4 + digits + 5 <= len && &attr_value[i..i + 4] == b"UID:" {
            let uid_start = i + 4;
            let auth_start = uid_start + digits;
            if attr_value[uid_start..auth_start] == uid_buf[..digits]
                && attr_value[auth_start] == b':'
            {
                let remaining = len - auth_start - 1;
                let auth = &attr_value[auth_start + 1..len];
                if remaining >= 4 && &auth[..4] == b"*USE" {
                    return true;
                }
                if remaining >= 4 && &auth[..4] == b"*ALL" {
                    return true;
                }
                if remaining >= 3 && &auth[..3] == b"USE" {
                    return true;
                }
                if remaining >= 3 && &auth[..3] == b"ALL" {
                    return true;
                }
            }
        }
        i += 1;
    }

    false
}

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
        return Ok(0);
    }

    match lookup_file_objtype(file) {
        ObjTypeLookup::Untagged => Ok(0),
        ObjTypeLookup::Known(_, index) => {
            inc_stat(STAT_OBJTYPE_BASE + index as u32);
            if lookup_file_public_auth_exclude(file) {
                let uid = (bpf_get_current_uid_gid() & 0xffff_ffff) as u32;
                if lookup_file_owner_uid_matches(file, uid) {
                    inc_stat(STAT_OPEN_ALLOWED_OWNER);
                } else if lookup_file_uid_auth_allows(file, uid) {
                    inc_stat(STAT_OPEN_ALLOWED_USER_AUTH);
                } else {
                    inc_stat(STAT_OPEN_DENIED_EXCLUDE);
                    warn!(
                        &ctx,
                        "Policy {}: file_open denegado por *PUBLIC:*EXCLUDE", L400_POLICY_VERSION
                    );
                    return Err(EACCES);
                }
            }
            inc_stat(STAT_OPEN_ALLOWED);
            info!(
                &ctx,
                "Policy {}: acceso permitido a objeto L400", L400_POLICY_VERSION
            );
            Ok(0)
        }
        ObjTypeLookup::Invalid => {
            inc_stat(STAT_DENIED_INVALID_TAG);
            warn!(
                &ctx,
                "Policy {}: etiqueta L400 irreconocible, acceso denegado", L400_POLICY_VERSION
            );
            Err(EACCES)
        }
    }
}

#[lsm(hook = "bprm_creds_from_file", sleepable)]
pub fn bprm_creds_from_file(ctx: LsmContext) -> i32 {
    match try_bprm_creds_from_file(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_bprm_creds_from_file(ctx: LsmContext) -> Result<i32, i32> {
    let file: *const c_void = unsafe { ctx.arg(1) };
    let file = file as *mut c_void;
    if file.is_null() {
        return Ok(0);
    }

    let pid = bpf_get_current_pid_tgid() as u32;
    let decision = match lookup_file_objtype(file) {
        ObjTypeLookup::Untagged => {
            inc_stat(STAT_EXEC_ALLOWED_NATIVE);
            info!(
                &ctx,
                "Policy {}: ejecución permitida para binario nativo no catalogado",
                L400_POLICY_VERSION
            );
            EXEC_ALLOW_NATIVE
        }
        ObjTypeLookup::Known(prefix, index) => {
            inc_stat(STAT_OBJTYPE_BASE + index as u32);
            if prefix == *b"*PGM" {
                if lookup_file_public_auth_exclude(file) {
                    let uid = (bpf_get_current_uid_gid() & 0xffff_ffff) as u32;
                    if lookup_file_owner_uid_matches(file, uid) {
                        inc_stat(STAT_EXEC_ALLOWED_OWNER);
                        info!(
                            &ctx,
                            "Policy {}: ejecución permitida para owner UID", L400_POLICY_VERSION
                        );
                        EXEC_ALLOW_PGM
                    } else if lookup_file_uid_auth_allows(file, uid) {
                        inc_stat(STAT_EXEC_ALLOWED_USER_AUTH);
                        info!(
                            &ctx,
                            "Policy {}: ejecución permitida por autoridad UID", L400_POLICY_VERSION
                        );
                        EXEC_ALLOW_PGM
                    } else {
                        inc_stat(STAT_EXEC_DENIED_EXCLUDE);
                        warn!(
                            &ctx,
                            "Policy {}: ejecución denegada por *PUBLIC:*EXCLUDE",
                            L400_POLICY_VERSION
                        );
                        EXEC_DENY_EXCLUDE
                    }
                } else if lookup_file_objattr(file) {
                    inc_stat(STAT_EXEC_ALLOWED_PGM);
                    info!(
                        &ctx,
                        "Policy {}: ejecución permitida para objeto *PGM nativo",
                        L400_POLICY_VERSION
                    );
                    EXEC_ALLOW_PGM
                } else {
                    inc_stat(STAT_EXEC_DENIED_INVALID_FORMAT);
                    warn!(
                        &ctx,
                        "Policy {}: ejecución denegada, el *PGM no tiene firma de toolchain válida",
                        L400_POLICY_VERSION
                    );
                    EXEC_DENY_INVALID_FORMAT
                }
            } else {
                inc_stat(STAT_EXEC_DENIED_WRONG_TYPE);
                warn!(
                    &ctx,
                    "Policy {}: ejecución denegada, sólo *PGM puede ejecutar", L400_POLICY_VERSION
                );
                EXEC_DENY_WRONG_TYPE
            }
        }
        ObjTypeLookup::Invalid => {
            inc_stat(STAT_DENIED_INVALID_TAG);
            warn!(
                &ctx,
                "Policy {}: ejecución denegada por etiqueta L400 inválida", L400_POLICY_VERSION
            );
            EXEC_DENY_INVALID_TAG
        }
    };

    let _ = EXEC_GUARD.insert(&pid, &decision, 0);

    match decision {
        EXEC_ALLOW_NATIVE | EXEC_ALLOW_PGM => Ok(0),
        EXEC_DENY_INVALID_TAG
        | EXEC_DENY_WRONG_TYPE
        | EXEC_DENY_INVALID_FORMAT
        | EXEC_DENY_EXCLUDE => Err(EACCES),
        _ => Ok(0),
    }
}

#[lsm(hook = "bprm_check_security")]
pub fn bprm_check_security(ctx: LsmContext) -> i32 {
    let pid = bpf_get_current_pid_tgid() as u32;
    let decision = unsafe { EXEC_GUARD.get(&pid).copied() };

    if let Some(decision) = decision {
        let _ = EXEC_GUARD.remove(&pid);
        match decision {
            EXEC_ALLOW_NATIVE | EXEC_ALLOW_PGM => {
                inc_stat(STAT_EXEC_CHECK_ALLOWED);
                info!(
                    &ctx,
                    "Policy {}: bprm_check_security confirma ejecución permitida",
                    L400_POLICY_VERSION
                );
                0
            }
            EXEC_DENY_INVALID_TAG
            | EXEC_DENY_WRONG_TYPE
            | EXEC_DENY_INVALID_FORMAT
            | EXEC_DENY_EXCLUDE => {
                inc_stat(STAT_EXEC_CHECK_DENIED);
                warn!(
                    &ctx,
                    "Policy {}: bprm_check_security deniega ejecución", L400_POLICY_VERSION
                );
                EACCES
            }
            _ => 0,
        }
    } else {
        inc_stat(STAT_EXEC_DECISION_MISSING);
        warn!(
            &ctx,
            "Policy {}: bprm_check_security sin decisión previa, permitiendo por compatibilidad",
            L400_POLICY_VERSION
        );
        0
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
