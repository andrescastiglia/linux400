#!/bin/bash
# build_initramfs.sh - Crea un initramfs live/install con switch_root para Linux/400

set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"
INITRAMFS_DIR="${OUTPUT_DIR}/initramfs"
KERNEL_VERSION="${KERNEL_VERSION:-$(uname -r)}"
L400_INIT="${INITRAMFS_DIR}/init"
USERSPACE_DIR="${OUTPUT_DIR}/userspace"
ROOTFS_DIR="${ROOTFS_DIR:-${OUTPUT_DIR}/rootfs-build}"

ensure_userspace() {
    if [ ! -x "${USERSPACE_DIR}/bin/l400-loader" ]; then
        "${L400_SRC_DIR}/scripts/build/build_userspace.sh"
    fi
}

copy_busybox_applets() {
    local applets=(
        sh
        ash
        mount
        umount
        mkdir
        mdev
        modprobe
        insmod
        cat
        cut
        grep
        sleep
        switch_root
        losetup
        find
        findfs
        blkid
        ln
        chown
        dmesg
        mountpoint
        tty
        zcat
    )

    for applet in "${applets[@]}"; do
        ln -sf busybox "${INITRAMFS_DIR}/bin/${applet}"
    done
}

prepare_tree() {
    rm -rf "${INITRAMFS_DIR}"
    mkdir -p "${INITRAMFS_DIR}"/{bin,sbin,etc,proc,sys,dev,run,tmp,mnt/media,mnt/root-ro,mnt/root-rw,mnt/newroot,l400/hooks,live}
    chmod 1777 "${INITRAMFS_DIR}/tmp"
}

copy_modules() {
    echo ">> Copiando módulos del kernel..."
    mkdir -p "${INITRAMFS_DIR}/lib"

    decompress_module() {
        local module_name="$1"
        local module_src=""
        local module_dst=""

        module_src="$(modinfo -k "${KERNEL_VERSION}" -n "${module_name}" 2>/dev/null || true)"
        [ -n "${module_src}" ] || return 0
        [ -f "${module_src}" ] || return 0

        case "${module_src}" in
            *.zst)
                if command -v zstd >/dev/null 2>&1; then
                    module_dst="${INITRAMFS_DIR}${module_src%.zst}"
                    mkdir -p "$(dirname "${module_dst}")"
                    zstd -d -c "${module_src}" > "${module_dst}"
                fi
                ;;
        esac
    }

    if [ -d "/lib/modules/${KERNEL_VERSION}" ]; then
        cp -a "/lib/modules/${KERNEL_VERSION}" "${INITRAMFS_DIR}/lib/"
        decompress_module overlay
        decompress_module vfat
        decompress_module fat
        decompress_module nls_cp437
        decompress_module nls_ascii
    else
        echo "WARNING: no se encontró /lib/modules/${KERNEL_VERSION}; se dependerá de módulos built-in."
    fi
}

copy_payloads() {
    echo ">> Copiando busybox y payloads..."
    cp /usr/bin/busybox "${INITRAMFS_DIR}/bin/busybox"
    copy_busybox_applets

    cp "${USERSPACE_DIR}/bin/l400-loader" "${INITRAMFS_DIR}/bin/l400-loader"
    if [ -f "${USERSPACE_DIR}/hooks/l400-ebpf" ]; then
        cp "${USERSPACE_DIR}/hooks/l400-ebpf" "${INITRAMFS_DIR}/l400/hooks/l400-ebpf"
    fi

    chmod +x "${INITRAMFS_DIR}/bin/busybox" "${INITRAMFS_DIR}/bin/l400-loader"
}

embed_live_rootfs() {
    if [ ! -d "${ROOTFS_DIR}" ]; then
        return 0
    fi

    if ! command -v mksquashfs >/dev/null 2>&1; then
        echo "WARNING: mksquashfs no disponible; initramfs se construirá sin rootfs embebido."
        return 0
    fi

    echo ">> Embebiendo rootfs.squashfs en initramfs..."
    mksquashfs "${ROOTFS_DIR}" "${INITRAMFS_DIR}/live/rootfs.squashfs" -noappend -comp xz -all-root >/dev/null
}

create_devices() {
    echo ">> Configurando dispositivos mínimos..."
    mknod "${INITRAMFS_DIR}/dev/console" c 5 1 2>/dev/null || true
    mknod "${INITRAMFS_DIR}/dev/null" c 1 3 2>/dev/null || true
    mknod "${INITRAMFS_DIR}/dev/tty" c 5 0 2>/dev/null || true
    mknod "${INITRAMFS_DIR}/dev/tty0" c 4 0 2>/dev/null || true
    mknod "${INITRAMFS_DIR}/dev/loop0" b 7 0 2>/dev/null || true
}

