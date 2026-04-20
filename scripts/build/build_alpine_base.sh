#!/bin/bash
# build_alpine_base.sh - Ensambla el rootfs Alpine base para Linux/400

set -euo pipefail

ALPINE_VERSION="${ALPINE_VERSION:-3.20}"
ARCH="${ARCH:-x86_64}"
L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"
ROOTFS_DIR="${ROOTFS_DIR:-${OUTPUT_DIR}/rootfs-build}"
USERSPACE_DIR="${OUTPUT_DIR}/userspace"
RUNTIME_DIR="${L400_SRC_DIR}/scripts/runtime"
MINIROOT="alpine-minirootfs-${ALPINE_VERSION}.0-${ARCH}.tar.gz"
MINIROOT_URL="https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}/releases/${ARCH}/${MINIROOT}"

copy_binary_with_runtime() {
    local binary="$1"
    local resolved=""
    local dep=""

    [ -e "${binary}" ] || return 0

    resolved="$(readlink -f "${binary}" 2>/dev/null || printf '%s' "${binary}")"
    [ -f "${resolved}" ] || return 0

    mkdir -p "${ROOTFS_DIR}$(dirname "${resolved}")"
    cp -L "${resolved}" "${ROOTFS_DIR}${resolved}"

    if [ "${binary}" != "${resolved}" ]; then
        mkdir -p "${ROOTFS_DIR}$(dirname "${binary}")"
        ln -sf "${resolved}" "${ROOTFS_DIR}${binary}"
    fi

    while IFS= read -r dep; do
        [ -n "${dep}" ] || continue
        [ -f "${dep}" ] || continue
        mkdir -p "${ROOTFS_DIR}$(dirname "${dep}")"
        cp -L "${dep}" "${ROOTFS_DIR}${dep}"
    done <<EOF
$(ldd "${resolved}" 2>/dev/null | awk '
    {
        for (i = 1; i <= NF; ++i) {
            if ($i ~ /^\//) {
                print $i
            }
        }
    }')
EOF
}

download_minrootfs() {
    mkdir -p "${OUTPUT_DIR}"

    if [ -f "${OUTPUT_DIR}/${MINIROOT}" ]; then
        return 0
    fi

    echo ">> Descargando Alpine minirootfs..."
    if command -v curl >/dev/null 2>&1; then
        curl -L -o "${OUTPUT_DIR}/${MINIROOT}" "${MINIROOT_URL}"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "${OUTPUT_DIR}/${MINIROOT}" "${MINIROOT_URL}"
    else
        echo "ERROR: se requiere curl o wget para descargar Alpine." >&2
        exit 1
    fi
}

ensure_userspace() {
    if [ ! -x "${USERSPACE_DIR}/bin/os400-tui" ]; then
        "${L400_SRC_DIR}/scripts/build/build_userspace.sh"
    fi
}

maybe_install_extra_packages() {
    cat > "${ROOTFS_DIR}/etc/apk/repositories" <<EOF
https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}/main
https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}/community
EOF

    if ! command -v apk >/dev/null 2>&1; then
        echo "WARNING: apk no está disponible en el host; el rootfs se arma con minirootfs puro."
        return 0
    fi

    local packages=(
        alpine-base
        bash
        openssh
        tzdata
        util-linux
        e2fsprogs
        dosfstools
        mtools
    )

    echo ">> Instalando paquetes extra con apk del host..."
    apk --root "${ROOTFS_DIR}" --arch "${ARCH}" update
    apk --root "${ROOTFS_DIR}" --arch "${ARCH}" add "${packages[@]}"
}

install_host_disk_tools_fallback() {
    local path_entry
    local tools=(
        /usr/bin/mount
        /usr/bin/umount
        /usr/bin/findmnt
        /usr/sbin/blkid
        /usr/sbin/findfs
        /usr/sbin/sfdisk
        /usr/sbin/mkfs.ext4
        /usr/sbin/mke2fs
        /usr/sbin/mkfs.fat
        /usr/sbin/fsck.fat
        /usr/sbin/mount.vfat
    )

    for path_entry in "${tools[@]}"; do
        copy_binary_with_runtime "${path_entry}"
    done
}

