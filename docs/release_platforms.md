# Release, plataformas y migracion de Linux/400

Este documento define el gate minimo de release y la matriz de soporte esperada. La idea es que un RC no sea solo una compilacion: debe demostrar que el entorno arranca, instala, conserva `/l400` y mantiene una experiencia operativa degradable.

## Gate de RC

El gate minimo es:

```bash
./scripts/test/test_release_rc.sh
```

Ese script ejecuta:

- `cargo test -p l400`;
- `cargo test -p clc`;
- `cargo test -p os400-tui`;
- build userspace con `scripts/build/build_userspace.sh`;
- build eBPF cuando existe toolchain compatible;
- smoke scripts de objetos, toolchain, workloads, loader y support profile.

Para validar instalacion y persistencia se debe ejecutar:

```bash
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

El build formal de RC (`scripts/build/build_release_rc.sh`) exige ese gate QEMU salvo que se use `RUN_RC_GATE=0` para builds internos no publicables.

Cada build formal deja evidencia reproducible en:

```bash
output/rc-evidence/<VERSION>/
```

Ese directorio contiene:

- `build.log`: log del pipeline de artefactos;
- `release-gate.log`: log del gate de RC;
- `release-gate.env`: que se probo, host, kernel, version de Rust/Cargo y comando de reproduccion;
- `support-profile`: capacidades activas segun `l400-support-report`;
- `artifacts/`: ISO, initramfs, kernel y EFI generados si existen;
- `SHA256SUMS`: hashes de los artefactos publicados.

## Matriz de soporte

| Modo | Requisitos | Garantia operativa |
| --- | --- | --- |
| `dev` | Linux comun, Rust, xattrs disponibles; sin requerir ZFS, BPF LSM ni cgroups. | Desarrollo local, tests de runtime, compiladores, comandos y TUI. La proteccion kernel no se considera activa. |
| `degraded` | `/l400` persistente con xattrs y cgroups v2 o entorno parcial; eBPF/BTF/ZFS pueden faltar. | Operacion basica, jobs best-effort, auditoria runtime y soporte instalable. El sistema debe mostrar el modo degradado explicitamente. |
| `full` | Kernel con BPF LSM habilitado, BTF disponible, loader en modo `full`, ZFS con `xattr=sa` para `/l400`. | Enforcement kernel de objetos, userspace completo, persistencia fuerte y soporte recomendado para instalacion estable. |

`l400-support-report --write` es la fuente operativa para clasificar el modo efectivo. El reporte debe incluir `effective_mode`, estado del loader, BPF/BTF/cgroups, backend de `/l400` y si el storage es persistente.

## Politica de `/l400`

`/l400` es el estado del sistema. Contiene bibliotecas, objetos, PF/LF/DTAQ, perfiles Linux/400 y logs estilo `QHST`.

Reglas de release:

- en modo live se permite `tmpfs`, siempre marcado como no persistente;
- en modo instalado `/l400` debe vivir en storage persistente y soportar xattrs;
- el camino recomendado es ZFS con `xattr=sa`;
- ext4/xfs son fallback aceptables en modo `degraded`;
- las copias y migraciones deben preservar xattrs.

## Upgrade y migracion

Antes de actualizar:

```bash
l400-support-report --write
WRKOBJ LIB(QSYS)
WRKOBJ LIB(QGPL)
```

Para backup en filesystem comun:

```bash
rsync -aX /l400/ /backup/l400/
```

Para restore desde ese backup:

```bash
rsync -aX --delete /backup/l400/ /l400/
l400-bootstrap --quiet
l400-support-report --write
```

Para backup con `tar`:

```bash
tar --xattrs --xattrs-include='user.*' -cpf l400-backup.tar /l400
```

Para restore con `tar`:

```bash
rm -rf /l400
mkdir -p /l400
tar --xattrs --xattrs-include='user.*' -xpf l400-backup.tar -C /
l400-bootstrap --quiet
l400-support-report --write
```

Para ZFS:

```bash
zfs snapshot pool/linux400@pre-upgrade
zfs send pool/linux400@pre-upgrade > linux400-pre-upgrade.zfs
```

Para rollback local de ZFS:

```bash
zfs rollback pool/linux400@pre-upgrade
l400-support-report --write
```

Despues de actualizar binarios o una instalacion:

```bash
l400-bootstrap --quiet
l400-support-report --write
WRKOBJ LIB(QSYS)
WRKOBJ LIB(QGPL)
WRKMBRPDM FILE(QGPL/QCLSRC)
```

`l400-bootstrap` debe ser idempotente: puede crear objetos base faltantes, pero no debe borrar bibliotecas ni miembros de usuario. Si una migracion cambia formato de metadatos, debe agregarse una fase explicita versionada antes de tocar datos existentes.

Downgrade de metadata no esta soportado. Si `.metadata-version` es mayor que la version objetivo, `l400-migrate` debe fallar y el operador debe restaurar un backup o snapshot tomado antes del upgrade.

Para validar una migracion local desde metadata anterior:

```bash
./scripts/test/test_l400_upgrade_metadata.sh
```

## Instalacion

El camino normal de instalacion es arrancar la ISO de RC y usar el instalador textual:

```bash
RUN_E2E_INSTALL=1 ./scripts/build/build_release_rc.sh
```

En consola interactiva, el instalador:

- enumera discos;
- exige confirmacion escribiendo `INSTALL`;
- crea particion EFI y root;
- copia el sistema;
- instala arranque UEFI;
- conserva `/l400` como estado persistente del sistema instalado.

Tras el primer boot instalado:

```bash
l400-support-report --write
l400-upgrade-check
WRKOBJ LIB(QSYS)
WRKOBJ LIB(QGPL)
```

## Modo Rescue

La ISO incluye una entrada `Linux/400 rescue`. Tambien puede activarse con el parametro de kernel:

```text
l400.rescue=1
```

El modo rescue abre shell de soporte en vez de iniciar la experiencia principal. Se usa para revisar discos, montar/restaurar `/l400`, ejecutar `l400-upgrade-check`, copiar backups y recuperar un sistema que no completa el arranque normal. No ejecuta downgrade automatico de metadata.
