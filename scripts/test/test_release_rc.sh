#!/bin/bash
set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
RUN_E2E_INSTALL="${RUN_E2E_INSTALL:-0}"
RUN_BUILD_USERSPACE="${RUN_BUILD_USERSPACE:-1}"
L400_RELEASE_GATE="${L400_RELEASE_GATE:-all}"

run_dev_fast() {
    echo "=== Cargo test gate ==="
    cargo test -p l400
    cargo test -p clc
    cargo test -p os400-tui
}

run_userspace() {
    if [[ "${RUN_BUILD_USERSPACE}" == "1" ]]; then
        echo "=== Building userspace ==="
        "${L400_SRC_DIR}/scripts/build/build_userspace.sh"
    else
        echo "Skipping userspace build (RUN_BUILD_USERSPACE=${RUN_BUILD_USERSPACE})"
    fi

    echo "=== Runtime smoke scripts ==="
    "${L400_SRC_DIR}/scripts/test/test_objects_v1_demo.sh"
    "${L400_SRC_DIR}/scripts/test/test_toolchain_v1_demo.sh"
    "${L400_SRC_DIR}/scripts/test/test_workload_demo.sh"
    "${L400_SRC_DIR}/scripts/test/test_admin_data_demo.sh"
    "${L400_SRC_DIR}/scripts/test/test_loader_modes.sh"
    bash "${L400_SRC_DIR}/scripts/test/test_support_profile.sh"
}

run_kernel_optional() {
    echo "=== eBPF build gate (optional toolchain) ==="
    "${L400_SRC_DIR}/scripts/test/test_ebpf_build_optional.sh"
}

run_upgrade_restore() {
    echo "=== Backup/restore and upgrade gate ==="
    bash "${L400_SRC_DIR}/scripts/test/test_l400_backup_restore.sh"
}

run_install_qemu() {
    if [[ "${RUN_E2E_INSTALL}" == "1" ]]; then
        echo "=== Running QEMU install smoke test ==="
        "${L400_SRC_DIR}/scripts/test/test_e2e_install_qemu.sh"
    else
        echo "Skipping QEMU install smoke test (set RUN_E2E_INSTALL=1 to enable)"
    fi
}

echo "=== Linux/400 RC smoke tests (${L400_RELEASE_GATE}) ==="

case "${L400_RELEASE_GATE}" in
    dev-fast)
        run_dev_fast
        ;;
    userspace)
        run_dev_fast
        run_userspace
        ;;
    kernel-optional)
        run_kernel_optional
        ;;
    install-qemu)
        run_install_qemu
        ;;
    upgrade-restore)
        run_upgrade_restore
        ;;
    all)
        run_dev_fast
        run_userspace
        run_kernel_optional
        run_upgrade_restore
        run_install_qemu
        ;;
    *)
        echo "ERROR: L400_RELEASE_GATE no soportado: ${L400_RELEASE_GATE}" >&2
        echo "Use: dev-fast, userspace, kernel-optional, install-qemu, upgrade-restore, all" >&2
        exit 2
        ;;
esac

echo "=== Linux/400 RC smoke tests passed ==="