write_init() {
    cat > "${L400_INIT}" <<'EOF'
#!/bin/busybox sh

set -eu

export PATH=/bin:/sbin
export LD_LIBRARY_PATH=/lib

log() {
    echo "[initramfs] $*"
}

panic_shell() {
    echo "[initramfs] ERROR: $*" >&2
    exec /bin/sh
}

mount_early_fs() {
    mount -t proc proc /proc
    mount -t sysfs sysfs /sys
    mount -t devtmpfs devtmpfs /dev 2>/dev/null || mount -t tmpfs tmpfs /dev
    mkdir -p /dev/pts /run
    mount -t devpts devpts /dev/pts 2>/dev/null || true
    mount -t tmpfs tmpfs /run
    mkdir -p /run/l400 /run/l400/media
    /bin/mdev -s
}

load_kernel_modules() {
    load_builtin_module() {
        local module_name="$1"
        local module_path=""

        for module_path in $(find /lib/modules -name "${module_name}.ko" 2>/dev/null); do
            [ -f "${module_path}" ] || continue
            insmod "${module_path}" 2>/dev/null || true
            return 0
        done
    }

    local kernel_version=""
    local overlay_module=""

    kernel_version="$(uname -r 2>/dev/null || true)"
    if [ -n "${kernel_version}" ] && [ -f "/lib/modules/${kernel_version}/kernel/fs/overlayfs/overlay.ko" ]; then
        overlay_module="/lib/modules/${kernel_version}/kernel/fs/overlayfs/overlay.ko"
    else
        for overlay_module in $(find /lib/modules -name overlay.ko 2>/dev/null); do
            [ -f "${overlay_module}" ] && break
        done
    fi

    if [ -n "${overlay_module}" ] && [ -f "${overlay_module}" ]; then
        insmod "${overlay_module}" 2>/dev/null || true
    fi

    for module in fat nls_cp437 nls_ascii vfat; do
        load_builtin_module "${module}"
    done

    for module in loop squashfs overlay isofs udf fat nls_cp437 nls_ascii vfat; do
        modprobe "${module}" 2>/dev/null || true
    done
}

load_l400_ebpf() {
    if [ -x /bin/l400-loader ] && [ -f /l400/hooks/l400-ebpf ]; then
        log "Cargando BPF LSM..."
        L400_BPF_PATH=/l400/hooks/l400-ebpf /bin/l400-loader >/run/l400-loader.log 2>&1 &
        echo "$!" > /run/l400-loader.pid
    else
        log "BPF LSM no disponible en initramfs; continuando sin loader."
    fi
}

find_live_media() {
    local dev
    mkdir -p /mnt/media

    for dev in \
        /dev/sr0 \
        /dev/vd* \
        /dev/xvd* \
        /dev/sd* \
        /dev/nvme*n1p* \
        /dev/mmcblk*p*; do
        [ -b "${dev}" ] || continue

        if mount -t iso9660 -o ro "${dev}" /mnt/media 2>/dev/null || \
            mount -o ro "${dev}" /mnt/media 2>/dev/null; then
                if [ -f /mnt/media/live/rootfs.squashfs ]; then
                    echo "${dev}"
                    return 0
                fi
                umount /mnt/media 2>/dev/null || true
            fi
    done

    return 1
}

get_cmdline_arg() {
    local key="$1"
    local token

    for token in $(cat /proc/cmdline); do
        case "${token}" in
            "${key}"=*)
                echo "${token#*=}"
                return 0
                ;;
        esac
    done

    return 1
}

