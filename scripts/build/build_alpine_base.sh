#!/bin/bash
# build_alpine_base.sh - Construye un rootfs Alpine Linux con musl
# para el entorno Linux/400 mínimo

set -e

ALPINE_VERSION="3.20"
ARCH="${ARCH:-x86_64}"
OUTPUT_DIR="${OUTPUT_DIR:-./output}"
ROOTFS_DIR="${OUTPUT_DIR}/rootfs"

echo "=== Construyendo Alpine Linux Base para Linux/400 ==="
echo "Versión: ${ALPINE_VERSION}"
echo "Arquitectura: ${ARCH}"

mkdir -p "${ROOTFS_DIR}"

# Descargar Alpine minirootfs
MINIROOT="alpine-minirootfs-${ALPINE_VERSION}.0-${ARCH}.tar.gz"
MINIROOT_URL="https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}/releases/${ARCH}/${MINIROOT}"

echo ">> Descargando minirootfs..."
if [ ! -f "${OUTPUT_DIR}/${MINIROOT}" ]; then
    curl -L -o "${OUTPUT_DIR}/${MINIROOT}" "${MINIROOT_URL}"
fi

# Extraer rootfs
echo ">> Extrayendo rootfs..."
 tar -xzf "${OUTPUT_DIR}/${MINIROOT}" -C "${ROOTFS_DIR}"

# Configurar repositorios
echo ">> Configurando repositorios..."
cat > "${ROOTFS_DIR}/etc/apk/repositories" << EOF
https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}/main
https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}/community
EOF

# Instalar paquetes base
echo ">> Instalando paquetes base..."
apk --root "${ROOTFS_DIR}" --arch "${ARCH}" update

pick_first_available_pkg() {
    for pkg in "$@"; do
        if apk --root "${ROOTFS_DIR}" --arch "${ARCH}" add --simulate "$pkg" >/dev/null 2>&1; then
            echo "$pkg"
            return 0
        fi
    done
    return 1
}

LLVM_PKG=""
if LLVM_PKG=$(pick_first_available_pkg llvm20 llvm19 llvm18 llvm); then
    echo ">> LLVM detectado: ${LLVM_PKG}"
else
    echo "WARNING: No se encontró paquete LLVM; continuando sin LLVM explícito."
fi

ZFS_PKGS=()
if apk --root "${ROOTFS_DIR}" --arch "${ARCH}" search -x zfs >/dev/null 2>&1; then
    ZFS_PKGS+=(zfs)
elif apk --root "${ROOTFS_DIR}" --arch "${ARCH}" search -x zfs-lts >/dev/null 2>&1; then
    ZFS_PKGS+=(zfs-lts)
else
    echo "WARNING: No se encontró paquete ZFS en repositorios Alpine ${ALPINE_VERSION}."
fi

BASE_PACKAGES=(
    alpine-base
    bash
    libstdc++
    clang
    musl
    openssh
    openssl
    curl
    zlib
    libuv
    elfutils
    file
    grep
    gawk
    sed
    coreutils
    util-linux
    e2fsprogs
    dosfstools
    mtools
    squashfs-tools
    cdrkit
)

if [ -n "${LLVM_PKG}" ]; then
    BASE_PACKAGES+=("${LLVM_PKG}")
fi

BASE_PACKAGES+=("${ZFS_PKGS[@]}")

apk --root "${ROOTFS_DIR}" --arch "${ARCH}" add "${BASE_PACKAGES[@]}"

# Configurar locale
echo ">> Configurando locale..."
if [ -f "${ROOTFS_DIR}/etc/locale.gen" ]; then
    sed -i 's/#en_US.UTF-8/en_US.UTF-8/' "${ROOTFS_DIR}/etc/locale.gen"
    sed -i 's/#en_US ISO-8859-1/en_US ISO-8859-1/' "${ROOTFS_DIR}/etc/locale.gen"
fi

# Crear usuario l400
echo ">> Creando usuario l400..."
if ! chroot "${ROOTFS_DIR}" /bin/sh -c "id -u l400 >/dev/null 2>&1"; then
    chroot "${ROOTFS_DIR}" /bin/sh -c "adduser -D -s /bin/bash l400" || true
fi

if chroot "${ROOTFS_DIR}" /bin/sh -c "id -u l400 >/dev/null 2>&1"; then
    echo "l400:l400" | chroot "${ROOTFS_DIR}" /bin/sh -c "chpasswd" || true
else
    echo "WARNING: No se pudo crear usuario l400 dentro del rootfs."
fi

# Configurar hostname
echo "linux400" > "${ROOTFS_DIR}/etc/hostname"

# Copiar binarios de libl400
echo ">> Instalando binarios Linux/400..."
mkdir -p "${ROOTFS_DIR}/opt/l400"
mkdir -p "${ROOTFS_DIR}/lib/l400"
mkdir -p "${ROOTFS_DIR}/l400"

# Directorio raíz del repositorio (soporte CI vía $L400_SRC_DIR)
L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"

# Preferir artefactos release sobre debug
if [ -f "${L400_SRC_DIR}/target/release/libl400.so" ]; then
    L400_TARGET="${L400_SRC_DIR}/target/release"
elif [ -f "${L400_SRC_DIR}/target/debug/libl400.so" ]; then
    L400_TARGET="${L400_SRC_DIR}/target/debug"
else
    echo "WARNING: No se encontró libl400.so en target/release ni target/debug."
    L400_TARGET=""
fi

if [ -n "${L400_TARGET}" ]; then
    cp -r "${L400_TARGET}/libl400.so" "${ROOTFS_DIR}/lib/l400/" 2>/dev/null || \
        echo "WARNING: No se pudo copiar libl400.so desde ${L400_TARGET}."
    cp -r "${L400_TARGET}/l400"* "${ROOTFS_DIR}/opt/l400/" 2>/dev/null || \
        echo "WARNING: No se encontraron binarios l400* en ${L400_TARGET}."
fi
cp -r "${L400_SRC_DIR}/scripts/"* "${ROOTFS_DIR}/opt/l400/scripts/" 2>/dev/null || true

# Configurar PATH
echo 'export PATH="/opt/l400:$PATH"' >> "${ROOTFS_DIR}/etc/profile"
echo 'export L400_ROOT="/l400"' >> "${ROOTFS_DIR}/etc/profile"
echo 'export LD_LIBRARY_PATH="/lib/l400:$LD_LIBRARY_PATH"' >> "${ROOTFS_DIR}/etc/profile"

# Crear punto de montaje /l400
mkdir -p "${ROOTFS_DIR}/l400"

echo "=== Rootfs Alpine creado en ${ROOTFS_DIR} ==="
echo "Para probar: chroot ${ROOTFS_DIR} /bin/bash"