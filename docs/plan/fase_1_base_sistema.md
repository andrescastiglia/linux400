# Fase 1: Base de Sistema

## Objetivo

Cerrar el sistema base para que Linux/400 pueda:

- bootear como live ISO
- instalarse en disco UEFI
- reiniciar al sistema instalado

## Alcance

- `scripts/build/build_alpine_base.sh`
- `scripts/build/build_initramfs.sh`
- `scripts/build/build_iso.sh`
- `scripts/build/build_distribution.sh`
- `scripts/runtime/install_linux400.sh`
- `scripts/test/test_e2e_install_qemu.sh`

## Trabajo

### 1. Live boot estable

- asegurar `switch_root` consistente
- mantener `overlayfs` operativo o degradar de forma explícita
- garantizar disponibilidad de assets de instalación desde el live

### 2. Instalación UEFI

- resolver el bloqueo actual de montaje EFI VFAT
- copiar correctamente kernel, initramfs y `BOOTX64.EFI`
- dejar `fstab` y bootloader correctos

### 3. Validación E2E

- automatizar `live -> install -> reboot -> boot desde disco`
- dejar logs claros para diagnóstico

## Bloqueador actual

El problema principal de esta fase es:

- el instalador ya particiona y formatea, pero falla al montar la partición EFI VFAT dentro del live

## Entregables

- ISO live reproducible
- instalación UEFI completa en QEMU
- prueba E2E automatizada en verde

## Criterio de aceptación

- `scripts/test/test_e2e_install_qemu.sh` termina sin errores
- el sistema instalado llega al primer boot desde qcow2