mount_installed_root() {
    local boot_mode="installed"
    local root_spec
    local root_dev=""

    root_spec="$(get_cmdline_arg root || true)"
    if [ -z "${root_spec}" ]; then
        root_spec="LABEL=linux400-root"
    fi

    case "${root_spec}" in
        /dev/*)
            root_dev="${root_spec}"
            ;;
        LABEL=*|UUID=*)
            root_dev="$(findfs "${root_spec}" 2>/dev/null || true)"
            ;;
    esac

    [ -n "${root_dev}" ] || panic_shell "No se pudo resolver root=${root_spec}."
    [ -b "${root_dev}" ] || panic_shell "El dispositivo raíz no existe: ${root_dev}"

    mount "${root_dev}" /mnt/newroot || panic_shell "No se pudo montar la raíz instalada ${root_dev}."

    mkdir -p /mnt/newroot/proc /mnt/newroot/sys /mnt/newroot/dev /mnt/newroot/run /mnt/newroot/home/l400 /mnt/newroot/l400
    mount -t tmpfs -o mode=0775 tmpfs /mnt/newroot/l400 2>/dev/null || true
    chown 1000:1000 /mnt/newroot/home/l400 2>/dev/null || true
    chown 1000:1000 /mnt/newroot/l400 2>/dev/null || true

    echo "${boot_mode}" > /run/l400/boot-mode

    mount --move /proc /mnt/newroot/proc
    mount --move /sys /mnt/newroot/sys
    mount --move /dev /mnt/newroot/dev
    mount --move /run /mnt/newroot/run
}

mount_live_root() {
    local boot_mode="live"

    case " $(cat /proc/cmdline) " in
        *" l400.install=1 "*) boot_mode="install" ;;
        *" l400.rescue=1 "*) boot_mode="rescue" ;;
    esac

    local media_dev=""
    local attempt

    if [ -f /live/rootfs.squashfs ]; then
        log "Usando rootfs live embebido en initramfs."
        mount -t squashfs -o loop /live/rootfs.squashfs /mnt/root-ro || \
            panic_shell "No se pudo montar el rootfs embebido."

        media_dev="$(find_live_media || true)"
        if [ -n "${media_dev}" ]; then
            log "Medio ISO detectado adicionalmente: ${media_dev}"
            mount --move /mnt/media /run/l400/media
        else
            log "No se pudo montar el medio ISO; el instalador dependerá de assets locales."
        fi
    else
        for attempt in 1 2 3 4 5 6 7 8 9 10; do
            media_dev="$(find_live_media || true)"
            if [ -n "${media_dev}" ]; then
                break
            fi
            sleep 1
            /bin/mdev -s
        done

        [ -n "${media_dev}" ] || panic_shell "No se encontró el medio live con /live/rootfs.squashfs."
        log "Medio live detectado: ${media_dev}"

        mount -t squashfs -o loop /mnt/media/live/rootfs.squashfs /mnt/root-ro || \
            panic_shell "No se pudo montar rootfs.squashfs."

        mount --move /mnt/media /run/l400/media
    fi

    mount -t tmpfs tmpfs /mnt/root-rw
    mkdir -p /mnt/root-rw/upper /mnt/root-rw/work /mnt/newroot

    if ! mount -t overlay overlay \
        -o lowerdir=/mnt/root-ro,upperdir=/mnt/root-rw/upper,workdir=/mnt/root-rw/work \
        /mnt/newroot; then
        log "Overlay no disponible; continuando con rootfs live en modo solo lectura."
        mount --bind /mnt/root-ro /mnt/newroot || panic_shell "No se pudo preparar el rootfs live sin overlay."
    fi

    mkdir -p /mnt/newroot/proc /mnt/newroot/sys /mnt/newroot/dev /mnt/newroot/run /mnt/newroot/home/l400 /mnt/newroot/l400
    mount -t tmpfs -o mode=0775 tmpfs /mnt/newroot/l400
    chown 1000:1000 /mnt/newroot/home/l400 2>/dev/null || true
    chown 1000:1000 /mnt/newroot/l400 2>/dev/null || true

    echo "${boot_mode}" > /run/l400/boot-mode
    mount --move /proc /mnt/newroot/proc
    mount --move /sys /mnt/newroot/sys
    mount --move /dev /mnt/newroot/dev
    mount --move /run /mnt/newroot/run
}

main() {
    log "=== Linux/400 initramfs ==="
    mount_early_fs
    load_kernel_modules
    load_l400_ebpf

    case " $(cat /proc/cmdline) " in
        *" l400.installed=1 "*)
            mount_installed_root
            ;;
        *)
            mount_live_root
            ;;
    esac

    exec switch_root /mnt/newroot /sbin/init
}

main "$@"
EOF

    chmod +x "${L400_INIT}"
}

pack_initramfs() {
    echo ">> Empaquetando initramfs..."
    (
        cd "${INITRAMFS_DIR}"
        find . -print0 | cpio --null -ov --format=newc > "${OUTPUT_DIR}/initramfs.cpio"
    ) >/dev/null 2>&1

    gzip -c "${OUTPUT_DIR}/initramfs.cpio" > "${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-l400.img"
    rm -f "${OUTPUT_DIR}/initramfs.cpio"
}

main() {
    echo "=== Construyendo initramfs Linux/400 ==="
    ensure_userspace
    prepare_tree
    copy_modules
    copy_payloads
    embed_live_rootfs
    create_devices
    write_init
    pack_initramfs

    echo "=== Initramfs listo ==="
    echo "Ubicación: ${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-l400.img"
}

main "$@"
