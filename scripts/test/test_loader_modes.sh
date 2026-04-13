#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

DEV_OUTPUT="$(RUST_LOG=info L400_BPF_PATH=/tmp/does-not-exist cargo run -p l400-loader -- --mode dev --once 2>&1)"
printf '%s\n' "$DEV_OUTPUT"
grep -q "Modo del loader: Dev" <<<"$DEV_OUTPUT"
grep -q "protección inactive" <<<"$DEV_OUTPUT"

DEGRADED_OUTPUT="$(RUST_LOG=info L400_BPF_PATH=/tmp/does-not-exist cargo run -p l400-loader -- --mode degraded --once 2>&1)"
printf '%s\n' "$DEGRADED_OUTPUT"
grep -q "Modo del loader: Degraded" <<<"$DEGRADED_OUTPUT"

if RUST_LOG=info L400_BPF_PATH=/tmp/does-not-exist cargo run -p l400-loader -- --mode full --once >/tmp/l400-loader-full.log 2>&1; then
    echo "ERROR: full mode no debía continuar sin binario BPF" >&2
    exit 1
fi
