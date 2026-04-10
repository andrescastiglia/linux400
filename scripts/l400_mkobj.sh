#!/bin/bash
set -e

if [[ $# -lt 3 ]]; then
    echo "Usage: $0 <OBJ_NAME> <TYPE> <LIB_NAME> [ASP_NUMBER]"
    echo "Supported types: *PGM, *FILE, *USRPRF, *DTAQ, *CMD, *SRVPGM, *OUTQ, etc."
    exit 1
fi

OBJ_NAME=$1
TYPE=$2
LIB_NAME=$3
ASP_NUMBER=${4:-}

MOUNT_POINT="/l400"

# Note: Setting xattrs usually requires proper ownership or root

if [[ -n "$ASP_NUMBER" ]]; then
    TARGET_DIR="${MOUNT_POINT}/asp${ASP_NUMBER}/${LIB_NAME}"
else
    TARGET_DIR="${MOUNT_POINT}/${LIB_NAME}"
fi

TARGET_FILE="${TARGET_DIR}/${OBJ_NAME}"

if [[ ! -d "$TARGET_DIR" ]]; then
    echo "[-] Library directory $TARGET_DIR does not exist!"
    exit 1
fi

if [[ "$TYPE" != "\*"* ]]; then
    echo "[-] Warning: Object type does not start with '*' (e.g. *PGM). Proceeding anyway."
fi

echo "[1] Creating object $TARGET_FILE..."
touch "$TARGET_FILE"

echo "[2] Setting OS/400 standard extended attributes..."
setfattr -n user.l400.objtype -v "$TYPE" "$TARGET_FILE"
setfattr -n user.l400.crtdate -v "$(date -Iseconds)" "$TARGET_FILE"

OWNER=${SUDO_USER:-${USER:-$(whoami)}}
setfattr -n user.l400.owner -v "$OWNER" "$TARGET_FILE"

echo "=== Object $OBJ_NAME created in $LIB_NAME with type $TYPE ==="
getfattr -d -m "user.l400" "$TARGET_FILE"
