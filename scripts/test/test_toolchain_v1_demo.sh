#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_ROOT"' EXIT

cd "$ROOT_DIR"

cargo run -p l400 --example objects_v1_demo -- "$TMP_ROOT" >/dev/null

C400_BUILD="$(cargo run -p c400c -- --input tests/hola_mundo.c --output "$TMP_ROOT/QSYS/HELLOC")"
printf '%s\n' "$C400_BUILD"
grep -q "Tipificación ZFS completada" <<<"$C400_BUILD"

C400_RUN="$("$TMP_ROOT/QSYS/HELLOC")"
printf '%s\n' "$C400_RUN"
grep -q "Hola Mundo desde Linux/400 (C/400)" <<<"$C400_RUN"

CLC_BUILD="$(cargo run -p clc -- --input tests/prueba.clp --output "$TMP_ROOT/QSYS/HELLOCL")"
printf '%s\n' "$CLC_BUILD"
grep -q "Objeto nativo L400 creado" <<<"$CLC_BUILD"

CLC_RUN="$("$TMP_ROOT/QSYS/HELLOCL")"
printf '%s\n' "$CLC_RUN"
grep -q "\[clc\] Executing CL stub compiled from tests/prueba.clp" <<<"$CLC_RUN"
grep -q "Hola desde Control Language" <<<"$CLC_RUN"
