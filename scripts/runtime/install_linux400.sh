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

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" >&2
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

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
    local device="$1"
    if [ ! -b "${device}" ]; then
        error "Dispositivo no válido o no existe: ${device}"
        error "Verifica que el disco esté conectado y sea un dispositivo de bloque."
        exit 1
    fi
    # Check if device is not mounted
    if grep -q "^${device}" /proc/mounts 2>/dev/null; then
        error "El dispositivo ${device} está montado. Desmóntalo antes de continuar."
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

    if [ ! -b "${disk}" ]; then
        error "Dispositivo no válido para particionar: ${disk}"
        exit 1
    fi

    # Check if disk has enough space (minimum 2GB)
    local disk_size_bytes
    disk_size_bytes=$(blockdev --getsize64 "${disk}" 2>/dev/null || echo "0")
    local min_size_bytes=2147483648  # 2GB
    if [ "${disk_size_bytes}" -lt "${min_size_bytes}" ]; then
        error "El disco ${disk} es demasiado pequeño. Mínimo 2GB requerido."
        exit 1
    fi

    if have_cmd sfdisk; then
        info "Particionando ${disk}..."
        if ! cat <<EOF | sfdisk --wipe always "${disk}" 2>/tmp/l400-sfdisk.log; then
label: gpt
size=${EFI_SIZE_MIB}MiB, type=U, name="LINUX400-EFI"
type=L, name="LINUX400-ROOT"
EOF
            error "sfdisk falló al particionar ${disk}"
            cat /tmp/l400-sfdisk.log >&2
            exit 1
        fi
        info "Particionado completado exitosamente."
        return 0
    fi

    error "No se encontró sfdisk para particionar automáticamente."
    error "Configura AUTO_PARTITION=0 y pasa ROOT_PART / EFI_PART ya creadas."
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
    local mkfs_fat_log="/tmp/l400-mkfs-fat.log"

    info "Formateando partición EFI ${EFI_PART}..."
    if [ ! -b "${EFI_PART}" ]; then
        error "La partición EFI ${EFI_PART} no existe o no es un dispositivo de bloque."
        exit 1
    fi

    if have_cmd mkfs.fat; then
        if ! mkfs.fat -F 32 -n "${EFI_LABEL}" "${EFI_PART}" >"${mkfs_fat_log}" 2>&1; then
            cat "${mkfs_fat_log}" >&2 || true
            error "mkfs.fat falló sobre ${EFI_PART}"
            exit 1
        fi
    else
        if ! mkdosfs -F 32 -n "${EFI_LABEL}" "${EFI_PART}" >"${mkfs_fat_log}" 2>&1; then
            cat "${mkfs_fat_log}" >&2 || true
            error "mkdosfs falló sobre ${EFI_PART}"
            exit 1
        fi
    fi

    if [ -s "${mkfs_fat_log}" ]; then
        grep -v 'cannot initialize conversion from codepage .* invalid argument' \
            "${mkfs_fat_log}" >&2 || true
    fi

    info "Formateando partición root ${ROOT_PART}..."
    if [ ! -b "${ROOT_PART}" ]; then
        error "La partición root ${ROOT_PART} no existe o no es un dispositivo de bloque."
        exit 1
    fi

    if have_cmd mkfs.ext4; then
        if ! mkfs.ext4 -F -L "${ROOT_LABEL}" "${ROOT_PART}" 2>/tmp/l400-mkfs-ext4.log; then
            error "mkfs.ext4 falló sobre ${ROOT_PART}"
            cat /tmp/l400-mkfs-ext4.log >&2
            exit 1
        fi
    else
        if ! mke2fs -t ext4 -F -L "${ROOT_LABEL}" "${ROOT_PART}" 2>/tmp/l400-mkfs-ext4.log; then
            error "mke2fs falló sobre ${ROOT_PART}"
            cat /tmp/l400-mkfs-ext4.log >&2
            exit 1
        fi
    fi
    info "Formateo completado exitosamente."
}

