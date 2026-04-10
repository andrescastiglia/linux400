#!/bin/bash
set -e

if [[ $EUID -ne 0 ]]; then
    echo "[-] This script must be run as root."
    exit 1
fi

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <LIB_NAME> [ASP_NUMBER]"
    exit 1
fi

LIB_NAME=$1
ASP_NUMBER=${2:-} # Optional, defaults to empty meaning ASP 1 (root of pool)

POOL_NAME="l400pool"
MOUNT_POINT="/l400"

if [[ -n "$ASP_NUMBER" ]]; then
    ASP_DIR="asp${ASP_NUMBER}"
    DATASET="${POOL_NAME}/${ASP_DIR}/${LIB_NAME}"
    TARGET_DIR="${MOUNT_POINT}/${ASP_DIR}/${LIB_NAME}"
    
    # Ensure ASP dataset exists if an ASP number is given
    if ! zfs list "${POOL_NAME}/${ASP_DIR}" >/dev/null 2>&1; then
        echo "[!] ASP dataset ${POOL_NAME}/${ASP_DIR} does not exist. Creating..."
        zfs create "${POOL_NAME}/${ASP_DIR}"
    fi
else
    DATASET="${POOL_NAME}/${LIB_NAME}"
    TARGET_DIR="${MOUNT_POINT}/${LIB_NAME}"
fi

echo "[1] Creating library (dataset) $DATASET..."
zfs create $DATASET

echo "[2] Tagging directory with L400 metadata..."
# Assign *LIB xattr
setfattr -n user.l400.objtype -v "*LIB" $TARGET_DIR

# Using SUDO_USER if available, otherwise fallback to $USER or root
OWNER=${SUDO_USER:-${USER:-root}}
setfattr -n user.l400.owner -v "$OWNER" $TARGET_DIR

# Also update POSIX ownership
chown $OWNER:$OWNER $TARGET_DIR

echo "=== Library $LIB_NAME successfully created at $TARGET_DIR ==="
getfattr -d -m "user.l400" $TARGET_DIR
