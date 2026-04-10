#![no_std]

// Shared types between kernel and user space

#[derive(Copy, Clone)]
pub struct L400ObjType {
    pub prefix: [u8; 4],
    pub name: &'static str,
}

pub const VALID_OBJ_TYPES: &[L400ObjType] = &[
    L400ObjType { prefix: *b"*PGM", name: "*PGM" },
    L400ObjType { prefix: *b"*FIL", name: "*FILE" },
    L400ObjType { prefix: *b"*USR", name: "*USRPRF" },
    L400ObjType { prefix: *b"*LIB", name: "*LIB" },
    L400ObjType { prefix: *b"*DTA", name: "*DTAQ" },
    L400ObjType { prefix: *b"*CMD", name: "*CMD" },
    L400ObjType { prefix: *b"*SRV", name: "*SRVPGM" },
    L400ObjType { prefix: *b"*OUT", name: "*OUTQ" },
];
