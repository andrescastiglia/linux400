#!/bin/bash
# test_e2e_install_qemu.sh - Valida instalación UEFI completa de Linux/400 sobre qcow2

set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output_e2e_qemu}"
ISO_NAME="${ISO_NAME:-linux400-e2e}"
ISO_PATH="${ISO_PATH:-${OUTPUT_DIR}/${ISO_NAME}.iso}"
DISK_PATH="${DISK_PATH:-${OUTPUT_DIR}/linux400-test.qcow2}"
OVMF_CODE="${OVMF_CODE:-/usr/share/OVMF/OVMF_CODE_4M.fd}"
OVMF_VARS_TEMPLATE="${OVMF_VARS_TEMPLATE:-/usr/share/OVMF/OVMF_VARS_4M.fd}"
OVMF_VARS="${OUTPUT_DIR}/OVMF_VARS_4M.fd"
DISK_SIZE="${DISK_SIZE:-16G}"
QEMU_MEM_MB="${QEMU_MEM_MB:-2048}"
QEMU_CPUS="${QEMU_CPUS:-2}"
LIVE_LOG="${OUTPUT_DIR}/qemu-live-install.log"
INSTALLED_LOG="${OUTPUT_DIR}/qemu-installed.log"

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "ERROR: falta el comando requerido: $1" >&2
        exit 1
    }
}

ensure_inputs() {
    require_cmd expect
    require_cmd qemu-img
    require_cmd qemu-system-x86_64

    [ -f "${OVMF_CODE}" ] || {
        echo "ERROR: no se encontró ${OVMF_CODE}" >&2
        exit 1
    }
    [ -f "${OVMF_VARS_TEMPLATE}" ] || {
        echo "ERROR: no se encontró ${OVMF_VARS_TEMPLATE}" >&2
        exit 1
    }

    if [ ! -f "${ISO_PATH}" ]; then
        mkdir -p "${OUTPUT_DIR}"
        OUTPUT_DIR="${OUTPUT_DIR}" ISO_NAME="${ISO_NAME}" \
            "${L400_SRC_DIR}/scripts/build/build_distribution.sh"
    fi
}

prepare_artifacts() {
    mkdir -p "${OUTPUT_DIR}"
    rm -f "${DISK_PATH}" "${OVMF_VARS}" "${LIVE_LOG}" "${INSTALLED_LOG}"
    qemu-img create -f qcow2 "${DISK_PATH}" "${DISK_SIZE}" >/dev/null
    cp "${OVMF_VARS_TEMPLATE}" "${OVMF_VARS}"
}

