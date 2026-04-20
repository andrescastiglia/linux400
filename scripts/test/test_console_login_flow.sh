#!/bin/bash
# test_console_login_flow.sh - Verifica que live/install/installed usen el flujo de login esperado

set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
ROOTFS_DIR="$(mktemp -d)"

cleanup() {
    rm -rf "${ROOTFS_DIR}"
}
trap cleanup EXIT

echo "=== Verificando flujo de login Linux/400 ==="

for required in \
    "${L400_SRC_DIR}/output/userspace/bin/os400-tui" \
    "${L400_SRC_DIR}/output/userspace/bin/l400-loader" \
    "${L400_SRC_DIR}/output/userspace/bin/c400c" \
    "${L400_SRC_DIR}/output/userspace/bin/clc" \
    "${L400_SRC_DIR}/output/userspace/bin/l400cmd" \
    "${L400_SRC_DIR}/output/userspace/lib/libl400.a"; do
    if [ ! -f "${required}" ]; then
        echo "ERROR: falta artefacto requerido: ${required}" >&2
        echo "Ejecute scripts/build/build_userspace.sh antes de esta prueba." >&2
        exit 1
    fi
done

ROOTFS_DIR="${ROOTFS_DIR}" "${L400_SRC_DIR}/scripts/build/build_alpine_base.sh" >/tmp/l400-build-rootfs.log

grep -q '^qsecofr:x:1000:1000:Linux/400 Security Officer:/home/qsecofr:/usr/local/bin/l400-session$' \
    "${ROOTFS_DIR}/etc/passwd"
grep -q '^qsecofr:' "${ROOTFS_DIR}/etc/shadow"
grep -q 'l400-console-autologin' "${ROOTFS_DIR}/etc/inittab"
grep -q 'exec /usr/local/bin/l400-installer' "${ROOTFS_DIR}/usr/local/bin/l400-console-autologin"
grep -q '/dev/ttyS\*|/dev/ttyAMA\*|/dev/hvc\*)' "${ROOTFS_DIR}/usr/local/bin/l400-console-autologin"
grep -q 'login_user="root"' "${ROOTFS_DIR}/usr/local/bin/l400-console-autologin"
grep -q 'exec /usr/local/bin/l400-session' "${ROOTFS_DIR}/home/qsecofr/.profile"
grep -q 'exec "${fallback_shell}" "\$@"' "${ROOTFS_DIR}/usr/local/bin/l400-session"
test -x "${ROOTFS_DIR}/usr/local/bin/l400-installer"

echo "PASS: la ISO/live queda configurada para login TUI en live y para instalador en modo install."
