#!/bin/sh
# install_linux400.sh - Instala Linux/400 desde el entorno live a disco

set -eu

TARGET_MNT="${TARGET_MNT:-/mnt/linux400-target}"
EFI_SIZE_MIB="${EFI_SIZE_MIB:-512}"
ROOT_LABEL="${ROOT_LABEL:-linux400-root}"
EFI_LABEL="${EFI_LABEL:-LINUX400_EFI}"
INSTALL_MODE="${INSTALL_MODE:-uefi}"
AUTO_PARTITION="${AUTO_PARTITION:-1}"

usage() {
    cat <<'EOF'
Uso:
  install_linux400.sh /dev/sdX
  install_linux400.sh /dev/nvme0n1

Variables opcionales:
  AUTO_PARTITION=0         Usa particiones ya creadas en ROOT_PART y EFI_PART
  ROOT_PART=/dev/sdX2
  EFI_PART=/dev/sdX1
  TARGET_MNT=/mnt/linux400-target
  EFI_SIZE_MIB=512
EOF
}

require_root() {
    if [ "$(id -u)" -ne 0 ]; then
        echo "ERROR: este instalador requiere root." >&2
        exit 1
    fi
}

have_cmd() {
    command -v "$1" >/dev/null 2>&1
}

require_device() {
    if [ ! -b "$1" ]; then
        echo "ERROR: dispositivo no válido: $1" >&2
        exit 1
    fi
}

partition_disk() {
    local disk="$1"

    if have_cmd sfdisk; then
        cat <<EOF | sfdisk --wipe always "${disk}"
label: gpt
unit: MiB

first-lba: 2048

size=${EFI_SIZE_MIB}, type=U, name="LINUX400-EFI"
type=L, name="LINUX400-ROOT"
EOF
        return 0
    fi

    echo "ERROR: no se encontró sfdisk para particionar automáticamente." >&2
    echo "Configura AUTO_PARTITION=0 y pasa ROOT_PART / EFI_PART ya creadas." >&2
    exit 1
}

resolve_parts() {
    local disk="$1"

    if [ "${AUTO_PARTITION}" = "1" ]; then
        partition_disk "${disk}"
        sleep 2
    fi

    if [ -z "${EFI_PART:-}" ] || [ -z "${ROOT_PART:-}" ]; then
        case "${disk}" in
            *nvme*|*mmcblk*)
                EFI_PART="${EFI_PART:-${disk}p1}"
                ROOT_PART="${ROOT_PART:-${disk}p2}"
                ;;
            *)
                EFI_PART="${EFI_PART:-${disk}1}"
                ROOT_PART="${ROOT_PART:-${disk}2}"
                ;;
        esac
    fi

    require_device "${EFI_PART}"
    require_device "${ROOT_PART}"
}

format_parts() {
    if have_cmd mkfs.fat; then
        mkfs.fat -F 32 -n "${EFI_LABEL}" "${EFI_PART}"
    else
        mkdosfs -F 32 -n "${EFI_LABEL}" "${EFI_PART}"
    fi

    if have_cmd mkfs.ext4; then
        mkfs.ext4 -F -L "${ROOT_LABEL}" "${ROOT_PART}"
    else
        mke2fs -t ext4 -F -L "${ROOT_LABEL}" "${ROOT_PART}"
    fi
}

mount_target() {
    mkdir -p "${TARGET_MNT}"
    mount "${ROOT_PART}" "${TARGET_MNT}"
    mkdir -p "${TARGET_MNT}/boot/efi"
    mount "${EFI_PART}" "${TARGET_MNT}/boot/efi"
}

copy_rootfs() {
    tar \
        --exclude="${TARGET_MNT}" \
        --exclude=/proc \
        --exclude=/sys \
        --exclude=/dev \
        --exclude=/run \
        --exclude=/tmp \
        --exclude=/mnt \
        --exclude=/media \
        --exclude=/l400 \
        --exclude=/var/cache/apk \
        -cpf - / | tar -xpf - -C "${TARGET_MNT}"
}

