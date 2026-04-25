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
PERSISTENCE_LOG="${OUTPUT_DIR}/qemu-installed-persistence.log"

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
    rm -f "${DISK_PATH}" "${OVMF_VARS}" "${LIVE_LOG}" "${INSTALLED_LOG}" "${PERSISTENCE_LOG}"
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

send -- "grep -q '^l400_root_persistent=yes' /run/l400/support-profile && test -d /l400/QSYS && test -d /l400/QGPL && test -d /l400/QUSRSYS && printf '__L400_PERSIST_OK__\\n' || printf '__L400_PERSIST_FAIL__\\n'\r"
expect {
    -re {__L400_PERSIST_OK__} {}
    timeout {
        send_user "ERROR: /l400 no quedó persistente o no conserva objetos base tras boot instalado\n"
        exit 1
    }
}

send -- "CRTLIB LIB(QE2E) >/tmp/l400-e2e-crtlib.out 2>&1 && printf '__E2E_CRTLIB_OK__\\n' || { cat /tmp/l400-e2e-crtlib.out; printf '__E2E_USER_SEED_FAIL__\\n'; }\r"
expect {
    -re {__E2E_CRTLIB_OK__} {}
    -re {__E2E_USER_SEED_FAIL__} {
        send_user "ERROR: no se pudo crear biblioteca de usuario QE2E\n"
        exit 1
    }
    timeout {
        send_user "ERROR: no se pudo crear biblioteca de usuario QE2E\n"
        exit 1
    }
}

send -- "CRTPF FILE(QE2E/CUST) RCDLEN(80) >/tmp/l400-e2e-crtpf.out 2>&1 && WRTPFM FILE(QE2E/CUST) KEY(C001) DATA(PERSISTED_CUSTOMER) >/tmp/l400-e2e-wrtpfm.out 2>&1 && CRTDTAQ DTAQ(QE2E/E2EQ) >/tmp/l400-e2e-crtdtaq.out 2>&1 && SNDDTAQ DTAQ(QE2E/E2EQ) MSG(PERSISTED_MESSAGE) >/tmp/l400-e2e-snddtaq.out 2>&1 && mkdir -p /l400/QGPL/QCLSRC && printf 'PGM\\nDCL VAR(&MSG) TYPE(*CHAR) LEN(32)\\nENDPGM\\n' >/l400/QGPL/QCLSRC/E2E.CLP && GRTOBJAUT OBJ(QE2E/CUST) USER(QPGMR) AUT(*USE) >/tmp/l400-e2e-grtaut.out 2>&1 && printf '__E2E_USER_STATE_SEEDED__\\n' || { cat /tmp/l400-e2e-*.out 2>/dev/null || true; printf '__E2E_USER_SEED_FAIL__\\n'; }\r"
expect {
    -re {__E2E_USER_STATE_SEEDED__} {}
    -re {__E2E_USER_SEED_FAIL__} {
        send_user "ERROR: no se pudo sembrar estado de usuario persistente\n"
        exit 1
    }
    timeout {
        send_user "ERROR: timeout sembrando estado de usuario persistente\n"
        exit 1
    }
}

send -- "sync; poweroff -f || halt -f\r"
expect {
    eof {}
    timeout {
        send_user "ERROR: la VM instalada no se apagó tras sembrar datos de usuario\n"
        exit 1
    }
}
EOF
}

