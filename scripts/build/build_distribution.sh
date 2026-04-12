#!/bin/bash
# build_distribution.sh - Orquesta la construcción live/install de Linux/400

set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"

export L400_SRC_DIR OUTPUT_DIR

"${L400_SRC_DIR}/scripts/build/build_userspace.sh"
"${L400_SRC_DIR}/scripts/build/build_alpine_base.sh"
"${L400_SRC_DIR}/scripts/build/build_initramfs.sh"
"${L400_SRC_DIR}/scripts/build/build_iso.sh"

echo "=== Pipeline Linux/400 completado ==="
echo "Artefactos en: ${OUTPUT_DIR}"
