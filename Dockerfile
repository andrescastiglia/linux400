# Imagen base oficial de Ubuntu 22.04 LTS (Jammy)
FROM ubuntu:22.04

# Prevenir interacciones bloqueantes en apt-get
ENV DEBIAN_FRONTEND=noninteractive

# 1. Instalar Toolchain del Sistema (C/C++, LLVM 15, BPF, Berkeley DB y ZFS)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    git \
    pkg-config \
    llvm-15 \
    llvm-15-dev \
    clang-15 \
    libclang-15-dev \
    lld-15 \
    libelf-dev \
    zlib1g-dev \
    libdb-dev \
    libzfs-dev \
    && rm -rf /var/lib/apt/lists/*

# Configurar LLVM 15 como default
RUN ln -s /usr/bin/llvm-config-15 /usr/bin/llvm-config && \
    ln -s /usr/bin/clang-15 /usr/bin/clang && \
    ln -s /usr/bin/lld-15 /usr/bin/lld

# 2. Instalar Rust via rustup (Se requiere Nightly para compilar BPF con Aya)
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly && \
    rustup component add rust-src

# 3. Instalar dependencias puras de eBPF (bpf-linker para el crate l400-ebpf)
# Nota: La compilación de bpf-linker toma unos minutos en el build de Docker.
RUN cargo install bpf-linker

# 4. Configurar Directorio de Trabajo
WORKDIR /linux400

# Punto de entrada por defecto
CMD ["/bin/bash"]
