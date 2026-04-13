#!/bin/bash
# build_iso.sh - Genera una ISO live/install de Linux/400 usando GRUB + squashfs

set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"
VERSION="${VERSION:-1.0.0}"
KERNEL_VERSION="${KERNEL_VERSION:-$(uname -r)}"
ISO_NAME="${ISO_NAME:-linux400-${VERSION}}"
ISO_NAME="${ISO_NAME%.iso}"
WORK_DIR="${OUTPUT_DIR}/iso_work"
ISO_ROOT="${WORK_DIR}/iso_root"
LIVE_DIR="${ISO_ROOT}/live"
BOOT_DIR="${ISO_ROOT}/boot"
GRUB_DIR="${BOOT_DIR}/grub"
ROOTFS_DIR="${ROOTFS_DIR:-${OUTPUT_DIR}/rootfs-build}"
INITRAMFS_IMG="${OUTPUT_DIR}/initramfs-${KERNEL_VERSION}-l400.img"
SQUASHFS_IMG="${LIVE_DIR}/rootfs.squashfs"
INSTALL_EFI="${OUTPUT_DIR}/BOOTX64.EFI"
HOST_TOOLS_DIR="${OUTPUT_DIR}/host-tools"

find_first_existing() {
    local candidate
    for candidate in "$@"; do
        if [ -f "${candidate}" ]; then
            echo "${candidate}"
            return 0
        fi
    done
    return 1
}

acquire_kernel() {
    local kernel_src=""
    local kernel_pkg_dir="${OUTPUT_DIR}/kernel_pkg"
    local kernel_pkg="linux-image-${KERNEL_VERSION}"

    kernel_src="$(find_first_existing \
        "${OUTPUT_DIR}/vmlinuz" \
        "/boot/vmlinuz-${KERNEL_VERSION}" \
        "/boot/vmlinuz" \
        "/boot/vmlinuz-linux" 2>/dev/null || true)"

    if [ -n "${kernel_src}" ] && [ -r "${kernel_src}" ]; then
        echo "${kernel_src}"
        return 0
    fi

    if command -v apt-get >/dev/null 2>&1 && command -v dpkg-deb >/dev/null 2>&1; then
        mkdir -p "${kernel_pkg_dir}"
        (
            cd "${kernel_pkg_dir}"
            if ! ls "${kernel_pkg}"_*.deb >/dev/null 2>&1; then
                apt-get download "${kernel_pkg}" >/dev/null
            fi
            rm -rf extracted
            dpkg-deb -x "${kernel_pkg}"_*.deb extracted
        )

        kernel_src="$(find_first_existing "${kernel_pkg_dir}/extracted/boot/vmlinuz-${KERNEL_VERSION}")"
        if [ -n "${kernel_src}" ] && [ -r "${kernel_src}" ]; then
            echo "${kernel_src}"
            return 0
        fi
    fi

    return 1
}

ensure_command_from_apt() {
    local command_name="$1"
    local package_name="$2"
    local package_dir="${HOST_TOOLS_DIR}/${package_name}"
    local extracted_dir="${package_dir}/extracted"

    if command -v "${command_name}" >/dev/null 2>&1; then
        return 0
    fi

    if ! command -v apt-get >/dev/null 2>&1 || ! command -v dpkg-deb >/dev/null 2>&1; then
        echo "ERROR: falta ${command_name} y no se puede descargar ${package_name}." >&2
        exit 1
    fi

    mkdir -p "${package_dir}"
    (
        cd "${package_dir}"
        if ! ls "${package_name}"_*.deb >/dev/null 2>&1; then
            apt-get download "${package_name}" >/dev/null
        fi
        rm -rf extracted
        dpkg-deb -x "${package_name}"_*.deb extracted
    )

    PATH="${extracted_dir}/usr/bin:${extracted_dir}/bin:${PATH}"
    export PATH

    command -v "${command_name}" >/dev/null 2>&1 || {
        echo "ERROR: no se pudo preparar ${command_name} desde ${package_name}." >&2
        exit 1
    }
}