mount_target() {
    info "Montando particiones..."

    # Load necessary kernel modules
    modprobe vfat 2>/dev/null || true
    modprobe fat 2>/dev/null || true
    modprobe nls_cp437 2>/dev/null || true
    modprobe nls_cp850 2>/dev/null || true
    modprobe nls_ascii 2>/dev/null || true
    modprobe nls_utf8 2>/dev/null || true

    mkdir -p "${TARGET_MNT}"

    # Mount root partition
    info "Montando partición root ${ROOT_PART} en ${TARGET_MNT}..."
    if ! mount "${ROOT_PART}" "${TARGET_MNT}" 2>/tmp/l400-mount-root.log; then
        error "No se pudo montar la partición root ${ROOT_PART}"
        cat /tmp/l400-mount-root.log >&2 || true
        log_mount_debug "${ROOT_PART}" "${TARGET_MNT}"
        exit 1
    fi

    mkdir -p "${TARGET_MNT}/boot/efi"

    # Mount EFI partition
    info "Montando partición EFI ${EFI_PART} en ${TARGET_MNT}/boot/efi..."
    if mount_efi_partition; then
        info "Particiones montadas exitosamente."
        return 0
    fi

    error "No se pudo montar la partición EFI ${EFI_PART} en ${TARGET_MNT}/boot/efi"
    cat /tmp/l400-mount-efi.log >&2 || true
    log_mount_debug "${EFI_PART}" "${TARGET_MNT}/boot/efi"
    error "Verifica que el sistema de archivos EFI sea válido y no esté corrupto."
    exit 1
}

copy_rootfs() {
    info "Copiando rootfs a ${TARGET_MNT}..."

    if [ ! -d "${TARGET_MNT}" ] || ! mountpoint -q "${TARGET_MNT}" 2>/dev/null; then
        error "El punto de montaje ${TARGET_MNT} no está disponible o no está montado."
        exit 1
    fi

    # Check available space
    local available_kb
    local required_kb=2000000 # ~2GB minimum
    available_kb=$(df -k "${TARGET_MNT}" 2>/dev/null | awk 'NR==2 {print $4}' || echo "0")
    if [ "${available_kb}" -lt "${required_kb}" ]; then
        error "Espacio insuficiente en ${TARGET_MNT}. Disponible: ${available_kb}KB, Requerido: ${required_kb}KB"
        exit 1
    fi

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
    ) | tar -xpf - -C "${TARGET_MNT}" 2>/tmp/l400-copy-rootfs.log

    if [ $? -ne 0 ]; then
        error "Error al copiar rootfs a ${TARGET_MNT}"
        cat /tmp/l400-copy-rootfs.log >&2 || true
        exit 1
    fi

    info "Rootfs copiado exitosamente."
}

bootstrap_l400_root() {
    local bootstrap_bin=""

    mkdir -p "${TARGET_MNT}/l400"

    if command -v l400-bootstrap >/dev/null 2>&1; then
        bootstrap_bin="$(command -v l400-bootstrap)"
    elif [ -x /opt/l400/bin/l400-bootstrap ]; then
        bootstrap_bin="/opt/l400/bin/l400-bootstrap"
    elif [ -x "${TARGET_MNT}/usr/local/bin/l400-bootstrap" ]; then
        bootstrap_bin="${TARGET_MNT}/usr/local/bin/l400-bootstrap"
    elif [ -x "${TARGET_MNT}/opt/l400/bin/l400-bootstrap" ]; then
        bootstrap_bin="${TARGET_MNT}/opt/l400/bin/l400-bootstrap"
    fi

    if [ -z "${bootstrap_bin}" ]; then
        echo "WARNING: l400-bootstrap no disponible; /l400 instalado queda sin objetos base." >&2
        return 0
    fi

    if ! L400_ROOT="${TARGET_MNT}/l400" "${bootstrap_bin}" --quiet; then
        echo "WARNING: l400-bootstrap fallo para ${TARGET_MNT}/l400; continuando instalacion." >&2
    fi

    register_install_metadata
}

