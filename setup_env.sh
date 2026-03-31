#!/bin/bash
# Script de Toolchain para Linux/400
# Compila las bases y setea los entornos virtuales y paths para que `clc` este en $PATH

set -e
echo "==== Inicializando Toolchain Linux/400 ===="

ROOT_DIR="/home/user/Source/os400/cl_compiler"

if [ ! -d "$ROOT_DIR" ]; then
    echo "Falla: No se ha detectado el source del toolchain en $ROOT_DIR"
    exit 1
fi

echo ">> Compilando el Compilador CL y la Runtime..."
cd "$ROOT_DIR"
cargo build --release

echo ">> Instalando clc en el binario local del usuario..."
mkdir -p "$HOME/.local/bin"
ln -sf "$ROOT_DIR/target/release/clc" "$HOME/.local/bin/clc"

export PATH="$HOME/.local/bin:$PATH"

echo ">> Compilando BPF l400-ebpf (requiere bpf-linker)..."
cd "/home/user/Source/os400/l400-ebpf"
cargo build --target bpfel-unknown-none --release

echo ">> Compilando Loader l400-loader..."
cd "/home/user/Source/os400/l400-loader"
cargo build --release

echo ">> Toolchain y módulos de Seguridad instalados exitosamente."
echo ">> Utiliza 'clc --help' para confirmar que el compilador responde."
echo ">> Utiliza 'sudo /home/user/Source/os400/l400-loader/target/release/l400-loader' para iniciar el daemon de seguridad."