install_host_mtools_fallback() {
    local mtools_root="${OUTPUT_DIR}/host-tools/mtools/extracted"
    local applet=""
    local dep=""

    if [ -x /usr/bin/mtools ]; then
        for applet in /usr/bin/mtools /usr/bin/mcopy /usr/bin/mmd /usr/bin/mdir /usr/bin/mformat /usr/bin/mlabel; do
            copy_binary_with_runtime "${applet}"
        done
    elif [ -x "${mtools_root}/usr/bin/mtools" ]; then
        mkdir -p "${ROOTFS_DIR}/usr/bin"
        cp -L "${mtools_root}/usr/bin/mtools" "${ROOTFS_DIR}/usr/bin/mtools"

        for applet in mcopy mmd mdir mformat mlabel; do
            ln -sf /usr/bin/mtools "${ROOTFS_DIR}/usr/bin/${applet}"
        done

        while IFS= read -r dep; do
            [ -n "${dep}" ] || continue
            [ -f "${dep}" ] || continue
            mkdir -p "${ROOTFS_DIR}$(dirname "${dep}")"
            cp -L "${dep}" "${ROOTFS_DIR}${dep}"
        done <<EOF
$(ldd "${mtools_root}/usr/bin/mtools" 2>/dev/null | awk '
    {
        for (i = 1; i <= NF; ++i) {
            if ($i ~ /^\//) {
                print $i
            }
        }
    }')
EOF
    fi
}

ensure_user_l400() {
    local passwd_file="${ROOTFS_DIR}/etc/passwd"
    local shadow_file="${ROOTFS_DIR}/etc/shadow"
    local group_file="${ROOTFS_DIR}/etc/group"

    mkdir -p "${ROOTFS_DIR}/home/l400"

    grep -q '^l400:' "${group_file}" 2>/dev/null || \
        echo 'l400:x:1000:' >> "${group_file}"

    if ! grep -q '^l400:' "${passwd_file}" 2>/dev/null; then
        echo 'l400:x:1000:1000:Linux/400 User:/home/l400:/bin/sh' >> "${passwd_file}"
    fi

    if ! grep -q '^l400:' "${shadow_file}" 2>/dev/null; then
        # Password por defecto: l400
        echo 'l400:$5$Tb0gqvL3IrC3D4Qx$4xrkxXHqP5cW5M6E1x2hMUPi8JjGCVr8K8Qm7N8Hj7/:20000:0:99999:7:::' >> "${shadow_file}"
    fi
}

install_userspace() {
    echo ">> Instalando userspace Linux/400..."

    mkdir -p \
        "${ROOTFS_DIR}/opt/l400/bin" \
        "${ROOTFS_DIR}/opt/l400/hooks" \
        "${ROOTFS_DIR}/opt/l400/scripts" \
        "${ROOTFS_DIR}/lib/l400" \
        "${ROOTFS_DIR}/usr/local/bin" \
        "${ROOTFS_DIR}/usr/local/sbin" \
        "${ROOTFS_DIR}/etc/profile.d" \
        "${ROOTFS_DIR}/etc" \
        "${ROOTFS_DIR}/var/lib/l400" \
        "${ROOTFS_DIR}/l400"

    cp "${USERSPACE_DIR}/bin/os400-tui" "${ROOTFS_DIR}/opt/l400/bin/"
    cp "${USERSPACE_DIR}/bin/l400-loader" "${ROOTFS_DIR}/opt/l400/bin/"
    cp "${USERSPACE_DIR}/bin/c400c" "${ROOTFS_DIR}/opt/l400/bin/"
    cp "${USERSPACE_DIR}/bin/clc" "${ROOTFS_DIR}/opt/l400/bin/"
    cp "${USERSPACE_DIR}/lib/libl400.a" "${ROOTFS_DIR}/lib/l400/"
    if [ -f "${USERSPACE_DIR}/lib/libl400.so" ]; then
        cp "${USERSPACE_DIR}/lib/libl400.so" "${ROOTFS_DIR}/lib/l400/"
    fi

    if [ -f "${USERSPACE_DIR}/hooks/l400-ebpf" ]; then
        cp "${USERSPACE_DIR}/hooks/l400-ebpf" "${ROOTFS_DIR}/opt/l400/hooks/"
    fi

    cp "${RUNTIME_DIR}/l400-session.sh" "${ROOTFS_DIR}/usr/local/bin/l400-session"
    cp "${RUNTIME_DIR}/l400-console-autologin.sh" "${ROOTFS_DIR}/usr/local/bin/l400-console-autologin"
    cp "${RUNTIME_DIR}/l400-support-report.sh" "${ROOTFS_DIR}/usr/local/bin/l400-support-report"
    cp "${RUNTIME_DIR}/install_linux400.sh" "${ROOTFS_DIR}/usr/local/sbin/install-linux400"

    cp -r "${L400_SRC_DIR}/scripts/"* "${ROOTFS_DIR}/opt/l400/scripts/" 2>/dev/null || true

    chmod +x \
        "${ROOTFS_DIR}/usr/local/bin/l400-session" \
        "${ROOTFS_DIR}/usr/local/bin/l400-console-autologin" \
        "${ROOTFS_DIR}/usr/local/bin/l400-support-report" \
        "${ROOTFS_DIR}/usr/local/sbin/install-linux400"

    ln -sf /opt/l400/bin/os400-tui "${ROOTFS_DIR}/usr/local/bin/os400-tui"
    ln -sf /opt/l400/bin/l400-loader "${ROOTFS_DIR}/usr/local/bin/l400-loader"
    ln -sf /opt/l400/bin/c400c "${ROOTFS_DIR}/usr/local/bin/c400c"
    ln -sf /opt/l400/bin/clc "${ROOTFS_DIR}/usr/local/bin/clc"
}

configure_shell_environment() {
    echo ">> Configurando entorno Linux/400..."

    cat > "${ROOTFS_DIR}/etc/profile.d/l400-env.sh" <<'EOF'
export PATH="/usr/local/sbin:/usr/local/bin:/opt/l400/bin:$PATH"
export L400_ROOT="/l400"
export L400_LIB_PATH="/lib/l400"
export LIBRARY_PATH="/lib/l400${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LD_LIBRARY_PATH="/lib/l400${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
EOF

    cat > "${ROOTFS_DIR}/home/l400/.profile" <<'EOF'
if [ -f /etc/profile ]; then
    . /etc/profile
fi

exec /usr/local/bin/l400-session
EOF

    cat > "${ROOTFS_DIR}/etc/motd" <<'EOF'
Linux/400 Live Environment
- Usuario por defecto: l400 / l400
- Instalación a disco: install-linux400 /dev/sdX
EOF

    echo "linux400" > "${ROOTFS_DIR}/etc/hostname"
}

configure_console_login() {
    echo ">> Configurando autologin live..."

    if [ -x "${ROOTFS_DIR}/sbin/openrc" ] && [ -f "${ROOTFS_DIR}/etc/inittab" ]; then
        sed -i 's#^tty1::respawn:.*#tty1::respawn:/sbin/getty -n -l /usr/local/bin/l400-console-autologin 115200 tty1 linux#' \
            "${ROOTFS_DIR}/etc/inittab"
        if grep -q '^ttyS0::respawn:' "${ROOTFS_DIR}/etc/inittab"; then
            sed -i 's#^ttyS0::respawn:.*#ttyS0::respawn:/sbin/getty -L -n -l /usr/local/bin/l400-console-autologin 115200 ttyS0 vt100#' \
                "${ROOTFS_DIR}/etc/inittab"
        else
            cat >> "${ROOTFS_DIR}/etc/inittab" <<'EOF'
ttyS0::respawn:/sbin/getty -L -n -l /usr/local/bin/l400-console-autologin 115200 ttyS0 vt100
EOF
        fi
    else
        cat > "${ROOTFS_DIR}/etc/inittab" <<'EOF'
::respawn:/sbin/getty -n -l /usr/local/bin/l400-console-autologin 115200 tty1 linux
ttyS0::respawn:/sbin/getty -L -n -l /usr/local/bin/l400-console-autologin 115200 ttyS0 vt100
::respawn:/sbin/getty 115200 tty2
::respawn:/sbin/getty 115200 tty3
::ctrlaltdel:/sbin/reboot
EOF
    fi
}

main() {
    echo "=== Construyendo rootfs Alpine para Linux/400 ==="
    echo "Versión Alpine: ${ALPINE_VERSION}"
    echo "Arquitectura   : ${ARCH}"

    ensure_userspace
    download_minrootfs

    rm -rf "${ROOTFS_DIR}"
    mkdir -p "${ROOTFS_DIR}"

    echo ">> Extrayendo rootfs base..."
    tar -xzf "${OUTPUT_DIR}/${MINIROOT}" -C "${ROOTFS_DIR}"

    maybe_install_extra_packages
    install_host_disk_tools_fallback
    install_host_mtools_fallback
    ensure_user_l400
    install_userspace
    configure_shell_environment
    configure_console_login

    echo "=== Rootfs Linux/400 listo ==="
    echo "Ubicación: ${ROOTFS_DIR}"
}

main "$@"
