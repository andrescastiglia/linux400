# Copilot Instructions

## Build, test, and lint commands

Use targeted Cargo commands from the repository root. Plain `cargo build` currently pulls in the eBPF crate and fails without extra target/build-std setup, so prefer per-package builds.

```bash
# Core library tests
cargo test -p l400

# Run one exact unit test
cargo test -p l400 db::tests::test_create_pf_and_round_trip -- --exact

# Build the user-space tools that currently compile cleanly
cargo build -p c400c
cargo build -p clc
cargo build -p l400-loader

# Formatting / lint checks that exist for the Rust workspace
cargo fmt --all --check
cargo clippy -p l400 --all-targets -- -D warnings
```

`cargo fmt --all --check` and `cargo clippy -p l400 --all-targets -- -D warnings` are useful checks, but the current tree is not clean under them. Treat failures there as existing baseline issues unless your change touches the reported code.

Environment-dependent flows:

```bash
# Build the eBPF program from its own crate when the BPF toolchain is available
cd l400-ebpf && cargo build --target bpfel-unknown-none --release

# End-to-end checks require root plus ZFS/BPF support
sudo ./test_e2e_bpf.sh
sudo ./test_e2e_zfs.sh

# Provision the intended local dev environment
./build_docker_env.sh
./run_dev_env.sh
```

## High-level architecture

Linux/400 is built around an OS/400-style object model on top of Linux primitives:

- `libl400/` is the core runtime. It owns object creation/deletion/copying, ZFS xattr helpers, `*FILE`/logical-file emulation over `sled`, data queues, and aligned buffers for direct I/O.
- `l400-ebpf-common/` is the shared contract between user space and kernel space. The valid object types live here and are reused by both `libl400` and the eBPF program.
- `l400-ebpf/` is the Aya LSM program. Its `file_open` hook reads `user.l400.objtype` and allows or denies access based on the shared valid-type table while updating stats in `L400_STATS`.
- `l400-loader/` is the privileged user-space loader. It loads the compiled eBPF object, attaches `file_open` and `bprm_check_security`, and logs per-type stats. It expects the eBPF artifact at `../l400-ebpf/target/bpfel-unknown-none/release/l400-ebpf` relative to the loader process.
- `cl_compiler/clc/` parses CL source with Pest, emits an object file through LLVM when the `llvm-backend` feature is enabled or through a stub C/clang fallback otherwise, then links against `libl400` and stamps the output as `*PGM`.
- `c400_compiler/` is a simpler C frontend that shells out to `clang`/`cc`, links against `libl400`, and then stamps the result as `*PGM`.
- The shell scripts under `scripts/` and the root e2e scripts model the expected runtime layout: ZFS datasets mounted under `/l400`, libraries represented as datasets/directories, and object metadata carried in extended attributes.

## Key conventions

- `user.l400.objtype` is the authoritative object-type boundary. If you add a new object type, update the shared list in `l400-ebpf-common/src/lib.rs`; `libl400` validation and the eBPF allowlist both depend on it.
- The repo uses OS/400 object names and concepts directly (`*PGM`, `*FILE`, `*LIB`, `*DTAQ`, PF/LF, libraries). Keep those terms in code and docs instead of translating them to generic Linux names.
- The top-level workspace is the active build graph. `cl_compiler/clc` links against the top-level `libl400`, not `cl_compiler/libl400`; avoid editing the duplicate nested runtime unless you are intentionally reviving that older standalone workspace.
- Many environment assumptions are strict, not optional: kernel `>= 6.11`, ZFS with `xattr=sa`, and root privileges for loader/e2e flows. The project docs and helper scripts prefer explicit failure over silent fallback when those prerequisites are missing.
- `libl400` code treats direct I/O constraints as part of the design, not an optimization detail: `open_object_direct` uses `O_DIRECT` on Linux and `AlignedBuffer` rounds to 4096-byte alignment.
- Some setup docs and helper scripts still contain historical absolute paths under `/home/user/Source/os400`. For this repository, use paths relative to the current repo root instead of copying those literals forward.
