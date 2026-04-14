#!/bin/bash
set -e

echo "=== Linux/400 - E2E LSM Hook Test ==="

if [ "$EUID" -ne 0 ]; then
  echo "[-] Por favor ejecute este script con privilegios root (sudo ./test_e2e_bpf.sh)"
  exit 1
fi

TEST_FILE="/tmp/myfile.obj"
INVALID_FILE="/tmp/invalid.obj"
EXEC_OK="/tmp/l400_exec_ok"
EXEC_BAD="/tmp/l400_exec_bad"

echo "[1] Creando archivos dummy..."
touch $TEST_FILE
touch $INVALID_FILE
cp /bin/true "$EXEC_OK"
cp /bin/true "$EXEC_BAD"
chmod +x "$EXEC_OK" "$EXEC_BAD"

echo "[2] Asignando atributos extendidos (Tipificacion L400/ZFS)..."
# Valido
setfattr -n user.l400.objtype -v "*PGM" $TEST_FILE
# Invalido
setfattr -n user.l400.objtype -v "*MAL" $INVALID_FILE
setfattr -n user.l400.objtype -v "*PGM" $EXEC_OK
setfattr -n user.l400.objtype -v "*FILE" $EXEC_BAD

echo "[3] Probando acceso SIN el BPF hook habilitado..."
cat $TEST_FILE > /dev/null && echo "  -> Acceso a $TEST_FILE OK"
cat $INVALID_FILE > /dev/null && echo "  -> Acceso a $INVALID_FILE OK"

echo "[4] Iniciando BPF Loader en background..."
if [[ ! -x ./target/release/l400-loader ]]; then
    cargo build -p l400-loader --release
fi

./target/release/l400-loader &
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

echo "[6] Probando ejecución CON el BPF hook habilitado..."
if "$EXEC_OK" >/dev/null 2>&1; then
    echo "  -> [EXCELENTE] Ejecución de $EXEC_OK permitida como *PGM"
else
    echo "  -> [ERROR] Fallo inesperado al ejecutar $EXEC_OK"
fi

if "$EXEC_BAD" >/dev/null 2>&1; then
    echo "  -> [ERROR] BPF fallo en denegar ejecución de objeto no *PGM!"
else
    echo "  -> [EXCELENTE] Ejecución de $EXEC_BAD DENEGADA por política de *PGM"
fi

echo "[7] Limpiando entorno..."
kill -SIGINT $BPF_PID
rm -f $TEST_FILE $INVALID_FILE $EXEC_OK $EXEC_BAD

echo "=== Pruebas E2E BPF completadas exitosamente ==="
