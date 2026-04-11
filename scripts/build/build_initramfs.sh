#!/bin/bash
# build_initramfs.sh - Crea initramfs con preload BPF LSM para Linux/400
# El BPF LSM se carga antes del init para garantizar protección desde el boot

set -e

OUTPUT_DIR="${OUTPUT_DIR:-./output}"
INITRAMFS_DIR="${OUTPUT_DIR}/initramfs"
KERNEL_VERSION="${KERNEL_VERSION:-6.11.0}"
L400_INIT="${INITRAMFS_DIR}/init"

echo "=== Construyendo Initramfs para Linux/400 ==="

rm -rf "${INITRAMFS_DIR}"
mkdir -p "${INITRAMFS_DIR}"/{bin,sbin,etc,lib,lib64,proc,sys,dev,run,l400/hooks}
chmod 1777 "${INITRAMFS_DIR}/dev"

# Copiar bins mínimos (static para evitar dependencias)
echo ">> Copiando bins mínimos..."
for bin in sh busybox mount umount mkdir mdev mkdir sed awk grep find cut cat; do
    cp -a "/bin/${bin}" "${INITRAMFS_DIR}/bin/" 2>/dev/null || \
    cp -a "/sbin/${bin}" "${INITRAMFS_DIR}/bin/" 2>/dev/null || \
    cp -a "/usr/bin/${bin}" "${INITRAMFS_DIR}/bin/" 2>/dev/null || true
done

# Configurar busybox
ln -sf busybox "${INITRAMFS_DIR}/bin/sh"
ln -sf busybox "${INITRAMFS_DIR}/bin/mount"
ln -sf busybox "${INITRAMFS_DIR}/bin/umount"
ln -sf busybox "${INITRAMFS_DIR}/bin/mkdir"
ln -sf busybox "${INITRAMFS_DIR}/bin/sed"
ln -sf busybox "${INITRAMFS_DIR}/bin/awk"
ln -sf busybox "${INITRAMFS_DIR}/bin/grep"
ln -sf busybox "${INITRAMFS_DIR}/bin/find"
ln -sf busybox "${INITRAMFS_DIR}/bin/cut"
ln -sf busybox "${INITRAMFS_DIR}/bin/cat"

# Copiar módulos del kernel
echo ">> Copiando módulos del kernel..."
if [ -d "/lib/modules/${KERNEL_VERSION}" ]; then
    cp -r "/lib/modules/${KERNEL_VERSION}" "${INITRAMFS_DIR}/lib/"
elif [ -d "/lib/modules/${KERNEL_VERSION}-l400" ]; then
    cp -r "/lib/modules/${KERNEL_VERSION}-l400" "${INITRAMFS_DIR}/lib/"
fi

# Directorio raíz del repositorio (soporte CI vía $L400_SRC_DIR)
L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"

# Compilar e incluir BPF LSM
echo ">> Compilando BPF LSM..."
if [ -d "${L400_SRC_DIR}/l400-ebpf" ]; then
    cd "${L400_SRC_DIR}/l400-ebpf"
    cargo build --target bpfel-unknown-none --release 2>/dev/null || true
    cp target/bpfel-unknown-none/release/l400-ebpf "${INITRAMFS_DIR}/l400/hooks/" 2>/dev/null || true
    cd - > /dev/null
fi

# Cargar el loader de eBPF (preferir release sobre debug)
echo ">> Preparando loader eBPF..."
if [ -f "${L400_SRC_DIR}/target/release/l400-loader" ]; then
    cp "${L400_SRC_DIR}/target/release/l400-loader" "${INITRAMFS_DIR}/bin/"
elif [ -f "${L400_SRC_DIR}/target/debug/l400-loader" ]; then
    cp "${L400_SRC_DIR}/target/debug/l400-loader" "${INITRAMFS_DIR}/bin/"
else
    echo "WARNING: l400-loader no encontrado; initramfs arrancará sin loader eBPF."
fi

# Dispositivos de terminal
echo ">> Configurando dispositivos..."
mknod "${INITRAMFS_DIR}/dev/null" c 1 3 2>/dev/null || true
mknod "${INITRAMFS_DIR}/dev/zero" c 1 5 2>/dev/null || true
mknod "${INITRAMFS_DIR}/dev/urandom" c 1 9 2>/dev/null || true
mknod "${INITRAMFS_DIR}/dev/random" c 1 8 2>/dev/null || true
mknod "${INITRAMFS_DIR}/dev/tty" c 5 0 2>/dev/null || true
mknod "${INITRAMFS_DIR}/dev/tty0" c 4 0 2>/dev/null || true
mknod "${INITRAMFS_DIR}/dev/console" c 5 1 2>/dev/null || true

# Script init
cat > "${L400_INIT}" << 'INITINITCH'
#!/bin/sh

export PATH=/bin:/sbin:/l400/bin
export LD_LIBRARY_PATH=/lib

echo "=== Linux/400 Bootstrap ==="
echo "Inicializando..."

mount -t proc none /proc
mount -t sysfs none /sys
mount -t devpts none /dev/pts 2>/dev/null || true
mount -t tmpfs none /run

mdev -s

# Cargar módulos del kernel necesarios
echo ">> Cargando módulos..."
modprobe zfs 2>/dev/null || true
modprobe zfs-arc 2>/dev/null || true

# Inicializar ZFS pool si existe el archivo
if [ -f /diskl400pool.img ]; then
    echo ">> Montando ZFS pool..."
    losetup /dev/loop0 /diskl400pool.img
    zpool import -d /dev -o cachefile=/run/zfs.cache l400pool 2>/dev/null || true
    zfs import -o cachefile=/run/zfs.cache l400pool 2>/dev/null || true
    mount -t zfs l400pool /l400 2>/dev/null || true
fi

# Cargar BPF LSM si está disponible
echo ">> Cargando BPF LSM..."
if [ -x /l400/hooks/l400-ebpf ]; then
    l400-loader &
    echo $! > /run/l400-loader.pid
fi

echo "=== Linux/400 Listo ==="
echo "Iniciando shell..."

exec /bin/sh

INITINITCH
chmod +x "${L400_INIT}"

# Crear initramfs.cpio
echo ">> Creando archivo initramfs..."
cd "${INITRAMFS_DIR}"
find . | cpio -H newc -ov > "${OUTPUT_DIR}/initramfs.cpio" 2>/dev/null || true
gzip -c "${OUTPUT_DIR}/initramfs.cpio" > "${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-l400.img"
rm "${OUTPUT_DIR}/initramfs.cpio"

echo "=== Initramfs creado ==="
echo "Ubicación: ${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-l400.img"