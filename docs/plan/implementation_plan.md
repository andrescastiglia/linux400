# Plan de implementacion: Linux/400 siguiente nivel

Este plan convierte la vision de `docs/KERNEL.md` en una hoja de ruta ejecutable sobre el estado actual descrito en `docs/PROJECT.md`.

El objetivo de esta etapa no es agregar comandos sueltos. Es elevar Linux/400 desde una base funcional a un sistema operable, instalable y administrable con una experiencia coherente tipo OS/400: menu primero, objetos primero, jobs visibles, seguridad auditable y persistencia confiable.

## Principios de ejecucion

- Cada fase debe dejar una mejora usable desde la TUI o desde comandos Linux/400, no solo una API interna.
- Cada comando nuevo o ampliado debe tener salida batch, ruta TUI y tests/smoke.
- Cada cambio de objeto debe actualizar contrato, metadatos, comandos, TUI y documentacion.
- Cada feature debe degradar explicitamente en entornos sin BPF, ZFS, cgroups o privilegios.
- `./scripts/test/test_release_rc.sh` debe seguir siendo el gate minimo local; QEMU install smoke debe ser el gate de RC.

## Milestone 1: Instalacion persistente verificable

Estado: **finalizado en esta iteracion**.

**Objetivo:** demostrar que Linux/400 instalado conserva estado real de usuario, no solo objetos base.

Trabajo:

- [x] Extender `scripts/test/test_e2e_install_qemu.sh` para crear antes del reboot:
  - biblioteca de usuario;
  - source member CL;
  - PF con registros;
  - DTAQ con mensaje;
  - autorizacion modificada.
- [x] Validar tras reboot desde la VM instalada:
  - `WRKOBJ` ve la biblioteca/objetos;
  - `WRKMBRPDM FILE(QGPL/QCLSRC)` ve miembros base;
  - `DSPPFM` muestra registros persistidos;
  - `DSPDTAQ` muestra mensajes persistidos;
  - `l400-support-report --write` reporta backend persistente.
- [x] Agregar modo rapido de QEMU smoke que reutilice ISO existente cuando `ISO_PATH` esta definido.
- [x] Documentar rollback/backup/restore de `/l400` con `rsync -aX`, `tar --xattrs` y ZFS snapshot.

Archivos probables:

- `scripts/test/test_e2e_install_qemu.sh`
- `scripts/runtime/install_linux400.sh`
- `scripts/runtime/l400-support-report.sh`
- `docs/release_platforms.md`

Criterio de cierre:

- `RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh` valida datos de usuario tras reboot.
- Un fallo de persistencia produce error claro y log accionable.

## Milestone 2: Pantallas dedicadas para administracion

Estado: **finalizado en esta iteracion**.

**Objetivo:** reducir dependencia de `SystemPanel` para operaciones frecuentes y hacer que la TUI sea la interfaz primaria real.

Trabajo:

- [x] Crear pantalla `ObjectDetail` para `DSPOBJD` con:
  - tipo, atributo, owner, owner UID, texto;
  - autorizaciones;
  - toolchain/signature si aplica;
  - acciones por opcion: autorizaciones, borrar, copiar, cambiar texto.
- [x] Crear pantalla `UserProfiles` para `WRKUSRPRF`:
  - listar perfiles;
  - crear perfil;
  - desactivar perfil;
  - ver detalle.
- [x] Crear pantalla `PolicyAudit`:
  - `DSPPOLICY`;
  - `DSPAUD`;
  - filtros por evento, usuario y objeto.
- [x] Crear pantalla `SpoolOutq` minima:
  - `WRKSPLF`;
  - `WRKOUTQ`;
  - visualizar spool text.
- [x] Normalizar mensajes de estado y confirmaciones visuales en todas las pantallas destructivas.

Archivos probables:

- `os400-tui/src/screens/*`
- `os400-tui/src/app.rs`
- `os400-tui/src/screens/mod.rs`
- `libl400/src/ffi_commands.rs`
- `libl400/src/bin/l400cmd.rs`

Criterio de cierre:

- Un operador puede administrar objetos, usuarios, politica y spool sin salir de la TUI.
- Las acciones destructivas requieren confirmacion visual o `CONFIRM(*YES)`.

## Milestone 3: Objeto `*CMD` y prompt de comandos real

Estado: **finalizado base**. Queda ampliar el catalogo de metadata para todos los comandos, pero el flujo `*CMD` existe y es operable.

**Objetivo:** que los comandos sean objetos describibles/promptables, no solo strings en el dispatcher.

Trabajo:

- [x] Definir metadata de comando:
  - nombre;
  - texto;
  - parametros;
  - tipo de dato;
  - requerido/opcional;
  - valores permitidos;
  - default;
  - autoridad requerida.
- [x] Catalogar comandos base como `*CMD` durante `l400-bootstrap`.
- [x] Reemplazar templates hardcodeados de `F4` por lectura de metadata `*CMD`.
- [x] Agregar comandos:
  - `DSPCMD`;
  - `WRKCMD`;
  - `CRTCMD` minimo para registrar comandos internos.
- [x] Conectar metadata de `*CMD` con `l400cmd` para validar parametros antes de ejecutar.

