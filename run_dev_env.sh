#!/bin/bash
# Script para instanciar el ecosistema interactivo de desarrollo Linux/400
# - Verifica si el host posee Kernel >= 6.11
# - Crea y configura un Virtual ZFS Pool local
# - Inicia la imagen multi-arquitectura inyectando el kernel actual y el pool

set -e

echo "=== Verificando Requerimientos del Sistema ==="
# Extraer versión principal y secundaria del kernel (ej. 6.11)
KVER=$(uname -r | grep -o '^[0-9]\+\.[0-9]\+')

if awk "BEGIN {exit !($KVER < 6.11)}"; then
    echo "=========================================================="
    echo " ERROR CRITICO: Verificación de BPF fallida"
    echo " La versión actual de tu Kernel es $KVER."
    echo " El proyecto requiere estrictamente un Kernel >= 6.11"
    echo " para compilar y cargar kfuncs asíncronos (bindgen) de Aya."
    echo "=========================================================="
    exit 1
else
    echo "✅ Kernel $KVER detectado (Soportado)."
fi

echo "=== Configurando ZFS Data Set Local ==="
mkdir -p .docker
ABS_IMG_PATH="$(pwd)/.docker/linux400zfs.img"

if [ ! -f "$ABS_IMG_PATH" ]; then
    echo "--> Creando disco crudo (sparse) de 2GB para el Pool ZFS..."
    truncate -s 2G "$ABS_IMG_PATH"
fi

if ! zpool list linux400pool > /dev/null 2>&1; then
    echo "--> Creando nuevo zpool 'linux400pool' (Requiere privilegios de Superusuario)..."
    sudo zpool create linux400pool "$ABS_IMG_PATH"
    
    echo "--> Configurando metadatos de atributos extendidos (Crucial para eBPF)..."
    sudo zfs set xattr=sa linux400pool
    sudo zfs set dnodesize=auto linux400pool
    sudo zfs set compression=lz4 linux400pool
else
    echo "✅ El pool 'linux400pool' ya se encuentra instanciado."
fi

# Extraer el punto de montaje real del pool para el Docker
ZFS_MOUNT=$(zfs get -H -o value mountpoint linux400pool || sudo zfs get -H -o value mountpoint linux400pool)

echo "=== Iniciando Contenedor de Desarrollo Linux/400 ==="
echo "Montando Pool ZFS Local ($ZFS_MOUNT) en el directorio contenedor /data"

# Ejecutamos con privilegios para permitir hooks BPF y montamos ZFS
docker run --rm -it --privileged \
  -v /sys/kernel/debug:/sys/kernel/debug \
  -v "$(pwd)":/linux400 \
  -v "$ZFS_MOUNT":/data \
  acastiglia/linux400-build-env:latest
