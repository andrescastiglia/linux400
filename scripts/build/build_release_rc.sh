#!/bin/bash
set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
VERSION="${VERSION:-1.0.0-rc1}"
ISO_NAME="${ISO_NAME:-linux400-${VERSION}}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"

echo "=== Linux/400 Release Candidate Build ==="
echo "Version   : ${VERSION}"
echo "ISO name  : ${ISO_NAME}"
echo "Output dir: ${OUTPUT_DIR}"

VERSION="${VERSION}" ISO_NAME="${ISO_NAME}" OUTPUT_DIR="${OUTPUT_DIR}" \
    "${L400_SRC_DIR}/scripts/build/build_distribution.sh"

echo "=== Linux/400 RC lista ==="
echo "ISO: ${OUTPUT_DIR}/${ISO_NAME}.iso"
echo "Siguientes pasos:"
echo "  ./scripts/test/test_release_rc.sh"
echo "  RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh"
