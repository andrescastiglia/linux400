#![no_std]

// Shared types between kernel and user space

/// V1 definitive policy version - updated for Phase 9 completion
pub const L400_POLICY_VERSION: &str = "v1.0";

pub const STAT_OPEN_ALLOWED: u32 = 0;
pub const STAT_DENIED_INVALID_TAG: u32 = 1;
pub const STAT_EXEC_ALLOWED_NATIVE: u32 = 2;
pub const STAT_EXEC_ALLOWED_PGM: u32 = 3;
pub const STAT_EXEC_DENIED_WRONG_TYPE: u32 = 4;
pub const STAT_EXEC_DECISION_MISSING: u32 = 5;
pub const STAT_EXEC_CHECK_ALLOWED: u32 = 6;
pub const STAT_EXEC_CHECK_DENIED: u32 = 7;
pub const STAT_EXEC_DENIED_INVALID_FORMAT: u32 = 8;
pub const STAT_EXEC_DENIED_EXCLUDE: u32 = 9;
pub const STAT_EXEC_ALLOWED_OWNER: u32 = 10;
pub const STAT_EXEC_ALLOWED_USER_AUTH: u32 = 11;
pub const STAT_OPEN_DENIED_EXCLUDE: u32 = 12;
pub const STAT_OPEN_ALLOWED_OWNER: u32 = 13;
pub const STAT_OPEN_ALLOWED_USER_AUTH: u32 = 14;
pub const STAT_OBJTYPE_BASE: u32 = 16;

#[derive(Copy, Clone)]
pub struct L400ObjType {
    pub prefix: [u8; 4],
    pub name: &'static str,
}

/// V1 definitive object types - aligned with runtime and eBPF enforcement
pub const VALID_OBJ_TYPES: &[L400ObjType] = &[
    L400ObjType {
        prefix: *b"*PGM",
        name: "*PGM",
    },
    L400ObjType {
        prefix: *b"*FIL",
        name: "*FILE",
    },
    L400ObjType {
        prefix: *b"*USR",
        name: "*USRPRF",
    },
    L400ObjType {
        prefix: *b"*LIB",
        name: "*LIB",
    },
    L400ObjType {
        prefix: *b"*DTA",
        name: "*DTAQ",
    },
    L400ObjType {
        prefix: *b"*CMD",
        name: "*CMD",
    },
    L400ObjType {
        prefix: *b"*SRV",
        name: "*SRVPGM",
    },
    L400ObjType {
        prefix: *b"*OUT",
        name: "*OUTQ",
    },
    L400ObjType {
        prefix: *b"*JOB",
        name: "*JOBQ",
    },
    L400ObjType {
        prefix: *b"*SPL",
        name: "*SPLF",
    },
    L400ObjType {
        prefix: *b"*AUT",
        name: "*AUTL",
    },
];
