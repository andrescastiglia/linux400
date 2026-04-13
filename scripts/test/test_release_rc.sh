#!/bin/bash
set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
RUN_E2E_INSTALL="${RUN_E2E_INSTALL:-0}"

echo "=== Linux/400 RC smoke tests ==="

"${L400_SRC_DIR}/scripts/test/test_objects_v1_demo.sh"
"${L400_SRC_DIR}/scripts/test/test_toolchain_v1_demo.sh"
"${L400_SRC_DIR}/scripts/test/test_workload_demo.sh"
"${L400_SRC_DIR}/scripts/test/test_loader_modes.sh"

if [[ "${RUN_E2E_INSTALL}" == "1" ]]; then
    echo "=== Running QEMU install smoke test ==="
    "${L400_SRC_DIR}/scripts/test/test_e2e_install_qemu.sh"
else
    echo "Skipping QEMU install smoke test (set RUN_E2E_INSTALL=1 to enable)"
fi

echo "=== Linux/400 RC smoke tests passed ==="
