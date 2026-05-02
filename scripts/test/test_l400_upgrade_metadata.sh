#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

L400_ROOT_DIR="${TMP_DIR}/l400"
RUN_DIR="${TMP_DIR}/run"

mkdir -p "${L400_ROOT_DIR}" "${RUN_DIR}"

L400_ROOT="${L400_ROOT_DIR}" cargo run -p l400 --bin l400-bootstrap -- --quiet >/dev/null
printf '0\n' > "${L400_ROOT_DIR}/.metadata-version"

L400_ROOT="${L400_ROOT_DIR}" cargo run -p l400 --bin l400cmd -- CRTLIB 'LIB(QUPGRADE)' >/dev/null
L400_ROOT="${L400_ROOT_DIR}" cargo run -p l400 --bin l400cmd -- CRTPF 'FILE(QUPGRADE/PFOLD)' 'RCDLEN(32)' >/dev/null
L400_ROOT="${L400_ROOT_DIR}" cargo run -p l400 --bin l400cmd -- WRTPFM 'FILE(QUPGRADE/PFOLD)' 'KEY(OLD)' 'DATA(V0)' >/dev/null

upgrade_check_output="$(L400_ROOT="${L400_ROOT_DIR}" L400_RUN_DIR="${RUN_DIR}" "${ROOT_DIR}/scripts/runtime/l400-upgrade-check.sh")"
printf '%s\n' "${upgrade_check_output}" | grep -q 'metadata_version=0'

migrate_output="$(L400_ROOT="${L400_ROOT_DIR}" L400_METADATA_VERSION=1 "${ROOT_DIR}/scripts/runtime/l400-migrate.sh")"
printf '%s\n' "${migrate_output}" | grep -q 'status=migrated'

grep -q '^1$' "${L400_ROOT_DIR}/.metadata-version"
L400_ROOT="${L400_ROOT_DIR}" cargo run -p l400 --bin l400cmd -- DSPPFM 'FILE(QUPGRADE/PFOLD)' | grep -q 'V0'
L400_ROOT="${L400_ROOT_DIR}" cargo run -p l400 --bin l400cmd -- CHKOBJINT 'OBJ(QUPGRADE/PFOLD)' | grep -q 'Result . . . . . . . : OK'

if L400_ROOT="${L400_ROOT_DIR}" L400_METADATA_VERSION=0 "${ROOT_DIR}/scripts/runtime/l400-migrate.sh" >/tmp/l400-downgrade.$$ 2>&1; then
    cat /tmp/l400-downgrade.$$
    rm -f /tmp/l400-downgrade.$$
    echo "ERROR: downgrade should not be supported" >&2
    exit 1
fi
grep -q 'downgrade de metadata no soportado' /tmp/l400-downgrade.$$
rm -f /tmp/l400-downgrade.$$

echo "=== metadata upgrade OK ==="
echo "root=${L400_ROOT_DIR}"
