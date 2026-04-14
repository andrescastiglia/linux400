#![no_std]

// Shared types between kernel and user space

pub const L400_POLICY_VERSION: &str = "phase3-v1";

pub const STAT_OPEN_ALLOWED: u32 = 0;
pub const STAT_DENIED_INVALID_TAG: u32 = 1;
pub const STAT_EXEC_ALLOWED_NATIVE: u32 = 2;
pub const STAT_EXEC_ALLOWED_PGM: u32 = 3;
pub const STAT_EXEC_DENIED_WRONG_TYPE: u32 = 4;
pub const STAT_EXEC_DECISION_MISSING: u32 = 5;
pub const STAT_EXEC_CHECK_ALLOWED: u32 = 6;
pub const STAT_EXEC_CHECK_DENIED: u32 = 7;
pub const STAT_OBJTYPE_BASE: u32 = 16;

#[derive(Copy, Clone)]
pub struct L400ObjType {
    pub prefix: [u8; 4],
    pub name: &'static str,
}

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
];
