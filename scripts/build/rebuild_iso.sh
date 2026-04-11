#!/bin/bash
# rebuild_iso.sh - Rebuild Linux/400 ISO with custom syslinux.cfg
# Preserva el bootloader de Alpine

ALPINE_ISO="${1:-/tmp/alpine.iso}"
OUTPUT="${2:-linux400-1.0.0.iso}"
SYSLINUX_CFG="${3:-/home/user/Source/linux400/output/l400_apps/boot/syslinux/syslinux.cfg}"

if [ ! -f "$ALPINE_ISO" ]; then
    echo "Error: Alpine ISO not found"
    exit 1
fi

echo "=== Building Linux/400 ISO ==="

xorriso \
    -indev "$ALPINE_ISO" \
    -outdev "$OUTPUT" \
    -map "$SYSLINUX_CFG" /boot/syslinux/syslinux.cfg \
    -return_with 32 \
    -commit 2>&1 || echo "Note: ISO created with warning"

echo "=== ISO ready: $OUTPUT ==="
ls -lh "$OUTPUT"