#!/bin/bash
set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
VERSION="${VERSION:-1.0.0-rc1}"
ISO_NAME="${ISO_NAME:-linux400-${VERSION}}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"
RUN_RC_GATE="${RUN_RC_GATE:-1}"

echo "=== Linux/400 Release Candidate Build ==="
echo "Version   : ${VERSION}"
echo "ISO name  : ${ISO_NAME}"
echo "Output dir: ${OUTPUT_DIR}"

VERSION="${VERSION}" ISO_NAME="${ISO_NAME}" OUTPUT_DIR="${OUTPUT_DIR}" \
    "${L400_SRC_DIR}/scripts/build/build_distribution.sh"

if [[ "${RUN_RC_GATE}" == "1" ]]; then
    if [[ "${RUN_E2E_INSTALL:-0}" != "1" ]]; then
        echo "ERROR: un RC requiere smoke QEMU antes de quedar listo." >&2
        echo "Ejecuta: RUN_E2E_INSTALL=1 ./scripts/build/build_release_rc.sh" >&2
        echo "Para builds internos no-RC usa RUN_RC_GATE=0." >&2
        exit 2
    fi

    RUN_BUILD_USERSPACE=0 RUN_E2E_INSTALL=1 "${L400_SRC_DIR}/scripts/test/test_release_rc.sh"
else
    echo "WARNING: gate RC omitido por RUN_RC_GATE=0"
fi

echo "=== Linux/400 RC lista ==="
echo "ISO: ${OUTPUT_DIR}/${ISO_NAME}.iso"
