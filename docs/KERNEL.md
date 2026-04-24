# Kernel y plataforma base de Linux/400

Linux/400 no necesita un fork permanente del kernel. La personalidad OS/400-style vive en userspace (`libl400`, `os400-tui`, compiladores y scripts de runtime) y usa el kernel como base de aislamiento, seguridad y arranque. El objetivo operativo es que el usuario entre a una pantalla de sign-on y a un menu verde, no a `bash`; el kernel debe hacer posible esa experiencia sin romper la compatibilidad normal de Linux.

Este documento separa lo que es requisito actual, lo que ya esta implementado en el repo y lo que queda como linea futura.

## Contrato actual

- El root logico de objetos es `L400_ROOT`, por defecto `/l400`.
- El tipo de objeto se define con `user.l400.objtype`; es la frontera autoritativa compartida por `libl400` y el eBPF LSM.
- Los metadatos complementarios usan xattrs como `user.l400.objattr`, `user.l400.text`, `user.l400.owner`, `user.l400.auth`, `user.l400.storage_backend`, `user.l400.record_len` y `user.l400.base_pf`.
- Los tipos validos salen de `l400-ebpf-common/src/lib.rs`: `*PGM`, `*FILE`, `*USRPRF`, `*LIB`, `*DTAQ`, `*CMD`, `*SRVPGM`, `*OUTQ`.
- El loader publica estado en `L400_RUN_DIR`, por defecto `/run/l400/loader-status`.
- La politica eBPF activa es `phase3-v1`.

## Requisitos de kernel

### Obligatorios para modo completo

| Area | Requisito | Uso en Linux/400 |
| --- | --- | --- |
| Kernel | Linux >= 6.11 recomendado | Base probada para BPF LSM y flujo live/install del proyecto. |
| BPF LSM | `CONFIG_BPF=y`, `CONFIG_BPF_SYSCALL=y`, `CONFIG_BPF_LSM=y`, `CONFIG_BPF_JIT=y` | Permite cargar los hooks de politica de objetos. |
| Orden LSM | `CONFIG_LSM` debe incluir `bpf` | Sin `bpf` en `/sys/kernel/security/lsm`, el loader no puede adjuntar politica. |
| BTF | `/sys/kernel/btf/vmlinux` presente, normalmente con `CONFIG_DEBUG_INFO_BTF=y` | Aya necesita BTF para cargar programas LSM. |
| cgroups v2 | `/sys/fs/cgroup/cgroup.controllers` presente | Separa `QINTER` y `QBATCH`, y alimenta la vista de trabajos. |
| xattrs | soporte `user.*` en el filesystem que contiene `/l400` | Persistencia del catalogo de objetos. |
| consola/TTY | VT, serial 8250 y UEFI en la ISO | Soporta el arranque live/install y el menu por consola. |

### Recomendados

| Area | Requisito | Uso |
| --- | --- | --- |
| ZFS | OpenZFS instalado, dataset para `/l400`, `xattr=sa` | Backend objetivo para objetos persistentes con metadatos eficientes. |
| LAM/TBI | `CONFIG_X86_64_LAM=y` en x86_64 o ABI tagged-address en AArch64 | Optimiza punteros etiquetados cuando el hardware lo permite. |
| overlay/squashfs/vfat/isofs | Modulos o built-ins | Arranque live ISO e instalador. |

## eBPF LSM implementado

El programa `l400-ebpf` implementa tres hooks:

- `file_open`: permite archivos sin etiqueta Linux/400, permite objetos con `user.l400.objtype` valido y deniega etiquetas desconocidas.
- `bprm_creds_from_file`: permite ejecucion nativa no catalogada, permite ejecucion de `*PGM` si tiene atributo de toolchain valido (`C` o `CL`) y deniega otros tipos.
- `bprm_check_security`: confirma la decision de ejecucion y registra estadisticas.

El loader `l400-loader` tiene tres modos:

- `full`: exige eBPF activo; falla si no puede resolver/cargar/adjuntar el bytecode.
- `degraded`: intenta activar enforcement y continua sin proteccion si falla.
- `dev`: tolerante para desarrollo local, especialmente cuando faltan BTF, hooks o el artefacto eBPF.

