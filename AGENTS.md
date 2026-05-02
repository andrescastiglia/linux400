# AGENTS.md - Linux/400 Development Guide

## Project

Linux/400 is an OS/400-style object model on Linux backed by ZFS xattrs, an eBPF LSM, and a `sled`-based runtime for `*FILE` and `*DTAQ` flows. Refer to:
- `docs/KERNEL.md` for the project vision and target experience
- `docs/PROJECT.md` for current implementation status
- `docs/plan/implementation_plan.md` for the roadmap and gap analysis

## Workspace

- `libl400/` - Core runtime for objects, PF/LF handling, data queues, aligned I/O, and ZFS helpers
- `l400-ebpf-common/` - Shared `no_std` contract between user space and the eBPF program
- `l400-ebpf/` - Aya-based LSM program
- `l400-loader/` - Privileged loader for the eBPF program and policy status
- `cl_compiler/clc/` - CL compiler with Pest parser and optional LLVM backend
- `c400_compiler/` - C frontend that builds native `*PGM` objects
- `os400-tui/` - OS/400-style green-screen TUI

## Build Commands

Use targeted Cargo commands from the repository root. Plain `cargo build` pulls in `l400-ebpf` and may fail without the BPF toolchain.

```bash
# eBPF (requires BPF toolchain)
cd l400-ebpf && cargo build --target bpfel-unknown-none --release
```

`os400-tui` is now part of validated test gates; include in `cargo test -p os400-tui` runs.

## Test Commands

```bash
# Core library and toolchain
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
cargo test -p l400 test_pf
cargo test -p l400 db::tests::test_name -- --exact

# V1 demos / smoke tests
./scripts/test/test_objects_v1_demo.sh
./scripts/test/test_toolchain_v1_demo.sh
./scripts/test/test_workload_demo.sh
./scripts/test/test_loader_modes.sh
./scripts/test/test_release_rc.sh
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

## Lint / Format

```bash
cargo fmt --all --check
cargo clippy -p l400 --all-targets -- -D warnings
```

These are part of the rapid local gate. The full quality gate also includes `./scripts/test/test_release_rc.sh` and (for release candidates) `RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh`. The tree is not always baseline-clean outside the code you are touching.

## Environment-Dependent Flows

These require root and/or host setup:

```bash
sudo ./test_e2e_bpf.sh
sudo ./test_e2e_zfs.sh
./build_docker_env.sh
./run_dev_env.sh
./scripts/build/build_release_rc.sh
```

## Platform Requirements

- Kernel >= 6.11 for the eBPF LSM flow
- ZFS with `xattr=sa`
- Root privileges for loader and end-to-end flows

### Platform Profiles (from `docs/KERNEL.md`)
| Profile | Objective |
| --- | --- |
| `dev` | Local development without BPF/ZFS/root; all user-space components testable. |
| `degraded` | Installable system without full kernel enforcement; TUI/reports explicitly indicate degraded mode. |
| `full` | Active BPF LSM, BTF available, cgroups v2, `/l400` persistent with xattrs (preferably ZFS `xattr=sa`). |

## High-Level Architecture

- `libl400/` owns object creation/deletion/copying, ZFS xattr helpers, PF/LF emulation over `sled`, and data queues.
- `l400-ebpf-common/` centralizes policy/version constants and the valid Linux/400 object types.
- `l400-ebpf/` enforces object-type policy in the kernel.
- `l400-loader/` attaches the eBPF hooks and persists loader status for support/reporting flows.
- `clc` and `c400c` both produce native Linux binaries and catalog them as `*PGM`.
- `os400-tui/` provides the green-screen interface for workload and system views.

## Key Conventions

### Object Types

`user.l400.objtype` is the authoritative object-type boundary. If you add a new object type, update `l400-ebpf-common/src/lib.rs` because both `libl400` validation and the eBPF allowlist depend on it.

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

### Runtime Storage Names

- PF members: `"PF_MEMBER"`
- LF secondary indexes: `"LF_IDX_<name>"`
- Data queues: `"DTAQ"`

### Auth Manifest
`user.l400.auth.manifest` stores structured authorization data with fields: profile, UID, authority, origin (`explicit`, `public`, `owner`), and version. `GRTOBJAUT`/`RVKOBJAUT` maintain both the manifest and flat format entries for runtime/eBPF compatibility.

### Workspace Notes

- `cl_compiler/clc` links against the top-level `libl400`, not `cl_compiler/libl400`.
- `l400-ebpf-common/` is `#![no_std]`; keep it limited to shared core types/constants.
- `l400-loader` expects the compiled eBPF artifact at `../l400-ebpf/target/bpfel-unknown-none/release/l400-ebpf`.
- Prefer paths relative to the current repo root. Some older scripts/docs still contain historical absolute paths.
