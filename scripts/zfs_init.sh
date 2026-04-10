#!/bin/bash
set -e

echo "=== Linux/400 - ZFS Initialization ==="

if [[ $EUID -ne 0 ]]; then
    echo "[-] This script must be run as root (sudo ./zfs_init.sh)"
    exit 1
fi

POOL_NAME="l400pool"
MOUNT_POINT="/l400"
IMAGE_FILE="/var/lib/l400/l400pool.img"

# Check if pool already exists
if zpool list $POOL_NAME >/dev/null 2>&1; then
    echo "[-] Zpool $POOL_NAME already exists."
    exit 0
fi

echo "[1] Creating preparation directory..."
mkdir -p /var/lib/l400

echo "[2] Creating sparse file for zpool (10G)..."
# Create a sparse file of 10G
truncate -s 10G $IMAGE_FILE

echo "[3] Initializing zpool..."
zpool create -o ashift=12 $POOL_NAME $IMAGE_FILE

echo "[4] Setting ZFS properties..."
zfs set xattr=sa $POOL_NAME
zfs set dnodesize=auto $POOL_NAME
zfs set acltype=posixacl $POOL_NAME
zfs set compression=lz4 $POOL_NAME
zfs set mountpoint=$MOUNT_POINT $POOL_NAME

echo "[5] Creating Base System Datasets (ASP 1)..."
zfs create $POOL_NAME/QSYS     # System library
zfs create $POOL_NAME/QGPL     # General purpose library
zfs create $POOL_NAME/QTEMP    # Temporary library
zfs create $POOL_NAME/QUSRSYS  # System User Objects

# Verify xattr inheritance
XATTR_PROP=$(zfs get -H -o value xattr $POOL_NAME/QSYS)
if [[ "$XATTR_PROP" != "sa" ]]; then
    echo "[-] Error: 'xattr=sa' was not inherited properly!"
    exit 1
fi

echo "=== ZFS Initialization Complete ==="
zpool status $POOL_NAME
