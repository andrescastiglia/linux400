#!/bin/bash
# build_distribution.sh - Orquesta la construcción live/install de Linux/400

set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"
ROOTFS_DIR="${ROOTFS_DIR:-${OUTPUT_DIR}/rootfs-build}"
KERNEL_VERSION="${KERNEL_VERSION:-$(uname -r)}"
LIVE_INITRAMFS_IMG="${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-l400.img"
INSTALL_INITRAMFS_IMG="${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-installed.img"

sync_live_boot_assets() {
    mkdir -p "${ROOTFS_DIR}/opt/l400/boot"

    cp "${OUTPUT_DIR}/vmlinuz" "${ROOTFS_DIR}/opt/l400/boot/vmlinuz"
    cp "${INSTALL_INITRAMFS_IMG}" "${ROOTFS_DIR}/opt/l400/boot/initramfs.img"
    cp "${OUTPUT_DIR}/BOOTX64.EFI" "${ROOTFS_DIR}/opt/l400/boot/BOOTX64.EFI"
}

export L400_SRC_DIR OUTPUT_DIR ROOTFS_DIR

"${L400_SRC_DIR}/scripts/build/build_userspace.sh"
"${L400_SRC_DIR}/scripts/build/build_alpine_base.sh"
EMBED_LIVE_ROOTFS=0 INITRAMFS_NAME="$(basename "${INSTALL_INITRAMFS_IMG}")" \
    "${L400_SRC_DIR}/scripts/build/build_initramfs.sh"
cp "${INSTALL_INITRAMFS_IMG}" "${LIVE_INITRAMFS_IMG}"
INITRAMFS_IMG="${LIVE_INITRAMFS_IMG}" "${L400_SRC_DIR}/scripts/build/build_iso.sh"
sync_live_boot_assets
INITRAMFS_NAME="$(basename "${LIVE_INITRAMFS_IMG}")" \
    "${L400_SRC_DIR}/scripts/build/build_initramfs.sh"
INITRAMFS_IMG="${LIVE_INITRAMFS_IMG}" "${L400_SRC_DIR}/scripts/build/build_iso.sh"

echo "=== Pipeline Linux/400 completado ==="
echo "Artefactos en: ${OUTPUT_DIR}"