La TUI muestra el estado del loader desde `/run/l400/loader-status`, y `scripts/runtime/l400-support-report.sh` clasifica la plataforma como `full`, `degraded` o `dev`.

## cgroups y subsistemas

`libl400/src/cgroup.rs` implementa:

- deteccion de cgroups v2;
- creacion de `l400.slice/l400.qinter` y `l400.slice/l400.qbatch`;
- asignacion de procesos a workload interactivo o batch;
- registro de trabajos en `L400_RUN_DIR/jobs`;
- parametros base de CPU, IO, memoria y PIDs.

El fallo de cgroups no debe impedir el uso de la TUI. En hosts sin permisos o sin cgroup v2, el runtime degrada y conserva el registro de jobs cuando puede.

## Almacenamiento y xattrs

ZFS es el backend objetivo para `/l400`, pero el codigo actual no depende de ZFS para pruebas unitarias: usa xattrs POSIX y puede funcionar sobre filesystems que soporten `user.*`.

Estado actual:

- Bibliotecas `*LIB`: directorios catalogados con xattrs.
- Objetos simples `*PGM`, `*CMD`, `*USRPRF`, etc.: archivos o directorios catalogados.
- `*FILE` PF/LF y `*DTAQ`: `sled` por defecto.
- Berkeley DB: backend opcional con `--features berkeleydb` y `L400_STORAGE_BACKEND=berkeleydb`.
- Source members: archivos planos dentro del directorio de un source file `*FILE`.

Para una instalacion persistente real falta cerrar la provision de `/l400` como dataset durable. El initramfs actual puede montar `/l400` como `tmpfs` en modo live/installed, util para demos pero no suficiente como almacenamiento de objetos permanente.

## Punteros etiquetados

`libl400/src/lam.rs` ya contiene:

- deteccion de modo (`IntelLam48`, `ArmTbi`, `SoftwareMask`, `Unsupported`);
- activacion best-effort por plataforma;
- helpers `tag_pointer`, `untag_pointer`, `get_space_bits`;
- mapeo tipo de objeto -> tag numerico.

Esto es soporte de runtime, no una dependencia obligatoria para la primera experiencia OS/400-style. Si LAM/TBI no esta disponible, el proyecto cae a mascara por software.

## DAX y sched_ext

DAX y `sched_ext` no son requisitos de la version actual.

- DAX no encaja directamente con ZFS porque ZFS no expone DAX de forma nativa. Puede quedar como perfil empresarial futuro para objetos/caches especiales sobre XFS/ext4/fsdax.
- `sched_ext` puede ser util mas adelante para planificacion fina de `QINTER`/`QBATCH`, pero hoy cgroups v2 cubre la separacion minima.
- eBPF `struct_ops` no forma parte del contrato actual; no debe documentarse como dependencia para `*DTAQ` o perfiles.

## Checklist de plataforma

```bash
uname -r
cat /sys/kernel/security/lsm
test -f /sys/kernel/btf/vmlinux && echo BTF_OK
test -f /sys/fs/cgroup/cgroup.controllers && echo CGROUP_V2_OK
getfattr -n user.l400.objtype /l400/QSYS 2>/dev/null
zfs get xattr "$(df /l400 | awk 'NR==2 {print $1}')" 2>/dev/null
L400_RUN_DIR=/run/l400 l400-support-report --write
```

## Gaps de kernel/plataforma

1. Asegurar que `scripts/build/build_kernel.sh` genere BTF usable para Aya (`/sys/kernel/btf/vmlinux`).
2. Hacer que el instalador cree o detecte un backend persistente para `/l400` en vez de depender de `tmpfs`.
3. Validar `xattr=sa` cuando `/l400` esta en ZFS y degradar con mensaje claro si no lo esta.
4. Empaquetar y arrancar `l400-loader` como servicio supervisado en el sistema instalado.
5. Convertir `support-profile` en una vista TUI/CL visible para administracion.