Archivos probables:

- `libl400/src/bootstrap.rs`
- nuevo modulo `libl400/src/cmd.rs`
- `libl400/src/bin/l400cmd.rs`
- `os400-tui/src/screens/cmd_line.rs`
- `docs/object_policy.md`

Criterio de cierre:

- `F4` muestra parametros desde objetos `*CMD`.
- `DSPCMD CMD(WRKOBJ)` describe parametros y autoridad.
- Un parametro invalido falla con mensaje formal antes de llegar al handler.

## Milestone 4: CPF y estado formal de comandos

Estado: **finalizado en esta iteracion**.

**Objetivo:** hacer que errores y `MONMSG` tengan semantica consistente en runtime, batch y TUI.

Trabajo:

- [x] Definir estructura `CommandStatus` con:
  - codigo CPF;
  - severidad;
  - mensaje corto;
  - detalle;
  - objeto relacionado.
- [x] Crear catalogo inicial de CPF Linux/400:
  - objeto no encontrado;
  - autoridad insuficiente;
  - tipo incorrecto;
  - parametro invalido;
  - comando fallido;
  - storage/backend no disponible.
- [x] Migrar comandos sensibles para setear `CommandStatus`, no solo imprimir texto.
- [x] Extender `MONMSG`:
  - genericidad (`CPF0000`);
  - rangos;
  - ultimo codigo por comando;
  - limpieza de estado despues de capturar.
- [x] Mostrar CPF en TUI y auditoria.

Archivos probables:

- nuevo modulo `libl400/src/status.rs`
- `libl400/src/ffi.rs`
- `libl400/src/ffi_commands.rs`
- `cl_compiler/clc/src/compiler.rs`
- `os400-tui/src/*`

Criterio de cierre:

- Un CL puede capturar un error real con `MONMSG MSGID(CPFxxxx)`.
- La TUI muestra el mismo codigo CPF que queda auditado.

## Milestone 5: Seguridad unificada runtime/eBPF

Estado: **finalizado en esta iteracion**.

**Objetivo:** alinear autorizaciones de runtime con enforcement kernel y hacerlo visible.

Trabajo:

- [x] Definir representacion unica de identidad:
  - perfil Linux/400;
  - UID Linux;
  - owner logico;
  - grupos.
- [x] Extender `user.l400.auth` o mover a manifest versionado si xattr plano queda corto.
- [x] Aplicar autorizaciones en `file_open` para objetos catalogados, no solo exec de `*PGM`.
- [x] Mantener modo `degraded` con runtime-only enforcement equivalente.
- [x] Auditar denegados de runtime y eBPF con formato comun.
- [x] Agregar tests e2e:
  - `*PUBLIC:*EXCLUDE`;
  - owner permitido;
  - usuario/grupo permitido;
  - tipo incorrecto;
  - firma/toolchain invalida.

Archivos probables:

- `libl400/src/auth.rs`
- `libl400/src/audit.rs`
- `l400-ebpf-common/src/lib.rs`
- `l400-ebpf/src/main.rs`
- `l400-loader/src/main.rs`
- `docs/object_policy.md`

Criterio de cierre:

- `CHKOBJAUT`, `CALL`, TUI y eBPF toman decisiones equivalentes para los casos cubiertos.
- `DSPAUD` muestra denegados con usuario, objeto, operacion y fuente (`runtime`/`ebpf`).

## Milestone 6: PF/LF/SQL de operacion real

Estado: **finalizado en esta iteracion**.

**Objetivo:** pasar de modelo `KEY/DATA` extendido a archivos utiles para aplicaciones administrativas.

Trabajo:

- [x] Validar escritura por schema:
  - `CHAR`;
  - `NUM`;
  - longitud;
  - claves requeridas.
- [x] Soportar claves compuestas en PF/LF.
- [x] Implementar LF con select/omit minimo.
- [x] Completar comandos de miembros:
  - listar miembros;
  - borrar miembro con confirmacion;
  - copiar miembro;
  - cambiar texto.
- [x] Mejorar `STRSQL`:
  - parser mas completo;
  - errores CPF;
  - paginacion vertical/horizontal;
  - `CREATE INDEX` como LF.
- [x] Agregar demo de aplicacion simple sobre PF/LF/SQL desde TUI.

Archivos probables:

- `libl400/src/db.rs`
- `libl400/src/ffi_commands.rs`
- `os400-tui/src/screens/str_sql.rs`
- `os400-tui/src/screens/object_browser.rs`
- `examples/`

Criterio de cierre:

- PF/LF mantienen integridad tras insert/update/delete repetidos.
- Una demo crea PF/LF, carga datos, consulta por SQL y navega resultados desde TUI.

## Milestone 7: Work management con colas reales

Estado: **finalizado en esta iteracion**.

**Objetivo:** que `SBMJOB` y `WRKACTJOB` representen un modelo de trabajo Linux/400, no solo procesos sueltos.

Trabajo:

- [x] Crear objetos/configuracion de job queue (`JOBQ`) minima.
- [x] Implementar dispatcher batch:
  - encolar;
  - tomar job;
  - ejecutar;
  - actualizar estado;
  - persistir log.