ensure_inputs() {
    if [ ! -d "${ROOTFS_DIR}" ]; then
        "${L400_SRC_DIR}/scripts/build/build_alpine_base.sh"
    fi

    if [ ! -f "${INITRAMFS_IMG}" ]; then
        "${L400_SRC_DIR}/scripts/build/build_initramfs.sh"
    fi

    command -v mksquashfs >/dev/null 2>&1 || {
        echo "ERROR: se requiere mksquashfs (squashfs-tools)." >&2
        exit 1
    }

    command -v grub-mkrescue >/dev/null 2>&1 || {
        echo "ERROR: se requiere grub-mkrescue." >&2
        exit 1
    }

    command -v grub-mkstandalone >/dev/null 2>&1 || {
        echo "ERROR: se requiere grub-mkstandalone." >&2
        exit 1
    }

    ensure_command_from_apt mformat mtools
    ensure_command_from_apt mcopy mtools
}

stage_tree() {
    rm -rf "${WORK_DIR}"
    mkdir -p "${GRUB_DIR}" "${LIVE_DIR}"

    local kernel_src
    kernel_src="$(acquire_kernel)" || {
        echo "ERROR: no se encontró kernel para la ISO." >&2
        exit 1
    }

    echo ">> Copiando kernel desde ${kernel_src}..."
    cp "${kernel_src}" "${BOOT_DIR}/vmlinuz"
    if [ "${kernel_src}" != "${OUTPUT_DIR}/vmlinuz" ]; then
        cp "${kernel_src}" "${OUTPUT_DIR}/vmlinuz"
    fi

    echo ">> Copiando initramfs..."
    cp "${INITRAMFS_IMG}" "${BOOT_DIR}/initramfs.img"

    echo ">> Generando squashfs live..."
    mksquashfs "${ROOTFS_DIR}" "${SQUASHFS_IMG}" -noappend -comp xz -all-root >/dev/null
}

write_grub_cfg() {
    cat > "${GRUB_DIR}/grub.cfg" <<'EOF'
set default=0
set timeout=5

menuentry "Linux/400 Live" {
    linux /boot/vmlinuz quiet console=tty0 console=ttyS0,115200 l400.live=1
    initrd /boot/initramfs.img
}

menuentry "Linux/400 Install" {
    linux /boot/vmlinuz quiet console=tty0 console=ttyS0,115200 l400.install=1
    initrd /boot/initramfs.img
}

menuentry "Linux/400 Rescue" {
    linux /boot/vmlinuz console=tty0 console=ttyS0,115200 l400.rescue=1
    initrd /boot/initramfs.img
}
EOF
}

build_installer_efi() {
    local tmp_cfg="${WORK_DIR}/grub-installed.cfg"

    cat > "${tmp_cfg}" <<'EOF'
set default=0
set timeout=3

menuentry "Linux/400" {
    linux /EFI/Linux400/vmlinuz root=LABEL=linux400-root rw quiet console=tty0 console=ttyS0,115200 l400.installed=1
    initrd /EFI/Linux400/initramfs.img
}
EOF

    echo ">> Generando BOOTX64.EFI para instalación UEFI..."
    grub-mkstandalone \
        -O x86_64-efi \
        -o "${INSTALL_EFI}" \
        "boot/grub/grub.cfg=${tmp_cfg}" >/dev/null

    cp "${INSTALL_EFI}" "${LIVE_DIR}/BOOTX64.EFI"
}

build_iso() {
    echo ">> Generando ISO híbrida con grub-mkrescue..."
    grub-mkrescue -o "${OUTPUT_DIR}/${ISO_NAME}.iso" "${ISO_ROOT}" >/dev/null
}

main() {
    echo "=== Generando ISO Linux/400 v${VERSION} ==="
    ensure_inputs
    stage_tree
    write_grub_cfg
    build_installer_efi
    build_iso

    echo "=== ISO lista ==="
    echo "ISO      : ${OUTPUT_DIR}/${ISO_NAME}.iso"
    echo "EFI UEFI : ${INSTALL_EFI}"
    ls -lh "${OUTPUT_DIR}/${ISO_NAME}.iso" "${INSTALL_EFI}"
}

main "$@"
