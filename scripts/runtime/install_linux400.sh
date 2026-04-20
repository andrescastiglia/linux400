#!/bin/sh
# install_linux400.sh - Instala Linux/400 desde el entorno live a disco

set -eu

TARGET_MNT="${TARGET_MNT:-/mnt/linux400-target}"
EFI_SIZE_MIB="${EFI_SIZE_MIB:-512}"
ROOT_LABEL="${ROOT_LABEL:-linux400-root}"
EFI_LABEL="${EFI_LABEL:-L400EFI}"
INSTALL_MODE="${INSTALL_MODE:-uefi}"
AUTO_PARTITION="${AUTO_PARTITION:-1}"
EFI_ACCESS_MODE="mount"

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

ensure_live_media_assets() {
    local media_dir="/run/l400/media"
    local dev=""

    if [ -d "${media_dir}/boot" ] && [ -f "${media_dir}/live/BOOTX64.EFI" ]; then
        return 0
    fi

    mkdir -p "${media_dir}"

    for dev in /dev/sr0 /dev/cdrom /dev/vdb /dev/sdb; do
        [ -b "${dev}" ] || continue
        if mountpoint -q "${media_dir}" 2>/dev/null; then
            break
        fi
        if mount -t iso9660 -o ro "${dev}" "${media_dir}" 2>/dev/null || \
            mount -o ro "${dev}" "${media_dir}" 2>/dev/null; then
            break
        fi
    done
}

require_device() {
    if [ ! -b "$1" ]; then
        echo "ERROR: dispositivo no válido: $1" >&2
        exit 1
    fi
}

log_mount_debug() {
    local device="$1"
    local mount_dir="$2"

    echo "DEBUG: mount target=${mount_dir} device=${device}" >&2
    echo "DEBUG: PATH=${PATH}" >&2
    echo "DEBUG: kernel filesystems:" >&2
    cat /proc/filesystems 2>/dev/null >&2 || true
    echo "DEBUG: lsblk del dispositivo:" >&2
    lsblk -f "${device}" 2>/dev/null >&2 || true
    echo "DEBUG: blkid del dispositivo:" >&2
    blkid "${device}" 2>/dev/null >&2 || true
    echo "DEBUG: binarios relevantes:" >&2
    command -v mount >&2 2>/dev/null || true
    command -v mount.vfat >&2 2>/dev/null || true
    command -v blkid >&2 2>/dev/null || true
    command -v findfs >&2 2>/dev/null || true
}

prepare_mtools_efi_access() {
    if ! have_cmd mcopy || ! have_cmd mmd; then
        return 1
    fi

    if mmd -D o -i "${EFI_PART}" ::/EFI >>/tmp/l400-mount-efi.log 2>&1 || \
        mdir -i "${EFI_PART}" ::/EFI >>/tmp/l400-mount-efi.log 2>&1; then
        EFI_ACCESS_MODE="mtools"
        return 0
    fi

    return 1
}

mount_efi_partition() {
    if mount -t vfat "${EFI_PART}" "${TARGET_MNT}/boot/efi" 2>/tmp/l400-mount-efi.log; then
        EFI_ACCESS_MODE="mount"
        return 0
    fi

    if mount -t vfat -o codepage=850 "${EFI_PART}" "${TARGET_MNT}/boot/efi" >>/tmp/l400-mount-efi.log 2>&1; then
        EFI_ACCESS_MODE="mount"
        return 0
    fi

    if mount "${EFI_PART}" "${TARGET_MNT}/boot/efi" >>/tmp/l400-mount-efi.log 2>&1; then
        EFI_ACCESS_MODE="mount"
        return 0
    fi

    if mount -t vfat -o utf8=1,iocharset=utf8,codepage=437 "${EFI_PART}" "${TARGET_MNT}/boot/efi" \
        >>/tmp/l400-mount-efi.log 2>&1; then
        EFI_ACCESS_MODE="mount"
        return 0
    fi

    if mount -t vfat -o utf8=0,iocharset=ascii,codepage=437 "${EFI_PART}" "${TARGET_MNT}/boot/efi" \
        >>/tmp/l400-mount-efi.log 2>&1; then
        EFI_ACCESS_MODE="mount"
        return 0
    fi

    prepare_mtools_efi_access
}

partition_disk() {
    local disk="$1"

    if have_cmd sfdisk; then
        cat <<EOF | sfdisk --wipe always "${disk}"
label: gpt
size=${EFI_SIZE_MIB}MiB, type=U, name="LINUX400-EFI"
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
    modprobe vfat 2>/dev/null || true
    modprobe fat 2>/dev/null || true
    modprobe nls_cp437 2>/dev/null || true
    modprobe nls_cp850 2>/dev/null || true
    modprobe nls_ascii 2>/dev/null || true
    modprobe nls_utf8 2>/dev/null || true

    mkdir -p "${TARGET_MNT}"
    mount "${ROOT_PART}" "${TARGET_MNT}"
    mkdir -p "${TARGET_MNT}/boot/efi"

    if mount_efi_partition; then
        return 0
    fi

    echo "ERROR: no se pudo montar la partición EFI ${EFI_PART} en ${TARGET_MNT}/boot/efi" >&2
    cat /tmp/l400-mount-efi.log >&2 || true
    log_mount_debug "${EFI_PART}" "${TARGET_MNT}/boot/efi"
    exit 1
}

