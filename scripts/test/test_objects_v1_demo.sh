#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_ROOT"' EXIT

cd "$ROOT_DIR"

OUTPUT="$(cargo run -p l400 --example objects_v1_demo -- "$TMP_ROOT")"
printf '%s\n' "$OUTPUT"

grep -q "Library QSYS:" <<<"$OUTPUT"
grep -q "HELLO \*PGM C Demo cataloged program" <<<"$OUTPUT"
grep -q "CUSTOMERS \*FILE PF Physical file" <<<"$OUTPUT"
grep -q "CUSTBYNAME \*FILE LF Logical file" <<<"$OUTPUT"
grep -q "Library QUSRSYS:" <<<"$OUTPUT"
grep -q "QEZJOBLOG \*DTAQ DTAQ Data queue" <<<"$OUTPUT"
grep -q "PF CUSTOMERS records: 2" <<<"$OUTPUT"
grep -q "LF CUSTBYNAME index entries: 2" <<<"$OUTPUT"
grep -q "DTAQ QEZJOBLOG messages: 1" <<<"$OUTPUT"
