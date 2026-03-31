#!/bin/bash
set -e

echo "=== Linux/400 - E2E LSM Hook Test ==="

if [ "$EUID" -ne 0 ]; then
  echo "[-] Por favor ejecute este script con privilegios root (sudo ./test_e2e_bpf.sh)"
  exit 1
fi

TEST_FILE="/tmp/myfile.obj"
INVALID_FILE="/tmp/invalid.obj"

echo "[1] Creando archivos dummy..."
touch $TEST_FILE
touch $INVALID_FILE

echo "[2] Asignando atributos extendidos (Tipificacion L400/ZFS)..."
# Valido
setfattr -n user.l400.objtype -v "*PGM" $TEST_FILE
# Invalido
setfattr -n user.l400.objtype -v "*MAL" $INVALID_FILE

echo "[3] Probando acceso SIN el BPF hook habilitado..."
cat $TEST_FILE > /dev/null && echo "  -> Acceso a $TEST_FILE OK"
cat $INVALID_FILE > /dev/null && echo "  -> Acceso a $INVALID_FILE OK"

echo "[4] Iniciando BPF Loader en background..."
# Asumimos que el binario fue construido previomente via setup_env.sh
./l400-loader/target/release/l400-loader &
BPF_PID=$!

echo "Esperando 2 segundos para montaje del hook BPF..."
sleep 2

echo "[5] Probando acceso CON el BPF hook habilitado..."
if cat $TEST_FILE > /dev/null 2>&1; then
    echo "  -> [EXCELENTE] Acceso a $TEST_FILE Permitido (Firma *PGM detectada por circulo-0)"
else
    echo "  -> [ERROR] Fallo inesperado al acceder a $TEST_FILE"
fi

if cat $INVALID_FILE > /dev/null 2>&1; then
    echo "  -> [ERROR] BPF fallo en denegar el acceso a un archivo invalido!"
else
    echo "  -> [EXCELENTE] Acceso a $INVALID_FILE DENEGADO (-EACCES disparado por BPF LSM)"
fi

echo "[6] Limpiando entorno..."
kill -SIGINT $BPF_PID
rm -f $TEST_FILE $INVALID_FILE

echo "=== Pruebas E2E BPF completadas exitosamente ==="
