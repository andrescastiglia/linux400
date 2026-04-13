# Linux/400 v1 RC

Esta guía fija cómo construir, instalar, arrancar y demostrar la **release candidate v1** de Linux/400.

## Plataforma oficial v1

La plataforma oficial de la RC v1 es:

| Área | Perfil oficial |
| --- | --- |
| Arquitectura | `x86_64` |
| Firmware | UEFI |
| Hipervisor validado | QEMU |
| Kernel objetivo | Linux `>= 6.11` |
| Metadata de objetos | xattrs `user.l400.*` |
| Storage v1 | `sled` para `*FILE`/`*DTAQ`, ZFS+xattrs como modo soportado de enforcement |
| Interfaz principal | `os400-tui` en `tty1` |
| Consola de recovery | `ttyS0` y modo `rescue` |

La v1 **no** exige cambios al kernel fuera de las capacidades ya descritas en `docs/KERNEL.md`; cuando el entorno no permite enforcement completo o separación estricta de workloads, Linux/400 entra en modos degradados explícitos en lugar de simular éxito.

## Modos operativos del loader

`l400-loader` soporta:

| Modo | Qué exige | Resultado si falla el hook |
| --- | --- | --- |
| `full` | bytecode eBPF, BTF y attach correctos | falla cerrado |
| `degraded` | intenta enforcement real | continúa sin protección activa |
| `dev` | feedback para desarrollo local | continúa sin protección activa |

Ejemplos:

```bash
cargo run -p l400-loader -- --mode full --once
cargo run -p l400-loader -- --mode degraded --once
cargo run -p l400-loader -- --mode dev --once
```

## Prerrequisitos

### Build host

- `cargo` y toolchain Rust del workspace
- `squashfs-tools`
- `grub-mkrescue`
- `grub-mkstandalone`
- `mtools`
- para smoke tests instalados: `expect`, `qemu-img`, `qemu-system-x86_64`, OVMF

### Runtime objetivo

- kernel `>= 6.11`
- UEFI
- `tty1` para experiencia principal
- `ttyS0` para depuración/recovery
- ZFS con `xattr=sa` para el modo soportado de enforcement de objetos
- BPF LSM para `full`

## Construcción de artefactos RC

Para construir la RC desde el pipeline actual:

```bash
./scripts/build/build_release_rc.sh
```

Eso genera:

- userspace
- rootfs Alpine
- initramfs live/install
- ISO híbrida
- resumen de artefactos RC

## Smoke tests de release

Smoke tests rápidos de RC:

```bash
./scripts/test/test_release_rc.sh
```

Incluye:

- demo de objetos
- demo de toolchain
- demo de workloads
- modos del loader

Para incluir además el smoke test de instalación QEMU:

```bash
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

## Instalación desde ISO

### Opción rápida: QEMU

```bash
./scripts/test/test_e2e_install_qemu.sh
```

Ese flujo construye la ISO si hace falta, arranca el live, instala en disco virtual y verifica el arranque instalado.

### Opción manual dentro del live

1. Arrancar la ISO en modo `Linux/400 Install`.
2. Entrar por consola serial o live shell.
3. Ejecutar:

```bash
install-linux400 /dev/vda
```

4. Apagar la VM.
5. Arrancar desde el disco instalado.

## Primer boot y operación

### Boot normal

- `tty1` entra al flujo Linux/400 y debe llegar a `os400-tui`.
- `ttyS0` queda como consola de depuración.

### Recovery

- la entrada `Linux/400 Rescue` abre shell sin ambigüedad.
- en recovery no se fuerza la TUI.

### TUI

La TUI es la shell principal de v1. Desde ella se pueden observar:

- catálogo de objetos cuando existe runtime real bajo `/l400`
- colas de datos
- trabajos/workloads visibles desde el registro de jobs
- línea de comandos para entrar a comandos de operación

## Uso del toolchain

### C/400

Ejemplo canónico:

```bash
c400c --input tests/hola_mundo.c --output /l400/QSYS/HELLOC
/l400/QSYS/HELLOC
```

### CL

Subset v1 soportado:

- `PGM`
- `SNDPGMMSG`
- `ENDPGM`

Ejemplo canónico:

```bash
clc --input tests/prueba.clp --output /l400/QSYS/HELLOCL
/l400/QSYS/HELLOCL
```

## Demo v1 recomendada

1. Construir RC: `./scripts/build/build_release_rc.sh`
2. Validar smoke tests: `./scripts/test/test_release_rc.sh`
3. Ejecutar instalación QEMU: `./scripts/test/test_e2e_install_qemu.sh`
4. En el sistema instalado, confirmar TUI en `tty1`
5. Compilar un `*PGM` con `c400c` o `clc`
6. Ejecutar demo de objetos:

```bash
cargo run -p l400 --example objects_v1_demo -- /tmp/l400-demo
```

7. Ejecutar demo de workloads:

```bash
./scripts/test/test_workload_demo.sh
```
