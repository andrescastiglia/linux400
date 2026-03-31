#!/bin/bash
# Script para iniciar Buidx Multiarquitectura (amd64 / arm64)
# basado en la propuesta oficial del proyecto Linux/400

set -e

echo "=== Configurando Entorno QEMU Multiarch ==="
docker run --rm --privileged multiarch/qemu-user-static --reset -p yes

echo "=== Prueba de plataforma ARM64 vía QEMU ==="
docker run --rm -it --platform linux/arm64 ubuntu:22.04 uname -m

echo "=== Iniciando Entorno Buildx ==="
# Creamos el builder si no existe e instruimos usarlo directamente
docker buildx create --name linux400_builder --use || docker buildx use linux400_builder

echo "=== Compilando y empaquetando imagen Docker multi-arch ==="
# Compila para x86_64 (LAM) y ARM64 (TBI) simultaneamente
# Nota: Cambia el tag "acastiglia" a posteriori si requiere otro namespace oficial en Docker Hub

docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -t acastiglia/linux400-build-env:latest \
    --push .

echo "¡Construcción finalizada!"
echo "Puedes iniciar un contenedor de desarrollo local con:"
echo "docker run --rm -it --privileged -v \$(pwd):/linux400 acastiglia/linux400-build-env:latest"