- [x] Agregar comandos:
  - `WRKJOBQ`;
  - `HLDJOB`;
  - `RLSJOB`;
  - `ENDJOB` como comando formal.
- [x] Agregar subsistemas configurables sobre `QINTER`/`QBATCH`.
- [x] Exponer limites cgroup por subsistema/perfil.
- [x] Mostrar logs de job desde TUI.

Archivos probables:

- `libl400/src/cgroup.rs`
- nuevo modulo `libl400/src/jobq.rs`
- `libl400/src/bin/sbmjob.rs`
- `os400-tui/src/screens/work_mgmt.rs`

Criterio de cierre:

- `SBMJOB` deja un job en cola antes de ejecutar.
- `WRKACTJOB` y `WRKJOBQ` permiten diagnosticar estado y log sin shell.

## Milestone 8: `*OUTQ` y spool

Estado: **finalizado en esta iteracion**.

**Objetivo:** completar el camino de salida operativo para programas y jobs.

Trabajo:

- [x] Definir objeto `*OUTQ` con backend persistente.
- [x] Crear spool files con metadata:
  - job;
  - usuario;
  - programa/comando;
  - fecha;
  - estado;
  - output queue.
- [x] Redirigir salida batch a spool opcionalmente.
- [x] Implementar comandos:
  - `CRTOUTQ`;
  - `DLTOUTQ`;
  - `WRKOUTQ`;
  - `WRKSPLF`;
  - `DSPSPLF`;
  - `DLTSPLF`.
- [x] Crear pantalla TUI para spool/output queues.

Archivos probables:

- nuevo modulo `libl400/src/spool.rs`
- `libl400/src/ffi_commands.rs`
- `scripts/build/build_userspace.sh`
- `os400-tui/src/screens/*`

Criterio de cierre:

- Un job batch genera spool visible por `WRKSPLF`.
- El operador puede ver y borrar spool desde TUI.

## Milestone 9: Release, upgrade y soporte de plataforma

Estado: **finalizado en esta iteracion**.

**Objetivo:** convertir el release en un proceso repetible y diagnosticable.

Trabajo:

- [x] Ejecutar QEMU smoke en CI o runner dedicado.
- [x] Separar artefactos por perfil:
  - dev;
  - degraded;
  - full.
- [x] Agregar `l400-upgrade-check`:
  - version de metadata;
  - backup recomendado;
  - xattrs presentes;
  - backend persistente;
  - compatibilidad de kernel.
- [x] Agregar `l400-migrate` para cambios versionados de `/l400`.
- [x] Publicar matriz de soporte generada desde `l400-support-report`.
- [x] Agregar test de restore desde backup.

Archivos probables:

- `scripts/test/test_release_rc.sh`
- `scripts/test/test_e2e_install_qemu.sh`
- `scripts/runtime/l400-support-report.sh`
- `scripts/build/*`
- `docs/release_platforms.md`

Criterio de cierre:

- Un RC produce evidencia: tests cargo, smoke, userspace, eBPF si aplica, QEMU install, persistencia, support profile.
- Un usuario puede actualizar sin perder `/l400`.

## Milestone 10: Pulido OS/400-style

Estado: **finalizado en esta iteracion**.

**Objetivo:** hacer que el sistema se sienta consistente y operable, no una coleccion de demos.

Trabajo:

- [x] Unificar textos, errores y encabezados de pantallas.
- [x] Agregar ayuda contextual por comando/pantalla.
- [x] Agregar busqueda/filtros comunes en listas.
- [x] Agregar convenciones de opciones por fila:
  - `2=Change`;
  - `3=Copy`;
  - `4=Delete/End`;
  - `5=Display`;
  - `8=Authorities`;
  - `9=Run/Work with`.
- [x] Agregar guia de operacion diaria.
- [x] Agregar demos guiadas desde menu principal.

Archivos probables:

- `os400-tui/src/style.rs`
- `os400-tui/src/widgets/*`
- `os400-tui/src/screens/*`
- `docs/cheetsheet.md`
- `examples/`

Criterio de cierre:

- Una persona nueva puede completar el ciclo definido en `KERNEL.md` sin leer codigo ni usar shell.

## Orden recomendado

1. **Milestone 1**: instala y conserva datos.
2. **Milestone 2**: TUI dedicada para administrar de verdad.
3. **Milestone 4**: CPF formal para errores y CL.
4. **Milestone 5**: seguridad unificada.
5. **Milestone 3**: `*CMD` como base de prompts y comandos ricos.
6. **Milestone 6**: datos PF/LF/SQL mas robustos.
7. **Milestone 7**: job queues y subsistemas.
8. **Milestone 8**: spool/output queues.
9. **Milestone 9**: release/upgrade.
10. **Milestone 10**: pulido de experiencia.

## Gates permanentes

Antes de cerrar cualquier milestone:

```bash
cargo fmt --all --check
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
cargo clippy -p l400 --all-targets -- -D warnings
./scripts/test/test_release_rc.sh
```

Para milestones que toquen instalacion, persistencia, release o plataforma:

```bash
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```
