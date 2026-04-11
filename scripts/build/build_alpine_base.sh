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
apk --root "${ROOTFS_DIR}" --arch "${ARCH}" add \
    alpine-base \
    bash \
    zfs-utils-linux \
    zfs-dkms \
    libstdc++ \
    clang \
    llvm20 \
    musl \
    openssh \
    openssl \
    curl \
    zlib \
    libuv \
    elfutils \
    file \
    grep \
    gawk \
    sed \
    coreutils \
    util-linux \
    e2fsprogs \
    dosfstools \
    mtools \
    squashfs-tools \
    cdrkit

# Configurar locale
echo ">> Configurando locale..."
sed -i 's/#en_US.UTF-8/en_US.UTF-8/' "${ROOTFS_DIR}/etc/locale.gen"
sed -i 's/#en_US ISO-8859-1/en_US ISO-8859-1/' "${ROOTFS_DIR}/etc/locale.gen"

# Crear usuario l400
echo ">> Creando usuario l400..."
adduser -D -s /bin/bash l400 -G users,wheel < "${ROOTFS_DIR}/etc/passwd" 2>/dev/null || true
echo "l400:l400" | chpasswd -R "${ROOTFS_DIR}"

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
L400_TARGET="${L400_SRC_DIR}/target/release"
[ -d "${L400_TARGET}" ] || L400_TARGET="${L400_SRC_DIR}/target/debug"

cp -r "${L400_TARGET}/libl400.so" "${ROOTFS_DIR}/lib/l400/" 2>/dev/null || true
cp -r "${L400_TARGET}/l400"* "${ROOTFS_DIR}/opt/l400/" 2>/dev/null || true
cp -r "${L400_SRC_DIR}/scripts/"* "${ROOTFS_DIR}/opt/l400/scripts/" 2>/dev/null || true

# Configurar PATH
echo 'export PATH="/opt/l400:$PATH"' >> "${ROOTFS_DIR}/etc/profile"
echo 'export L400_ROOT="/l400"' >> "${ROOTFS_DIR}/etc/profile"
echo 'export LD_LIBRARY_PATH="/lib/l400:$LD_LIBRARY_PATH"' >> "${ROOTFS_DIR}/etc/profile"

# Crear punto de montaje /l400
mkdir -p "${ROOTFS_DIR}/l400"

echo "=== Rootfs Alpine creado en ${ROOTFS_DIR} ==="
echo "Para probar: chroot ${ROOTFS_DIR} /bin/bash"