copy_rootfs() {
    (
        cd /
        tar \
            --exclude=./proc \
            --exclude=./sys \
            --exclude=./dev \
            --exclude=./run \
            --exclude=./tmp \
            --exclude=./mnt \
            --exclude=./media \
            --exclude=./l400 \
            --exclude=./var/cache/apk \
            -cpf - .
    ) | tar -xpf - -C "${TARGET_MNT}"
}

install_boot_assets() {
    local iso_boot_dir=""
    local efi_asset=""
    local candidate

    ensure_live_media_assets

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

    cp "${iso_boot_dir}/vmlinuz" "${TARGET_MNT}/boot/vmlinuz"
    cp "${iso_boot_dir}/initramfs.img" "${TARGET_MNT}/boot/initramfs.img"

    if [ -z "${efi_asset}" ]; then
        echo "ERROR: BOOTX64.EFI no encontrado dentro de los assets de instalación." >&2
        exit 1
    fi

    cat > /tmp/l400-grub.cfg <<'EOF'
set timeout=5
set default=0
search --no-floppy --file /EFI/BOOT/BOOTX64.EFI --set=root

menuentry "Linux/400" {
    linux /EFI/LINUX400/VMLINUZ root=LABEL=linux400-root rw quiet console=tty0 console=ttyS0,115200 l400.installed=1 l400.efi=LABEL=L400EFI
    initrd /EFI/LINUX400/INITRD.IMG
}
EOF

    case "${EFI_ACCESS_MODE}" in
        mount)
            mkdir -p "${TARGET_MNT}/boot/efi/EFI/BOOT" "${TARGET_MNT}/boot/efi/EFI/LINUX400"
            cp "${iso_boot_dir}/vmlinuz" "${TARGET_MNT}/boot/efi/EFI/LINUX400/VMLINUZ"
            cp "${iso_boot_dir}/initramfs.img" "${TARGET_MNT}/boot/efi/EFI/LINUX400/INITRD.IMG"
            cp "${efi_asset}" "${TARGET_MNT}/boot/efi/EFI/BOOT/BOOTX64.EFI"
            cp /tmp/l400-grub.cfg "${TARGET_MNT}/boot/efi/EFI/BOOT/grub.cfg"
            ;;
        mtools)
            mmd -D o -i "${EFI_PART}" ::/EFI ::/EFI/BOOT ::/EFI/LINUX400 >/dev/null 2>&1 || true
            mcopy -D o -n -i "${EFI_PART}" "${iso_boot_dir}/vmlinuz" ::/EFI/LINUX400/VMLINUZ
            mcopy -D o -n -i "${EFI_PART}" "${iso_boot_dir}/initramfs.img" ::/EFI/LINUX400/INITRD.IMG
            mcopy -D o -n -i "${EFI_PART}" "${efi_asset}" ::/EFI/BOOT/BOOTX64.EFI
            mcopy -D o -n -i "${EFI_PART}" /tmp/l400-grub.cfg ::/EFI/BOOT/grub.cfg
            ;;
        *)
            echo "ERROR: modo EFI desconocido: ${EFI_ACCESS_MODE}" >&2
            exit 1
            ;;
    esac
}

configure_installed_system() {
    mkdir -p "${TARGET_MNT}/etc"

    cat > "${TARGET_MNT}/etc/fstab" <<EOF
LABEL=${ROOT_LABEL} / ext4 defaults 0 1
LABEL=${EFI_LABEL} /boot/efi vfat umask=0077 0 2
EOF

    if [ -f "${TARGET_MNT}/etc/inittab" ]; then
        if grep -q '^tty1::respawn:' "${TARGET_MNT}/etc/inittab"; then
            sed -i 's#^tty1::respawn:.*#tty1::respawn:/sbin/getty -n -l /usr/local/bin/l400-console-autologin 115200 tty1 linux#' \
                "${TARGET_MNT}/etc/inittab"
        else
            cat >> "${TARGET_MNT}/etc/inittab" <<'EOF'
tty1::respawn:/sbin/getty -n -l /usr/local/bin/l400-console-autologin 115200 tty1 linux
EOF
        fi

        if grep -q '^ttyS0::respawn:' "${TARGET_MNT}/etc/inittab"; then
            sed -i 's#^ttyS0::respawn:.*#ttyS0::respawn:/sbin/getty -L -n -l /usr/local/bin/l400-console-autologin 115200 ttyS0 vt100#' \
                "${TARGET_MNT}/etc/inittab"
        else
            cat >> "${TARGET_MNT}/etc/inittab" <<'EOF'
ttyS0::respawn:/sbin/getty -L -n -l /usr/local/bin/l400-console-autologin 115200 ttyS0 vt100
EOF
        fi
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
