# Imagen base oficial de Ubuntu 26.04
FROM ubuntu:26.04

# Prevenir interacciones bloqueantes y consolidar ENV
ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# 1. Instalar Toolchain del Sistema + ca-certificates (CRÍTICO)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    pkg-config \
    llvm \
    llvm-dev \
    clang \
    libclang-dev \
    lld \
    libelf-dev \
    zlib1g-dev \
    libdb-dev \
    libpolly-21-dev \
    && rm -rf /var/lib/apt/lists/*

# Verificar que LLVM 21 (nativo de Ubuntu 26.04) está correctamente instalado
RUN llvm-config --version

# 2. Instalar Rust via rustup
# Se usa el PATH definido en ENV para que rustup sea reconocido inmediatamente
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain nightly && \
    # Usamos la ruta completa o confiamos en el ENV PATH definido arriba
    /usr/local/cargo/bin/rustup component add rust-src

# 3. Instalar bpf-linker
RUN cargo install bpf-linker

WORKDIR /linux400

CMD ["/bin/bash"]