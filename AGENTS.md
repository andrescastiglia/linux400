# AGENTS.md - Linux/400 Development Guide

## Project

OS/400-style object model on Linux (ZFS xattrs, eBPF LSM, sled database).

## Crates

- `libl400/` - Core runtime (objects, PF/LF, data queues, ZFS helpers)
- `l400-ebpf/` - eBPF LSM program (Aya)
- `l400-ebpf-common/` - Shared types (no_std)
- `l400-loader/` - Privileged eBPF loader
- `cl_compiler/clc/` - CL compiler with Pest parser
- `c400_compiler/` - C frontend

## Build Commands

```bash
# User-space only (safe - avoids BPF toolchain)
cargo build -p c400c
cargo build -p clc
cargo build -p l400-loader

# eBPF (requires BPF toolchain)
cd l400-ebpf && cargo build --target bpfel-unknown-none --release

# WARNING: `cargo build` from root pulls in eBPF and may fail
```

## Test Commands

```bash
cargo test -p l400
cargo test -p l400 test_pf              # pattern match
cargo test -p l400 db::tests::test_name -- --exact  # exact match
```

## Lint/Format

```bash
cargo fmt --all
cargo clippy -p l400 --all-targets -- -D warnings
```

## Environment-Dependent Tests (require root + special setup)

```bash
sudo ./test_e2e_bpf.sh    # requires BPF LSM kernel support
sudo ./test_e2e_zfs.sh     # requires ZFS pool setup
./build_docker_env.sh      # Docker multi-arch build environment
./run_dev_env.sh           # ZFS + BPF dev container
```

## Platform Requirements

- Kernel >= 6.11 (eBPF LSM)
- ZFS with `xattr=sa`
- Root for loader/e2e flows

## Architecture

### Object Types

Authoritative boundary: `user.l400.objtype` xattr. Valid types in `l400-ebpf-common/src/lib.rs` (shared with eBPF).

```rust
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
```

### Sled Tree Names

- PF members: `"PF_MEMBER"`
- LF secondary indexes: `"LF_IDX_<name>"`
- Data queues: `"DTAQ"`

### Workspace Note

`cl_compiler/clc` links against top-level `libl400`, NOT `cl_compiler/libl400`.

## eBPF-Specific

- `l400-ebpf-common/` is `#![no_std]` - core types only
- Loader expects: `../l400-ebpf/target/bpfel-unknown-none/release/l400-ebpf`
