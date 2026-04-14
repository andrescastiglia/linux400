#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

TMP_RUN="$(mktemp -d)"
trap 'rm -rf "$TMP_RUN"' EXIT

DEV_OUTPUT="$(RUST_LOG=info L400_RUN_DIR="$TMP_RUN/dev" L400_BPF_PATH=/tmp/does-not-exist cargo run -p l400-loader -- --mode dev --once 2>&1)"
printf '%s\n' "$DEV_OUTPUT"
grep -q "Modo del loader: Dev" <<<"$DEV_OUTPUT"
grep -q "protección inactive" <<<"$DEV_OUTPUT"
grep -q "mode=dev" "$TMP_RUN/dev/loader-status"
grep -q "protection_active=0" "$TMP_RUN/dev/loader-status"

DEGRADED_OUTPUT="$(RUST_LOG=info L400_RUN_DIR="$TMP_RUN/degraded" L400_BPF_PATH=/tmp/does-not-exist cargo run -p l400-loader -- --mode degraded --once 2>&1)"
printf '%s\n' "$DEGRADED_OUTPUT"
grep -q "Modo del loader: Degraded" <<<"$DEGRADED_OUTPUT"
grep -q "mode=degraded" "$TMP_RUN/degraded/loader-status"
grep -q "phase=fallback" "$TMP_RUN/degraded/loader-status"

if RUST_LOG=info L400_RUN_DIR="$TMP_RUN/full" L400_BPF_PATH=/tmp/does-not-exist cargo run -p l400-loader -- --mode full --once >/tmp/l400-loader-full.log 2>&1; then
    echo "ERROR: full mode no debía continuar sin binario BPF" >&2
    exit 1
fi
