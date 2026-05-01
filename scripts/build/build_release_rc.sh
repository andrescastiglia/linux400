#!/bin/bash
set -euo pipefail

L400_SRC_DIR="${L400_SRC_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
VERSION="${VERSION:-1.0.0-rc1}"
ISO_NAME="${ISO_NAME:-linux400-${VERSION}}"
OUTPUT_DIR="${OUTPUT_DIR:-${L400_SRC_DIR}/output}"
RUN_RC_GATE="${RUN_RC_GATE:-1}"
EVIDENCE_DIR="${L400_RC_EVIDENCE_DIR:-${OUTPUT_DIR}/rc-evidence/${VERSION}}"

echo "=== Linux/400 Release Candidate Build ==="
echo "Version   : ${VERSION}"
echo "ISO name  : ${ISO_NAME}"
echo "Output dir: ${OUTPUT_DIR}"
echo "Evidence : ${EVIDENCE_DIR}"

mkdir -p "${EVIDENCE_DIR}"

VERSION="${VERSION}" ISO_NAME="${ISO_NAME}" OUTPUT_DIR="${OUTPUT_DIR}" \
    "${L400_SRC_DIR}/scripts/build/build_distribution.sh" 2>&1 | tee "${EVIDENCE_DIR}/build.log"

if [[ "${RUN_RC_GATE}" == "1" ]]; then
    if [[ "${RUN_E2E_INSTALL:-0}" != "1" ]]; then
        echo "ERROR: un RC requiere smoke QEMU antes de quedar listo." >&2
        echo "Ejecuta: RUN_E2E_INSTALL=1 ./scripts/build/build_release_rc.sh" >&2
        echo "Para builds internos no-RC usa RUN_RC_GATE=0." >&2
        exit 2
    fi

    L400_RC_VERSION="${VERSION}" L400_RC_EVIDENCE_DIR="${EVIDENCE_DIR}" \
        RUN_BUILD_USERSPACE=0 RUN_E2E_INSTALL=1 \
        "${L400_SRC_DIR}/scripts/test/test_release_rc.sh"
else
    echo "WARNING: gate RC omitido por RUN_RC_GATE=0"
fi

{
    echo "version=${VERSION}"
    echo "iso_name=${ISO_NAME}"
    echo "output_dir=${OUTPUT_DIR}"
    echo "evidence_dir=${EVIDENCE_DIR}"
    echo "run_rc_gate=${RUN_RC_GATE}"
    echo "run_e2e_install=${RUN_E2E_INSTALL:-0}"
    echo "host_uname=$(uname -a)"
    echo "built_at_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "reproduce=VERSION=${VERSION} RUN_E2E_INSTALL=1 ./scripts/build/build_release_rc.sh"
} > "${EVIDENCE_DIR}/rc-manifest.env"

artifact_dir="${EVIDENCE_DIR}/artifacts"
mkdir -p "${artifact_dir}"
for artifact in \
    "${OUTPUT_DIR}/${ISO_NAME}.iso" \
    "${OUTPUT_DIR}/vmlinuz" \
    "${OUTPUT_DIR}/BOOTX64.EFI" \
    "${OUTPUT_DIR}"/initramfs-*.img
do
    if [[ -f "${artifact}" ]]; then
        cp -f "${artifact}" "${artifact_dir}/"
    fi
done

if compgen -G "${artifact_dir}/*" >/dev/null; then
    (cd "${artifact_dir}" && sha256sum * > "${EVIDENCE_DIR}/SHA256SUMS")
fi

echo "=== Linux/400 RC lista ==="
echo "ISO: ${OUTPUT_DIR}/${ISO_NAME}.iso"
echo "Evidence: ${EVIDENCE_DIR}"
