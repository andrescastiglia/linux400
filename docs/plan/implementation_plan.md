# Roadmap V1 de Linux/400

Este documento reemplaza el plan anterior de `docs/plan/` y redefine el trabajo hacia una **v1 operable** de Linux/400, usando como visión de producto [PROJECT.md](/home/user/Source/linux400/docs/PROJECT.md:1) y como restricción el estado real del repositorio y del pipeline live/install.

## Definición de V1

La v1 de Linux/400 queda definida como:

- Una distribución **live e instalable** que bootea en QEMU UEFI y en entorno controlado.
- Un sistema que arranca al flujo Linux/400, con **TUI como experiencia principal**.
- Una base de runtime que integra `libl400`, `os400-tui`, `clc`, `c400c`, `l400-loader` y el hook BPF.
- Un modelo de objetos usable con `*LIB`, `*PGM`, `*FILE`, `*DTAQ`, `*USRPRF`.
- Un backend de storage funcional para v1, con enforcement básico y subsistemas interactivo/lote observables.

No forma parte obligatoria de v1:

- fidelidad completa a SLS/TIMI
- soporte amplio de hardware fuera de QEMU/lab
- enforcement universal sobre todas las syscalls
- implementación definitiva de todas las ideas teóricas de `PROJECT.md`

## Principios de Planificación

1. Primero se estabiliza el **sistema booteable e instalable**.
2. Después se consolida la **experiencia Linux/400 visible al usuario**.
3. Luego se endurecen **runtime, objetos, storage y enforcement**.
4. Finalmente se empaqueta una **release candidate v1 reproducible**.

## Estado Actual Relevante

Hoy el repositorio ya tiene avances importantes:

- ISO live/install, initramfs, rootfs y scripts de build.
- TUI funcional y autologin para flujo Linux/400.
- Toolchain `clc` y `c400c`.
- `libl400`, `l400-loader`, `l400-ebpf`, `l400-ebpf-common`.
- Gestión de workloads con cgroups v2.
- Harness E2E de instalación UEFI sobre QEMU+qcow2.

Bloqueadores detectados en el relevamiento reciente:

- La instalación UEFI todavía no completa porque el live no logra montar la partición EFI VFAT dentro del entorno instalado.
- El pipeline compila el hook eBPF con fallback, pero no hay validación robusta de runtime del loader/hook en el sistema instalado.
- El modelo de objetos y storage existe, pero todavía no está cerrado como flujo v1 de punta a punta dentro del sistema instalado.

## Milestones

### M1. Sistema Base Cerrado

Objetivo:

- ISO live estable
- instalador UEFI funcionando
- reboot desde disco validado en QEMU

Salida esperada:

- `scripts/test/test_e2e_install_qemu.sh` pasa de punta a punta
- el sistema instalado arranca desde qcow2 sin intervención

### M2. Experiencia Linux/400 Operable

Objetivo:

- arranque normal a TUI
- consola de recovery separada
- sesión Linux/400 consistente en live e instalado

Salida esperada:

- `tty1` entra a `l400-session`
- `os400-tui` se convierte en flujo principal de operación

### M3. Runtime Integrado

Objetivo:

- toolchain y runtime funcionando dentro del sistema
- carga de componentes del proyecto validada en entorno real

Salida esperada:

- compilar y ejecutar programas desde el sistema Linux/400

### M4. Objetos + Storage + Enforcement V1

Objetivo:

- objetos tipados utilizables
- backend de storage v1 definido y documentado
- enforcement básico verificable

Salida esperada:

- demo funcional con `*LIB`, `*PGM`, `*FILE`, `*DTAQ`

### M5. Release Candidate V1

Objetivo:

- documentación, matriz de soporte, demo y criterios de aceptación cerrados

Salida esperada:

- release candidate reproducible en QEMU y entorno controlado

## Fases del Plan

- [fase_1_base_sistema.md](/home/user/Source/linux400/docs/plan/fase_1_base_sistema.md)
- [fase_2_experiencia_runtime.md](/home/user/Source/linux400/docs/plan/fase_2_experiencia_runtime.md)
- [fase_3_objetos_storage.md](/home/user/Source/linux400/docs/plan/fase_3_objetos_storage.md)
- [fase_4_toolchain_workloads.md](/home/user/Source/linux400/docs/plan/fase_4_toolchain_workloads.md)
- [fase_5_release_v1.md](/home/user/Source/linux400/docs/plan/fase_5_release_v1.md)

## Riesgos Principales

- Dependencia de kernel/módulos para `overlay`, `vfat`, `zfs`, `bpf`.
- Entorno Alpine mínimo sin `apk` en host, lo que obliga a empaquetado híbrido.
- Diferencia entre “compila” y “funciona en runtime” para loader/eBPF.
- Riesgo de perseguir fidelidad teórica antes de cerrar una base operable.

## Criterio de Cierre de V1

La v1 se considera alcanzada cuando:

1. La ISO live bootea en QEMU UEFI.
2. La instalación a disco completa y reinicia al sistema instalado.
3. El sistema entra al flujo Linux/400 con TUI.
4. `clc`, `c400c`, `libl400` y `os400-tui` funcionan dentro del sistema.
5. Existe una demo reproducible con objetos tipados y storage v1.
