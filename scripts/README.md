# scripts

## Objetivo

`scripts` contiene automatizacion de build, instalacion, runtime y pruebas. Su objetivo es convertir los crates de Linux/400 en un sistema arrancable/instalable, validar demos V1, ejecutar gates de release y dar herramientas de soporte como migracion, upgrade check y reportes.

Las subcarpetas principales son:

- `build/`: construccion de userspace, rootfs, initramfs, kernel, ISO y release candidates.
- `runtime/`: instalador, sesion, autologin, migracion, upgrade check y soporte.
- `test/`: smoke tests, release gates, backup/restore, metadata upgrade y QEMU install.

## Nivel de avance

Estado: **medio**.

Ya hay scripts para ISO live/install, instalacion a disco, QEMU smoke, backup/restore por `rsync -aX`, migracion de metadata, support report y gates de release.

Para plena funcionalidad faltan:

- flujo formal de PTFs con paquete, precheck, apply, rollback y auditoria;
- comandos/pantallas de backup y restore integrados al operador, no solo recetas shell;
- hardening de instalacion para mas discos, errores y upgrades;
- cobertura automatizada de instalacion persistente en mas perfiles;
- documentacion operacional para diagnostico y recuperacion.

## Gate de release

Gate minimo:

```bash
./scripts/test/test_release_rc.sh
```

Ese gate ejecuta tests de `l400`, `clc` y `os400-tui`, build de userspace, build eBPF opcional y smoke tests de objetos, toolchain, workloads, loader, backup/restore, migracion y support profile.

Para validar instalacion y persistencia:

```bash
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

El build formal de RC:

```bash
./scripts/build/build_release_rc.sh
```

deja evidencia en `output/rc-evidence/<VERSION>/`: logs, entorno de reproduccion, support profile, artefactos y hashes.

## Perfiles de plataforma

| Modo | Requisitos | Garantia operativa |
| --- | --- | --- |
| `dev` | Linux comun, Rust y xattrs; sin ZFS/BPF/cgroups obligatorios. | Desarrollo local, tests de runtime, compiladores, comandos y TUI. |
| `degraded` | `/l400` persistente con xattrs y entorno parcial. | Operacion basica con auditoria runtime; el sistema debe mostrar degradacion. |
| `full` | BPF LSM, BTF, cgroups v2, loader `full`, `/l400` persistente con xattrs y preferentemente ZFS `xattr=sa`. | Enforcement kernel y plataforma recomendada para instalacion estable. |

`l400-support-report --write` es la fuente operativa para clasificar el modo efectivo.

## Backup, restore y migracion

`/l400` contiene el estado del sistema. Toda copia debe preservar xattrs.

Backup con filesystem comun:

```bash
rsync -aX /l400/ /backup/l400/
```

Restore:

```bash
rsync -aX --delete /backup/l400/ /l400/
l400-bootstrap --quiet
l400-support-report --write
```

Backup con `tar`:

```bash
tar --xattrs --xattrs-include='user.*' -cpf l400-backup.tar /l400
```

Restore con `tar`:

```bash
rm -rf /l400
mkdir -p /l400
tar --xattrs --xattrs-include='user.*' -xpf l400-backup.tar -C /
l400-bootstrap --quiet
l400-support-report --write
```

ZFS recomendado para sistemas instalados:

```bash
zfs snapshot pool/linux400@pre-upgrade
zfs send pool/linux400@pre-upgrade > linux400-pre-upgrade.zfs
zfs rollback pool/linux400@pre-upgrade
```

Despues de actualizar binarios o instalacion:

```bash
l400-bootstrap --quiet
l400-support-report --write
l400-upgrade-check
WRKOBJ LIB(QSYS)
WRKOBJ LIB(QGPL)
```

`l400-bootstrap` debe ser idempotente: puede crear objetos base faltantes, pero no debe borrar bibliotecas ni miembros de usuario. Downgrade de metadata no esta soportado; si `.metadata-version` es mayor que la version objetivo, `l400-migrate` debe fallar y el operador debe restaurar un backup/snapshot anterior.

## Instalacion y rescue

El camino normal es arrancar la ISO live/install y usar el instalador textual, que enumera discos, exige confirmacion `INSTALL`, crea particion EFI/root, copia el sistema e instala arranque UEFI.

La ISO incluye modo rescue con `l400.rescue=1`. Rescue abre shell de soporte para montar/restaurar `/l400`, ejecutar `l400-upgrade-check`, copiar backups y recuperar un sistema que no completa el arranque normal. No ejecuta downgrade automatico de metadata.
