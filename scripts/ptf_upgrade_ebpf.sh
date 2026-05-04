#!/bin/bash
# PTF upgrade policy for eBPF artifact
# Phase 9: Define upgrade/PTF policy for eBPF artifact
#
# This script handles:
# - Checking current eBPF artifact version
# - Applying PTF updates to eBPF artifact
# - Rolling back if upgrade fails
# - Verifying policy version after upgrade

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

EBPF_SRC="$ROOT_DIR/l400-ebpf"
EBPF_TARGET="$ROOT_DIR/l400-ebpf/target/bpfel-unknown-none/release/l400-ebpf"
INSTALL_DIR="/opt/l400/hooks"
BACKUP_DIR="/opt/l400/hooks/backup"

echo "=== PTF Upgrade Policy for eBPF Artifact ==="

# Function to get current policy version
get_policy_version() {
    if [ -f "$INSTALL_DIR/l400-ebpf" ]; then
        # Read version from ELF notes or use file metadata
        strings "$INSTALL_DIR/l400-ebpf" 2>/dev/null | grep -m1 "v[0-9]\+\.[0-9]\+" || echo "unknown"
    else
        echo "not-installed"
    fi
}

# Function to backup current artifact
backup_artifact() {
    if [ -f "$INSTALL_DIR/l400-ebpf" ]; then
        mkdir -p "$BACKUP_DIR"
        cp "$INSTALL_DIR/l400-ebpf" "$BACKUP_DIR/l400-ebpf.$(date +%Y%m%d_%H%M%S)"
        echo "Backup created in $BACKUP_DIR"
    fi
}

# Function to build eBPF artifact
build_ebpf() {
    echo "Building eBPF artifact..."
    cd "$EBPF_SRC"
    cargo build --target bpfel-unknown-none --release 2>&1 | tail -20
    if [ ! -f "$EBPF_TARGET" ]; then
        echo "ERROR: Build failed, artifact not found"
        return 1
    fi
    echo "Build successful: $EBPF_TARGET"
}

# Function to install eBPF artifact
install_ebpf() {
    echo "Installing eBPF artifact..."
    mkdir -p "$INSTALL_DIR"
    cp "$EBPF_TARGET" "$INSTALL_DIR/l400-ebpf"
    chmod +x "$INSTALL_DIR/l400-ebpf"
    echo "Installed to $INSTALL_DIR/l400-ebpf"
}

# Function to verify installation
verify_installation() {
    echo "Verifying installation..."
    if [ ! -f "$INSTALL_DIR/l400-ebpf" ]; then
        echo "ERROR: Artifact not installed"
        return 1
    fi
    
    # Check if loader can read it
    export L400_BPF_PATH="$INSTALL_DIR/l400-ebpf"
    if RUST_LOG=error cargo run -p l400-loader -- --mode full --once 2>&1 | grep -q "LSM Hooks.*ensamblados"; then
        echo "OK: Loader accepts the artifact"
    else
        echo "WARN: Loader could not verify artifact (may need root)"
    fi
}

# Function to rollback
rollback() {
    echo "Rolling back to previous version..."
    LATEST_BACKUP=$(ls -t "$BACKUP_DIR"/l400-ebpf.* 2>/dev/null | head -1)
    if [ -n "$LATEST_BACKUP" ]; then
        cp "$LATEST_BACKUP" "$INSTALL_DIR/l400-ebpf"
        echo "Rolled back to: $LATEST_BACKUP"
    else
        echo "ERROR: No backup found for rollback"
        return 1
    fi
}

# Main logic
case "${1:-status}" in
    status)
        echo "Current policy version: $(get_policy_version)"
        echo "Install directory: $INSTALL_DIR"
        if [ -f "$INSTALL_DIR/l400-ebpf" ]; then
            echo "Artifact size: $(du -h "$INSTALL_DIR/l400-ebpf" | cut -f1)"
            echo "Artifact date: $(stat -c %y "$INSTALL_DIR/l400-ebpf" 2>/dev/null || stat -f "%Sm" "$INSTALL_DIR/l400-ebpf" 2>/dev/null)"
        else
            echo "Artifact: not installed"
        fi
        ;;
    
    build)
        build_ebpf
        ;;
    
    install)
        backup_artifact
        build_ebpf || { echo "Build failed"; exit 1; }
        install_ebpf
        verify_installation
        echo "=== Installation complete ==="
        ;;
    
    rollback)
        rollback
        verify_installation
        ;;
    
    check)
        echo "Checking eBPF artifact..."
        echo "Policy version: $(get_policy_version)"
        echo "Expected: v1.0"
        if [ -f "$INSTALL_DIR/l400-ebpf" ]; then
            echo "Status: installed"
        else
            echo "Status: not installed"
        fi
        ;;
    
    *)
        echo "Usage: $0 {status|build|install|rollback|check}"
        exit 1
        ;;
esac
