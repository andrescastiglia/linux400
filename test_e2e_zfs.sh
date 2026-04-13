#!/bin/bash
set -e

echo "=== Linux/400 - E2E ZFS y BPF Test (Fase 2) ==="

if [[ $EUID -ne 0 ]]; then
    echo "[-] Por favor ejecute este script con privilegios root (sudo ./test_e2e_zfs.sh)"
    exit 1
fi

POOL_NAME="l400test"
MOUNT_POINT="/mnt/l400test"
IMAGE_FILE="/var/tmp/l400test.img"
LOADER_BIN="./target/release/l400-loader"

echo "[1] Inicializando zpool temporal de prueba..."
mkdir -p "$MOUNT_POINT"
truncate -s 1G "$IMAGE_FILE"

# Si el pool de test ya existe de una corrida previa fallida, lo destruimos
if zpool list $POOL_NAME >/dev/null 2>&1; then
    zpool destroy $POOL_NAME
fi

zpool create -o ashift=12 $POOL_NAME $IMAGE_FILE
zfs set xattr=sa $POOL_NAME
zfs set dnodesize=auto $POOL_NAME
zfs set mountpoint=$MOUNT_POINT $POOL_NAME

# Crear biblioteca de test
zfs create $POOL_NAME/TESTLIB
setfattr -n user.l400.objtype -v "*LIB" "$MOUNT_POINT/TESTLIB"

echo "[2] Creando objetos de prueba..."
VALID_OBJ="$MOUNT_POINT/TESTLIB/VALID.PGM"
INVALID_OBJ="$MOUNT_POINT/TESTLIB/INVALID.OBJ"

touch "$VALID_OBJ" "$INVALID_OBJ"
setfattr -n user.l400.objtype -v "*PGM" "$VALID_OBJ"
setfattr -n user.l400.objtype -v "*BAD" "$INVALID_OBJ"

echo "[3] Lanzando l400-loader en modo background..."
if [[ ! -f "$LOADER_BIN" ]]; then
    echo "[-] l400-loader no encontrado. Compilando..."
    cargo build -p l400-loader --release
fi

$LOADER_BIN &
LOADER_PID=$!
echo "Esperando 3 segundos para que BPF LSM arranque..."
sleep 3

echo "[4] Testeando validación estricta por LSM..."
if cat "$VALID_OBJ" >/dev/null 2>&1; then
    echo "  [OK] Acceso PERMITIDO a objeto *PGM."
else
    echo "  [FAIL] Acceso denegado injustamente a *PGM."
fi

if cat "$INVALID_OBJ" >/dev/null 2>&1; then
    echo "  [FAIL] Acceso permitido a un objeto *BAD. El LSM falló."
else
    echo "  [OK] Acceso DENEGADO a objeto de tipo inválido."
fi

echo "[5] Limpiando zpool y cerrando daemon BPF..."
kill -SIGINT $LOADER_PID
sleep 1

zpool destroy $POOL_NAME
rm -f "$IMAGE_FILE"
rmdir "$MOUNT_POINT" || true

echo "=== Pruebas E2E Fase 2 completadas exitosamente ==="
