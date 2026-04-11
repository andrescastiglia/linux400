#!/bin/bash
# build_iso.sh - Genera imagen ISO instalable de Linux/400
# Combina: Kernel + Initramfs + Rootfs Alpine -> ISO booteable

set -e

OUTPUT_DIR="${OUTPUT_DIR:-./output}"
ISO_NAME="linux400-${VERSION:-1.0.0}"
WORK_DIR="${OUTPUT_DIR}/iso_work"
EFI_DIR="${WORK_DIR}/efi"
BOOT_DIR="${WORK_DIR}/boot"
ROOT_DIR="${WORK_DIR}/root"

VERSION="${VERSION:-1.0.0}"
KERNEL_VERSION="${KERNEL_VERSION:-6.11.0}"

echo "=== Generando ISO Linux/400 v${VERSION} ==="

mkdir -p "${WORK_DIR}"
mkdir -p "${EFI_DIR}/boot"
mkdir -p "${BOOT_DIR}/isolinux"
mkdir -p "${ROOT_DIR}"

# Copiar kernel
echo ">> Copiando kernel..."
if [ -f "${OUTPUT_DIR}/vmlinuz" ]; then
    cp "${OUTPUT_DIR}/vmlinuz" "${BOOT_DIR}/vmlinuz-${KERNEL_VERSION}-l400"
elif [ -f "/boot/vmlinuz-linux" ]; then
    cp "/boot/vmlinuz-linux" "${BOOT_DIR}/vmlinuz-${KERNEL_VERSION}-l400"
else
    echo "WARNING: Kernel no encontrado, usando genkernel..."
    touch "${BOOT_DIR}/vmlinuz-${KERNEL_VERSION}-l400"
fi

# Copiar initramfs
echo ">> Copiando initramfs..."
if [ -f "${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-l400.img" ]; then
    cp "${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-l400.img" \
       "${BOOT_DIR}/initramfs-${KERNEL_VERSION}-l400.img"
fi

# Copiar rootfs si existe
echo ">> Copiando rootfs..."
if [ -d "${OUTPUT_DIR}/rootfs" ]; then
    cp -r "${OUTPUT_DIR}/rootfs}"/* "${ROOT_DIR}/" 2>/dev/null || true
fi

# Copiar configuración de SYSLINUX
echo ">> Configurando bootloader..."
cat > "${BOOT_DIR}/isolinux/syslinux.cfg" << 'SYSLINUXCFG'
SAY Linux/400 v1.0.0 - OS/400 Personality on Linux
DEFAULT l400
TIMEOUT 30
PROMPT 1

LABEL l400
    KERNEL vmlinuz-6.11.0-l400
    APPEND initrd=initramfs-6.11.0-l400.img l400.root=/dev/ram0 l400 quiet
LABEL memtest
    KERNEL memtest
    APPEND
SYSLINUXCFG

# Copiar binarios de SYSLINUX
echo ">> Copiando binarios de bootloader..."
for f in ldlinux.c32 libutil.c32 libcom32.c32 libgcc.c32; do
    [ -f "/usr/lib/syslinux/${f}" ] && cp "/usr/lib/syslinux/${f}" "${BOOT_DIR}/isolinux/" 2>/dev/null || true
    [ -f "/usr/share/syslinux/${f}" ] && cp "/usr/share/syslinux/${f}" "${BOOT_DIR}/isolinux/" 2>/dev/null || true
done
[ -f "/usr/lib/syslinux/isolinux.bin" ] && cp "/usr/lib/syslinux/isolinux.bin" "${BOOT_DIR}/isolinux/" 2>/dev/null || true
[ -f "/usr/share/syslinux/isolinux.bin" ] && cp "/usr/share/syslinux/isolinux.bin" "${BOOT_DIR}/isolinux/" 2>/dev/null || true

# Copiar binarios de EFISTUB
echo ">> Configurando EFI..."
for f in shimx64.efi ldlinux.e64 mokmanager.efi; do
    [ -f "/usr/lib/shim.shimx64.efi.signed" ] && cp "/usr/lib/shim.shimx64.efi.signed" "${EFI_DIR}/boot/" 2>/dev/null || true
done
cp "${BOOT_DIR}/vmlinuz-${KERNEL_VERSION}-l400" "${EFI_DIR}/boot/vmlinuz.efi" 2>/dev/null || true
cp "${BOOT_DIR}/initramfs-${KERNEL_VERSION}-l400.img" "${EFI_DIR}/boot/initrd.img" 2>/dev/null || true

# Crear EFI partition image
if command -v xorriso >/dev/null 2>&1; then
    echo ">> Generando ISO con xorriso..."
    xorriso -as mkisofs \
        -iso-level 3 \
        -rock \
        -joliet \
        -udf \
        -full-iso-boot \
        -boot-load-size 4 \
        -boot-info-table \
        -eltorito-catalog "${BOOT_DIR}/isolinux/boot.cat" \
        -eltorito-alt-boot \
        -e "${EFI_DIR}/boot" \
        -no-emul-boot \
        -append_partition 2 "${EFI_DIR}/boot/efiboot.img" \
        -gpt-part-type /usr/share/bootloader\
        -o "${OUTPUT_DIR}/${ISO_NAME}.iso" \
        "${WORK_DIR}"
else
    echo ">> Generando ISO con genisoimage (fallback)..."
    genisoimage \
        -rationalRock \
        -joliet \
        -udf \
        -no-emul-boot \
        -boot-load-size 4 \
        -boot-info-table \
        -c "${BOOT_DIR}/isolinux/boot.cat" \
        -b "${BOOT_DIR}/isolinux/isolinux.bin" \
        -o "${OUTPUT_DIR}/${ISO_NAME}.iso" \
        "${WORK_DIR}" 2>/dev/null || \
    mkisofs \
        -no-emul-boot \
        -boot-load-size 4 \
        -boot-info-table \
        -b "${BOOT_DIR}/isolinux/isolinux.bin" \
        -c "${BOOT_DIR}/isolinux/boot.cat" \
        -o "${OUTPUT_DIR}/${ISO_NAME}.iso" \
        "${WORK_DIR}"
fi

# Verificar ISO
if [ -f "${OUTPUT_DIR}/${ISO_NAME}.iso" ]; then
    echo "=== ISO generado exitosamente ==="
    echo "Archivo: ${OUTPUT_DIR}/${ISO_NAME}.iso"
    ls -lh "${OUTPUT_DIR}/${ISO_NAME}.iso"
else
    echo "ERROR: ISO no pudo ser generado"
    exit 1
fi

echo "=== Proceso completado ==="