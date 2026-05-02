#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

SRC_ROOT="${TMP_DIR}/l400-src"
DST_ROOT="${TMP_DIR}/l400-dst"
BACKUP_DIR="${TMP_DIR}/backup"

mkdir -p "${SRC_ROOT}" "${DST_ROOT}" "${BACKUP_DIR}"

L400_ROOT="${SRC_ROOT}" cargo run -p l400 --bin l400-bootstrap -- --quiet >/dev/null
L400_ROOT="${SRC_ROOT}" cargo run -p l400 --bin l400cmd -- CRTLIB 'LIB(QRESTORE)' >/dev/null
L400_ROOT="${SRC_ROOT}" cargo run -p l400 --bin l400cmd -- CRTPF 'FILE(QRESTORE/PF1)' 'RCDLEN(32)' >/dev/null
L400_ROOT="${SRC_ROOT}" cargo run -p l400 --bin l400cmd -- WRTPFM 'FILE(QRESTORE/PF1)' 'KEY(K1)' 'DATA(V1)' >/dev/null
L400_ROOT="${SRC_ROOT}" cargo run -p l400 --bin l400cmd -- CRTLF 'FILE(QRESTORE/PF1L)' 'SRCFILE(QRESTORE/PF1)' >/dev/null
L400_ROOT="${SRC_ROOT}" cargo run -p l400 --bin l400cmd -- CRTDTAQ 'DTAQ(QRESTORE/DQ1)' >/dev/null
L400_ROOT="${SRC_ROOT}" cargo run -p l400 --bin l400cmd -- SNDDTAQ 'DTAQ(QRESTORE/DQ1)' 'MSG(RESTORE_MESSAGE)' >/dev/null

rsync -aX "${SRC_ROOT}/" "${BACKUP_DIR}/"
rsync -aX --delete "${BACKUP_DIR}/" "${DST_ROOT}/"

L400_ROOT="${DST_ROOT}" cargo run -p l400 --bin l400cmd -- WRKOBJ 'LIB(QRESTORE)' | grep -q 'PF1'
L400_ROOT="${DST_ROOT}" cargo run -p l400 --bin l400cmd -- DSPPFM 'FILE(QRESTORE/PF1)' | grep -q 'V1'
L400_ROOT="${DST_ROOT}" cargo run -p l400 --bin l400cmd -- DSPDTAQ 'DTAQ(QRESTORE/DQ1)' | grep -q 'RESTORE_MESSAGE'
L400_ROOT="${DST_ROOT}" cargo run -p l400 --bin l400cmd -- CHKOBJINT 'OBJ(QRESTORE/PF1)' | grep -q 'Result . . . . . . . : OK'
L400_ROOT="${DST_ROOT}" cargo run -p l400 --bin l400cmd -- CHKOBJINT 'OBJ(QRESTORE/PF1L)' | grep -q 'Result . . . . . . . : OK'
L400_ROOT="${DST_ROOT}" cargo run -p l400 --bin l400cmd -- CHKOBJINT 'OBJ(QRESTORE/DQ1)' | grep -q 'Result . . . . . . . : OK'

if command -v getfattr >/dev/null 2>&1; then
    getfattr -n user.l400.data.version --only-values "${DST_ROOT}/QRESTORE/PF1" | grep -q '^1$'
    getfattr -n user.l400.data.version --only-values "${DST_ROOT}/QRESTORE/DQ1" | grep -q '^1$'
fi

echo "=== backup/restore OK ==="
echo "source=${SRC_ROOT}"
echo "restored=${DST_ROOT}"