run_live_install() {
    local qemu_args=(
        qemu-system-x86_64
        -m "${QEMU_MEM_MB}"
        -smp "${QEMU_CPUS}"
        -machine q35
        -drive "if=pflash,format=raw,readonly=on,file=${OVMF_CODE}"
        -drive "if=pflash,format=raw,file=${OVMF_VARS}"
        -drive "if=virtio,format=qcow2,file=${DISK_PATH}"
        -drive "if=ide,media=cdrom,format=raw,file=${ISO_PATH}"
        -boot order=d
        -netdev user,id=n1
        -device virtio-net-pci,netdev=n1
        -serial stdio
        -display none
        -no-reboot
    )

    env \
        QEMU_LIVE_LOG="${LIVE_LOG}" \
        QEMU_CMD="$(printf "%q " "${qemu_args[@]}")" \
        expect <<'EOF'
set timeout 360
set qemu_cmd $env(QEMU_CMD)
set live_log $env(QEMU_LIVE_LOG)

log_file -noappend $live_log
spawn -noecho sh -lc $qemu_cmd

expect {
    -re {login\[[0-9]+\]: root login on 'ttyS0'} {}
    timeout {
        send_user "ERROR: timeout esperando shell root en fase live\n"
        exit 1
    }
    eof {
        send_user "ERROR: QEMU terminó antes de exponer shell live\n"
        exit 1
    }
}

sleep 1
send -- "printf 'E2E_READY\\n'\r"
expect {
    -re {E2E_READY} {}
    timeout {
        send_user "ERROR: la shell live no respondió al handshake inicial\n"
        exit 1
    }
}

send -- "stty -echo\r"
expect {
    -re {\r?\n\(none\):~# $} {}
    timeout {
        send_user "ERROR: no se pudo desactivar el eco en la shell live\n"
        exit 1
    }
}

send -- "printf '__BOOT_MODE__'; cat /run/l400/boot-mode 2>/dev/null || echo no_boot_mode\r"
expect {
    -re {__BOOT_MODE__live} {}
    timeout {
        send_user "ERROR: no apareció boot-mode=live\n"
        exit 1
    }
}

send -- "if mount | grep -q ' on / type overlay '; then printf '__OVERLAY_OK__\\n'; else printf '__OVERLAY_FALLBACK__\\n'; fi\r"
expect {
    -re {__OVERLAY_OK__|__OVERLAY_FALLBACK__} {}
    timeout {
        send_user "ERROR: no se pudo determinar el estado de overlayfs\n"
        exit 1
    }
}

send -- "if test -d /run/l400/media/boot && test -f /run/l400/media/live/BOOTX64.EFI; then printf '__INSTALL_ASSETS_OK__\\n'; elif test -f /opt/l400/boot/vmlinuz && test -f /opt/l400/boot/initramfs.img && test -f /opt/l400/boot/BOOTX64.EFI; then printf '__INSTALL_ASSETS_OK__\\n'; else printf '__INSTALL_ASSETS_MISSING__\\n'; fi\r"
expect {
    -re {__INSTALL_ASSETS_OK__} {}
    timeout {
        send_user "ERROR: el live no expuso los assets de instalación\n"
        exit 1
    }
}

send -- "if grep -qw vfat /proc/filesystems; then printf '__VFAT_FS_OK__\\n'; else printf '__VFAT_FS_MISSING__\\n'; fi\r"
expect {
    -re {__VFAT_FS_OK__} {}
    timeout {
        send_user "ERROR: vfat no aparece en /proc/filesystems dentro del live\n"
        exit 1
    }
}

send -- "install-linux400 /dev/vda\r"
expect {
    -re {=== Linux/400 instalado ===} {}
    timeout {
        send_user "ERROR: la instalación no terminó dentro del tiempo esperado\n"
        exit 1
    }
}

send -- "sync; poweroff -f || halt -f\r"
expect {
    eof {}
    timeout {
        send_user "ERROR: la VM live no se apagó tras instalar\n"
        exit 1
    }
}
EOF
}

run_installed_validation() {
    local qemu_args=(
        qemu-system-x86_64
        -m "${QEMU_MEM_MB}"
        -smp "${QEMU_CPUS}"
        -machine q35
        -drive "if=pflash,format=raw,readonly=on,file=${OVMF_CODE}"
        -drive "if=pflash,format=raw,file=${OVMF_VARS}"
        -drive "if=virtio,format=qcow2,file=${DISK_PATH}"
        -boot order=c
        -netdev user,id=n1
        -device virtio-net-pci,netdev=n1
        -serial stdio
        -display none
        -no-reboot
    )

    env \
        QEMU_INSTALLED_LOG="${INSTALLED_LOG}" \
        QEMU_CMD="$(printf "%q " "${qemu_args[@]}")" \
        expect <<'EOF'
set timeout 300
set qemu_cmd $env(QEMU_CMD)
set installed_log $env(QEMU_INSTALLED_LOG)

log_file -noappend $installed_log
spawn -noecho sh -lc $qemu_cmd

expect {
    -re {login\[[0-9]+\]: root login on 'ttyS0'} {}
    timeout {
        send_user "ERROR: timeout esperando shell root en sistema instalado\n"
        exit 1
    }
    eof {
        send_user "ERROR: QEMU terminó antes de exponer shell del sistema instalado\n"
        exit 1
    }
}

sleep 1
send -- "printf 'E2E_READY\\n'\r"
expect {
    -re {E2E_READY} {}
    timeout {
        send_user "ERROR: la shell instalada no respondió al handshake inicial\n"
        exit 1
    }
}

send -- "stty -echo\r"
expect {
    -re {\r?\n\(none\):~# $} {}
    timeout {
        send_user "ERROR: no se pudo desactivar el eco en la shell instalada\n"
        exit 1
    }
}

send -- "printf '__BOOT_MODE__'; cat /run/l400/boot-mode 2>/dev/null || echo no_boot_mode\r"
expect {
    -re {__BOOT_MODE__installed} {}
    timeout {
        send_user "ERROR: no apareció boot-mode=installed\n"
        exit 1
    }
}

send -- "if grep -q 'l400.installed=1' /proc/cmdline; then printf '__EFI_BOOT_OK__\\n'; else printf '__EFI_BOOT_MISSING__\\n'; fi\r"
expect {
    -re {__EFI_BOOT_OK__} {}
    timeout {
        send_user "ERROR: no se encontró BOOTX64.EFI en el sistema instalado\n"
        exit 1
    }
}

send -- "grep '^tty1::respawn:' /etc/inittab || true\r"
expect {
    -re {l400-console-autologin} {}
    timeout {
        send_user "ERROR: tty1 no quedó configurado para lanzar Linux/400\n"
        exit 1
    }
}

send -- "test -x /opt/l400/bin/os400-tui && printf '__TUI_BIN_OK__\\n' || printf '__TUI_BIN_MISSING__\\n'\r"
expect {
    -re {__TUI_BIN_OK__} {}
    timeout {
        send_user "ERROR: os400-tui no está disponible en el sistema instalado\n"
        exit 1
    }
}

send -- "test -x /usr/local/bin/l400-support-report && printf '__SUPPORT_REPORT_OK__\\n' || printf '__SUPPORT_REPORT_MISSING__\\n'\r"
expect {
    -re {__SUPPORT_REPORT_OK__} {}
    timeout {
        send_user "ERROR: l400-support-report no está disponible en el sistema instalado\n"
        exit 1
    }
}

send -- "mkdir -p /run && l400-support-report --write >/run/l400-support.out && grep -q '^effective_mode=' /run/l400/support-profile && printf '__SUPPORT_PROFILE_OK__\\n' || printf '__SUPPORT_PROFILE_FAIL__\\n'\r"
expect {
    -re {__SUPPORT_PROFILE_OK__} {}
    timeout {
        send_user "ERROR: no se pudo generar support-profile en el sistema instalado\n"
        exit 1
    }
}

send -- "poweroff -f || halt -f\r"
expect {
    eof {}
    timeout {
        send_user "ERROR: la VM instalada no se apagó correctamente\n"
        exit 1
    }
}
EOF
}

summarize() {
    echo "=== E2E Linux/400 OK ==="
    echo "ISO      : ${ISO_PATH}"
    echo "Disco    : ${DISK_PATH}"
    echo "Live log : ${LIVE_LOG}"
    echo "Boot log : ${INSTALLED_LOG}"
}

main() {
    ensure_inputs
    prepare_artifacts
    run_live_install
    run_installed_validation
    summarize
}

main "$@"