register_install_metadata() {
    local root="${TARGET_MNT}/l400"
    mkdir -p "${root}"

    # Register installed version
    if [ -f "/VERSION" ]; then
        cp "/VERSION" "${root}/VERSION" 2>/dev/null || true
    else
        echo "0.2.0" > "${root}/VERSION"
    fi

    # Register build ID if available
    if [ -f "/BUILD_ID" ]; then
        cp "/BUILD_ID" "${root}/BUILD_ID" 2>/dev/null || true
    else
        echo "build-$(date +%Y%m%d%H%M%S)" > "${root}/BUILD_ID"
    fi

    # Register metadata version via xattr if xattr tools available
    if have_cmd setfattr; then
        setfattr -n "user.l400.version" -v "1.0" "${root}" 2>/dev/null || true
        # Detect platform profile
        local profile="unknown"
        if [ -d "/proc" ] && grep -q "xattr=sa" /proc/mounts 2>/dev/null; then
            profile="full"
        elif [ -d "/sys/fs/bpf" ] && [ -d "/proc" ] && grep -q "cgroup" /proc/filesystems 2>/dev/null; then
            profile="degraded"
        else
            profile="dev"
        fi
        setfattr -n "user.l400.profile" -v "${profile}" "${root}" 2>/dev/null || true
    fi

    info "Metadata de instalación registrada en ${root}"
}

install_boot_assets() {
    local iso_boot_dir=""
    local efi_asset=""
    local candidate
    local efi_vendor_dir="LINUX400"

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

    mkdir -p "${TARGET_MNT}/boot" "${TARGET_MNT}/boot/efi/EFI/BOOT" "${TARGET_MNT}/boot/efi/EFI/${efi_vendor_dir}"

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
            mkdir -p "${TARGET_MNT}/boot/efi/EFI/BOOT" "${TARGET_MNT}/boot/efi/EFI/${efi_vendor_dir}"
            cp "${iso_boot_dir}/vmlinuz" "${TARGET_MNT}/boot/efi/EFI/${efi_vendor_dir}/VMLINUZ"
            cp "${iso_boot_dir}/initramfs.img" "${TARGET_MNT}/boot/efi/EFI/${efi_vendor_dir}/INITRD.IMG"
            cp "${efi_asset}" "${TARGET_MNT}/boot/efi/EFI/BOOT/BOOTX64.EFI"
            cp /tmp/l400-grub.cfg "${TARGET_MNT}/boot/efi/EFI/BOOT/grub.cfg"
            ;;
        mtools)
            mmd -D o -i "${EFI_PART}" ::/EFI ::/EFI/BOOT "::/EFI/${efi_vendor_dir}" >/dev/null 2>&1 || true
            mcopy -D o -n -i "${EFI_PART}" "${iso_boot_dir}/vmlinuz" "::/EFI/${efi_vendor_dir}/VMLINUZ"
            mcopy -D o -n -i "${EFI_PART}" "${iso_boot_dir}/initramfs.img" "::/EFI/${efi_vendor_dir}/INITRD.IMG"
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

    rm -rf "${TARGET_MNT}/home/l400" 2>/dev/null || true
    mkdir -p "${TARGET_MNT}/home/qsecofr"
    chown -R 0:0 "${TARGET_MNT}/home/qsecofr" 2>/dev/null || true
    : > "${TARGET_MNT}/etc/motd"
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
    bootstrap_l400_root
    install_boot_assets
    configure_installed_system

    echo "=== Linux/400 instalado ==="
    echo "Disco : ${disk}"
    echo "EFI   : ${EFI_PART}"
    echo "Root  : ${ROOT_PART}"
}

main "$@"
