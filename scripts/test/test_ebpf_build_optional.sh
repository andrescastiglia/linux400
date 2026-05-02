#!/bin/bash
# Build l400-ebpf when a BPF Rust toolchain is available.

set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
LOG_DIR="${LOG_DIR:-${L400_SRC_DIR}/output/test-logs}"

mkdir -p "${LOG_DIR}"

if ! command -v bpf-linker >/dev/null 2>&1; then
    echo "Skipping eBPF build: bpf-linker is not available."
    exit 0
fi

run_build() {
    local log_file="$1"
    shift

    echo "=== eBPF build: $* ==="
    if "$@" >"${log_file}" 2>&1; then
        echo "eBPF build OK (${log_file})"
        return 0
    fi

    echo "eBPF build failed (${log_file})" >&2
    sed -n '1,160p' "${log_file}" >&2 || true
    return 1
}

if command -v cargo >/dev/null 2>&1 && \
    command -v rustup >/dev/null 2>&1 && \
    rustup target list --installed | grep -qx bpfel-unknown-none; then
    if run_build "${LOG_DIR}/l400-ebpf-stable.log" \
        cargo build --manifest-path "${L400_SRC_DIR}/l400-ebpf/Cargo.toml" \
            --target bpfel-unknown-none --release; then
        exit 0
    fi
    exit 1
fi

if command -v rustup >/dev/null 2>&1 && rustup toolchain list | grep -Eq '^nightly'; then
    if rustup component add rust-src --toolchain nightly >/dev/null 2>&1; then
        run_build "${LOG_DIR}/l400-ebpf-nightly.log" \
            cargo +nightly build -Z build-std=core \
                --manifest-path "${L400_SRC_DIR}/l400-ebpf/Cargo.toml" \
                --target bpfel-unknown-none --release
        exit 0
    fi
fi

echo "Skipping eBPF build: bpfel-unknown-none/nightly build-std toolchain is not available."
