#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_RUN="$(mktemp -d)"
trap 'rm -rf "$TMP_RUN"' EXIT

cd "$ROOT_DIR"

OUTPUT="$(L400_RUN_DIR="$TMP_RUN" cargo run -p l400 --example workload_demo)"
printf '%s\n' "$OUTPUT"

grep -q "== Workload snapshot ==" <<<"$OUTPUT"
grep -q "WORKLOADDEMO" <<<"$OUTPUT"
grep -q "BATCHDEMO" <<<"$OUTPUT"
grep -Eq "ACTIVE|DEGRADED" <<<"$OUTPUT"
grep -q "QBATCH" <<<"$OUTPUT"
grep -q "Linux/400 batch demo" <<<"$OUTPUT"
