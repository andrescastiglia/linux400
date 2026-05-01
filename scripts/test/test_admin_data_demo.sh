#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

L400_ROOT_DIR="${TMP_DIR}/l400"
L400_RUN_DIR="${TMP_DIR}/run"
L400_SPOOL_DIR="${TMP_DIR}/spool"

cd "${ROOT_DIR}"
cargo build -p l400 --bin l400-bootstrap --bin l400cmd --bin sbmjob >/dev/null

L400_ROOT="${L400_ROOT_DIR}" target/debug/l400-bootstrap --quiet >/dev/null
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd CRTLIB 'LIB(QADMIN)' >/dev/null
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd CRTPF \
    'FILE(QADMIN/CUSTOMERS)' \
    'RCDLEN(80)' \
    'FIELDS(KEY:CHAR:8:Customer id,DATA:CHAR:72:Customer name)' \
    'KEY(KEY)' >/dev/null
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd CRTLF \
    'FILE(QADMIN/CUSTBYKEY)' \
    'SRCFILE(QADMIN/CUSTOMERS)' >/dev/null

L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd STRSQL \
    "INSERT INTO QADMIN/CUSTOMERS (KEY, DATA) VALUES ('C001', 'Ana Gomez')" >/dev/null
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd STRSQL \
    "INSERT INTO QADMIN/CUSTOMERS (KEY, DATA) VALUES ('C002', 'Luis Perez')" >/dev/null
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd STRSQL \
    "UPDATE QADMIN/CUSTOMERS SET DATA='Carla Ruiz' WHERE KEY='C002'" >/dev/null

L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd CRTDTAQ 'DTAQ(QADMIN/NOTIFY)' >/dev/null
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd SNDDTAQ \
    'DTAQ(QADMIN/NOTIFY)' \
    'MSG(Customer report ready)' >/dev/null

L400_ROOT="${L400_ROOT_DIR}" L400_RUN_DIR="${L400_RUN_DIR}" L400_SPOOL_DIR="${L400_SPOOL_DIR}" \
    target/debug/sbmjob --job CSTRPT target/debug/l400cmd DSPPFM 'FILE(QADMIN/CUSTOMERS)' >/dev/null

for _ in $(seq 1 50); do
    if grep -R "status=COMPLETED" "${L400_SPOOL_DIR}" >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd STRSQL \
    "SELECT KEY, DATA FROM QADMIN/CUSTOMERS ORDER BY KEY" | grep -q 'Carla Ruiz'
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd DSPDTAQ 'DTAQ(QADMIN/NOTIFY)' | grep -q 'Customer report ready'
grep -R 'Ana Gomez' "${L400_SPOOL_DIR}" >/dev/null
grep -R 'Carla Ruiz' "${L400_SPOOL_DIR}" >/dev/null
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd CHKOBJINT 'OBJ(QADMIN/CUSTOMERS)' | grep -q 'Result . . . . . . . : OK'
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd CHKOBJINT 'OBJ(QADMIN/CUSTBYKEY)' | grep -q 'Result . . . . . . . : OK'
L400_ROOT="${L400_ROOT_DIR}" target/debug/l400cmd CHKOBJINT 'OBJ(QADMIN/NOTIFY)' | grep -q 'Result . . . . . . . : OK'

echo "=== admin data demo OK ==="