run_persistence_validation() {
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
        QEMU_PERSISTENCE_LOG="${PERSISTENCE_LOG}" \
        QEMU_CMD="$(printf "%q " "${qemu_args[@]}")" \
        expect <<'EOF'
set timeout 300
set qemu_cmd $env(QEMU_CMD)
set persistence_log $env(QEMU_PERSISTENCE_LOG)

log_file -noappend $persistence_log
spawn -noecho sh -lc $qemu_cmd

expect {
    -re {login\[[0-9]+\]: root login on 'ttyS0'} {}
    timeout {
        send_user "ERROR: timeout esperando shell root para validar persistencia\n"
        exit 1
    }
    eof {
        send_user "ERROR: QEMU terminó antes de validar persistencia\n"
        exit 1
    }
}

sleep 1
send -- "printf 'E2E_READY\\n'\r"
expect {
    -re {E2E_READY} {}
    timeout {
        send_user "ERROR: la shell de persistencia no respondió al handshake inicial\n"
        exit 1
    }
}

send -- "stty -echo\r"
expect {
    -re {\r?\n\(none\):~# $} {}
    timeout {
        send_user "ERROR: no se pudo desactivar el eco en validacion de persistencia\n"
        exit 1
    }
}

send -- "WRKOBJ LIB(QE2E) >/tmp/l400-e2e-wrkobj.out 2>&1 && grep -q 'CUST' /tmp/l400-e2e-wrkobj.out && grep -q 'E2EQ' /tmp/l400-e2e-wrkobj.out && printf '__E2E_WRKOBJ_OK__\\n' || { cat /tmp/l400-e2e-wrkobj.out; printf '__E2E_WRKOBJ_FAIL__\\n'; }\r"
expect {
    -re {__E2E_WRKOBJ_OK__} {}
    -re {__E2E_WRKOBJ_FAIL__} {
        send_user "ERROR: WRKOBJ no encontró objetos de usuario persistidos\n"
        exit 1
    }
    timeout {
        send_user "ERROR: timeout validando WRKOBJ de objetos persistidos\n"
        exit 1
    }
}

send -- "WRKMBRPDM FILE(QGPL/QCLSRC) >/tmp/l400-e2e-wrkmbrpdm.out 2>&1 && grep -q 'E2E.CLP' /tmp/l400-e2e-wrkmbrpdm.out && printf '__E2E_WRKMBRPDM_OK__\\n' || { cat /tmp/l400-e2e-wrkmbrpdm.out; printf '__E2E_WRKMBRPDM_FAIL__\\n'; }\r"
expect {
    -re {__E2E_WRKMBRPDM_OK__} {}
    -re {__E2E_WRKMBRPDM_FAIL__} {
        send_user "ERROR: WRKMBRPDM no encontró el miembro CL persistido\n"
        exit 1
    }
    timeout {
        send_user "ERROR: timeout validando miembro CL persistido\n"
        exit 1
    }
}

send -- "DSPPFM FILE(QE2E/CUST) >/tmp/l400-e2e-dsppfm.out 2>&1 && grep -q 'PERSISTED_CUSTOMER' /tmp/l400-e2e-dsppfm.out && printf '__E2E_DSPPFM_OK__\\n' || { cat /tmp/l400-e2e-dsppfm.out; printf '__E2E_DSPPFM_FAIL__\\n'; }\r"
expect {
    -re {__E2E_DSPPFM_OK__} {}
    -re {__E2E_DSPPFM_FAIL__} {
        send_user "ERROR: DSPPFM no mostró el registro persistido\n"
        exit 1
    }
    timeout {
        send_user "ERROR: timeout validando registros PF persistidos\n"
        exit 1
    }
}

send -- "DSPDTAQ DTAQ(QE2E/E2EQ) >/tmp/l400-e2e-dspdtaq.out 2>&1 && grep -q 'PERSISTED_MESSAGE' /tmp/l400-e2e-dspdtaq.out && printf '__E2E_DSPDTAQ_OK__\\n' || { cat /tmp/l400-e2e-dspdtaq.out; printf '__E2E_DSPDTAQ_FAIL__\\n'; }\r"
expect {
    -re {__E2E_DSPDTAQ_OK__} {}
    -re {__E2E_DSPDTAQ_FAIL__} {
        send_user "ERROR: DSPDTAQ no mostró el mensaje persistido\n"
        exit 1
    }
    timeout {
        send_user "ERROR: timeout validando DTAQ persistida\n"
        exit 1
    }
}

send -- "DSPOBJAUT OBJ(QE2E/CUST) >/tmp/l400-e2e-dspobjaut.out 2>&1 && grep -q 'QPGMR' /tmp/l400-e2e-dspobjaut.out && grep -q '\\*USE' /tmp/l400-e2e-dspobjaut.out && printf '__E2E_AUTH_OK__\\n' || { cat /tmp/l400-e2e-dspobjaut.out; printf '__E2E_AUTH_FAIL__\\n'; }\r"
expect {
    -re {__E2E_AUTH_OK__} {}
    -re {__E2E_AUTH_FAIL__} {
        send_user "ERROR: DSPOBJAUT no mostró la autorizacion persistida\n"
        exit 1
    }
    timeout {
        send_user "ERROR: timeout validando autorizacion persistida\n"
        exit 1
    }
}

send -- "mkdir -p /run/l400 && l400-support-report --write >/tmp/l400-e2e-support.out 2>&1 && grep -q '^l400_root_persistent=yes' /run/l400/support-profile && printf '__E2E_SUPPORT_PERSIST_OK__\\n' || { cat /tmp/l400-e2e-support.out; printf '__E2E_SUPPORT_PERSIST_FAIL__\\n'; }\r"
expect {
    -re {__E2E_SUPPORT_PERSIST_OK__} {}
    -re {__E2E_SUPPORT_PERSIST_FAIL__} {
        send_user "ERROR: support-report no reportó backend persistente tras reboot\n"
        exit 1
    }
    timeout {
        send_user "ERROR: timeout validando support-report persistente\n"
        exit 1
    }
}

send -- "poweroff -f || halt -f\r"
expect {
    eof {}
    timeout {
        send_user "ERROR: la VM de persistencia no se apagó correctamente\n"
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
    echo "Persist : ${PERSISTENCE_LOG}"
}

main() {
    ensure_inputs
    prepare_artifacts
    run_live_install
    run_installed_validation
    run_persistence_validation
    summarize
}

main "$@"