install_boot_assets() {
    local iso_boot_dir=""
    local efi_asset=""
    local candidate

    for candidate in \
        "${L400_BOOT_ASSET_DIR:-}" \
        "/run/l400/media/boot" \
        "/opt/l400/boot"; do
        [ -n "${candidate}" ] || continue
        if [ -d "${candidate}" ]; then
            iso_boot_dir="${candidate}"
            break
        fi
    done

    for candidate in \
        "${L400_BOOT_ASSET_DIR:-}" \
        "/run/l400/media/live" \
        "/opt/l400/boot"; do
        [ -n "${candidate}" ] || continue
        if [ -f "${candidate}/BOOTX64.EFI" ]; then
            efi_asset="${candidate}/BOOTX64.EFI"
            break
        fi
    done

    if [ -z "${iso_boot_dir}" ]; then
        echo "ERROR: no se encontró /run/l400/media/boot con los artefactos del live ISO." >&2
        echo "Sugerencia: exporta L400_BOOT_ASSET_DIR=/ruta/con/vmlinuz initramfs.img y BOOTX64.EFI." >&2
        exit 1
    fi

    mkdir -p "${TARGET_MNT}/boot" "${TARGET_MNT}/boot/efi/EFI/BOOT" "${TARGET_MNT}/boot/efi/EFI/Linux400"

    cp "${iso_boot_dir}/vmlinuz" "${TARGET_MNT}/boot/efi/EFI/Linux400/vmlinuz"
    cp "${iso_boot_dir}/initramfs.img" "${TARGET_MNT}/boot/efi/EFI/Linux400/initramfs.img"

    if [ -n "${efi_asset}" ]; then
        cp "${efi_asset}" "${TARGET_MNT}/boot/efi/EFI/BOOT/BOOTX64.EFI"
    else
        echo "ERROR: BOOTX64.EFI no encontrado dentro de los assets de instalación." >&2
        exit 1
    fi

    cat > "${TARGET_MNT}/boot/efi/EFI/BOOT/grub.cfg" <<'EOF'
set timeout=5
set default=0

menuentry "Linux/400" {
    linux /EFI/Linux400/vmlinuz root=LABEL=linux400-root rw quiet l400.installed=1
    initrd /EFI/Linux400/initramfs.img
}
EOF
}

configure_installed_system() {
    mkdir -p "${TARGET_MNT}/etc"

    cat > "${TARGET_MNT}/etc/fstab" <<EOF
LABEL=${ROOT_LABEL} / ext4 defaults 0 1
LABEL=${EFI_LABEL} /boot/efi vfat umask=0077 0 2
EOF

    if [ -f "${TARGET_MNT}/etc/inittab" ]; then
        sed -i 's#^tty1::respawn:.*#tty1::respawn:/sbin/getty 115200 tty1#' "${TARGET_MNT}/etc/inittab"
    fi

    mkdir -p "${TARGET_MNT}/home/l400"
    chown -R 1000:1000 "${TARGET_MNT}/home/l400" 2>/dev/null || true
}

cleanup_mounts() {
    sync
    umount "${TARGET_MNT}/boot/efi" 2>/dev/null || true
    umount "${TARGET_MNT}" 2>/dev/null || true
}

main() {
    require_root

    if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
        usage
        exit 0
    fi

    if [ $# -lt 1 ]; then
        usage
        exit 1
    fi

    local disk="$1"
    require_device "${disk}"

    resolve_parts "${disk}"
    format_parts
    mount_target
    trap cleanup_mounts EXIT
    copy_rootfs
    install_boot_assets
    configure_installed_system

    echo "=== Linux/400 instalado ==="
    echo "Disco : ${disk}"
    echo "EFI   : ${EFI_PART}"
    echo "Root  : ${ROOT_PART}"
}

main "$@"
