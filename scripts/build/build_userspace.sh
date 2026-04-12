#!/bin/bash
# build_userspace.sh - Compila y empaqueta el userspace Linux/400 para live/install

set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"
TARGET_TRIPLE="${TARGET_TRIPLE:-x86_64-unknown-linux-musl}"
PROFILE="${PROFILE:-release}"
USERSPACE_DIR="${OUTPUT_DIR}/userspace"
BIN_DIR="${USERSPACE_DIR}/bin"
LIB_DIR="${USERSPACE_DIR}/lib"
HOOKS_DIR="${USERSPACE_DIR}/hooks"
ENABLE_CLC_LLVM="${ENABLE_CLC_LLVM:-0}"

mkdir -p "${BIN_DIR}" "${LIB_DIR}" "${HOOKS_DIR}"

echo "=== Compilando userspace Linux/400 ==="
echo "Target : ${TARGET_TRIPLE}"
echo "Perfil : ${PROFILE}"

if ! rustup target list --installed | grep -qx "${TARGET_TRIPLE}"; then
    echo ">> Instalando target Rust ${TARGET_TRIPLE}..."
    rustup target add "${TARGET_TRIPLE}"
fi

cd "${L400_SRC_DIR}"

COMMON_CARGO_ARGS=(--target "${TARGET_TRIPLE}")
if [ "${PROFILE}" = "release" ]; then
    COMMON_CARGO_ARGS+=(--release)
fi

echo ">> Compilando librería base..."
cargo build -p l400 --lib "${COMMON_CARGO_ARGS[@]}"

echo ">> Compilando loader eBPF..."
cargo build -p l400-loader "${COMMON_CARGO_ARGS[@]}"

echo ">> Compilando TUI..."
cargo build -p os400-tui "${COMMON_CARGO_ARGS[@]}"

echo ">> Compilando C/400..."
cargo build -p c400c "${COMMON_CARGO_ARGS[@]}"

echo ">> Compilando CL compiler..."
if [ "${ENABLE_CLC_LLVM}" = "1" ]; then
    cargo build -p clc --features llvm-backend "${COMMON_CARGO_ARGS[@]}"
else
    cargo build -p clc "${COMMON_CARGO_ARGS[@]}"
fi

TARGET_DIR="${L400_SRC_DIR}/target/${TARGET_TRIPLE}/${PROFILE}"

copy_required() {
    local src="$1"
    local dst="$2"

    if [ ! -f "${src}" ]; then
        echo "ERROR: artefacto requerido no encontrado: ${src}" >&2
        exit 1
    fi

    cp "${src}" "${dst}"
}

copy_optional() {
    local src="$1"
    local dst="$2"

    if [ -f "${src}" ]; then
        cp "${src}" "${dst}"
    else
        echo "WARNING: artefacto opcional no encontrado: ${src}"
    fi
}

find_artifact() {
    local name="$1"
    local candidate

    for candidate in \
        "${TARGET_DIR}/${name}" \
        "${TARGET_DIR}/deps/${name}"; do
        if [ -f "${candidate}" ]; then
            echo "${candidate}"
            return 0
        fi
    done

    return 1
}

copy_required "${TARGET_DIR}/os400-tui" "${BIN_DIR}/os400-tui"
copy_required "${TARGET_DIR}/l400-loader" "${BIN_DIR}/l400-loader"
copy_required "${TARGET_DIR}/c400c" "${BIN_DIR}/c400c"
copy_required "${TARGET_DIR}/clc" "${BIN_DIR}/clc"
copy_required "$(find_artifact libl400.a)" "${LIB_DIR}/libl400.a"
if libl400_so="$(find_artifact libl400.so 2>/dev/null)"; then
    copy_optional "${libl400_so}" "${LIB_DIR}/libl400.so"
fi

echo ">> Compilando bytecode eBPF..."
if cargo build --manifest-path "${L400_SRC_DIR}/l400-ebpf/Cargo.toml" \
    --target bpfel-unknown-none --release; then
    copy_optional \
        "${L400_SRC_DIR}/target/bpfel-unknown-none/release/l400-ebpf" \
        "${HOOKS_DIR}/l400-ebpf"
elif rustup component add rust-src --toolchain nightly >/dev/null 2>&1 && \
    cargo +nightly build -Z build-std=core \
        --manifest-path "${L400_SRC_DIR}/l400-ebpf/Cargo.toml" \
        --target bpfel-unknown-none --release; then
    copy_optional \
        "${L400_SRC_DIR}/target/bpfel-unknown-none/release/l400-ebpf" \
        "${HOOKS_DIR}/l400-ebpf"
else
    echo "WARNING: no se pudo compilar l400-ebpf; la ISO seguirá sin hook cargable."
fi

echo "=== Userspace listo ==="
find "${USERSPACE_DIR}" -maxdepth 2 -type f | sort
