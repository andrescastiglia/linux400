# Plan de implementacion: Linux/400 siguiente nivel

Fecha de corte: 2026-04-26.

Este plan reemplaza la hoja de ruta inicial. La base v1 ya existe: objetos,
comandos, TUI, PF/LF/DTAQ, CL/C toolchain, loader eBPF, instalador y smoke
tests. El siguiente nivel no es sumar comandos aislados; es convertir esa base
en un sistema coherente, verificable y operable como appliance Linux/400.

## Diagnostico actual

Fortalezas ya disponibles:

- runtime de objetos con xattrs y tipos Linux/400;
- `l400cmd` con catalogo `*CMD` promptable y bootstrap desde `COMMAND_METADATA`;
- comandos base para objetos, usuarios, autoridad, PF/LF/DTAQ, jobs, spool y
  sistema;
- `PWRDWNSYS` con confirmacion, root, dry-run y fallback de apagado/reinicio;
- TUI con sign-on, menu, command line, objetos, jobs, DTAQ, PDM, SEU y SQL;
- CL compiler con `MONMSG` sobre CPF runtime;
- eBPF LSM para frontera de tipos y ejecucion `*PGM`;
- scripts de build, instalacion, QEMU smoke, backup/restore y release RC.

Brechas que frenan el salto de calidad:

- algunas rutas criticas todavia tienen comportamiento parcial o duplicado
  entre runtime, eBPF, TUI y docs;
- `user.l400.auth` runtime guarda perfiles (`QPGMR:*USE`) mientras la ruta eBPF
  espera entradas `UID:<uid>:*USE`;
- `ENDJOB` debe confirmar terminacion real antes de cambiar estado persistente;
- algunas pantallas ejecutan comandos con `split_whitespace`, rompiendo
  argumentos CL con comillas;
- el catalogo `*CMD` existe, pero la validacion formal aun convive con listas
  manuales en `l400cmd`;
- `*OUTQ` y `*SRVPGM` necesitan una definicion mas profunda de producto;
- los gates de release prueban bastante runtime, pero poco flujo interactivo
  end-to-end de TUI;
- la documentacion de estado y politica debe volver a sincronizarse con la
  implementacion reciente.

## Objetivo

Linux/400 debe permitir que una persona arranque o instale el sistema, entre
al menu principal y complete un ciclo operativo completo sin shell:

1. crear biblioteca y source file;
2. editar miembro CL;
3. compilarlo a `*PGM`;
4. ejecutar el programa con autoridad verificable;
5. enviar un job batch y revisar su spool/log;
6. crear PF/LF/DTAQ y operar datos;
7. administrar autorizaciones;
8. revisar auditoria y politica activa;
9. reiniciar y verificar persistencia;
10. apagar o reiniciar con `PWRDWNSYS` desde la interfaz Linux/400.

## Reglas de ejecucion

- Primero se corrigen inconsistencias P0/P1; despues se agrega alcance nuevo.
- Toda capacidad nueva debe tener comando, metadata `*CMD`, ruta TUI o salida
  batch, test y documentacion.
- Runtime, TUI, eBPF y docs deben compartir una fuente de verdad siempre que sea
  razonable.
- Los modos `dev`, `degraded` y `full` deben degradar explicitamente, no fallar
  con mensajes Linux crudos.
- `scripts/test/test_release_rc.sh` es el gate local minimo; QEMU install smoke
  es el gate de release candidate.

## Fase 0: estabilizacion inmediata

Estado: **finalizado para 0.2-pre**.

Objetivo: cerrar riesgos que pueden producir comportamiento incorrecto aunque
la demo funcione.

Trabajo:

- [x] Corregir paridad de autoridad runtime/eBPF:
  - mantener grants por perfil en formato `USER:*AUTH`;
  - agregar entradas espejo `UID:<uid>:*AUTH` cuando el perfil `*USRPRF` se puede resolver;
  - mantener compatibilidad con entradas `UID:<uid>:*AUTH` si ya existen;
  - agregar test para `*PUBLIC:*EXCLUDE` + `QPGMR:*USE`.
- [x] Corregir `ENDJOB`:
  - no marcar `Failed`/terminado inmediatamente despues de `SIGTERM`;
  - esperar salida real con timeout;
  - distinguir estados `ENDING`, `ENDED`, `FAILED` y `KILLED`.
- [x] Unificar tokenizacion CL:
  - reemplazar `split_whitespace` en `SystemPanel`;
  - preservar `CMD('MY CMD')`, textos con espacios y comillas anidadas;
  - reutilizar el tokenizer de `CommandLine` en paneles de ejecucion.
- [x] Sincronizar documentos de estado:
  - `docs/PROJECT.md`;
  - `docs/object_policy.md`;
  - este plan.

Criterio de cierre:

- [x] Los tres bugs P0/P1 anteriores tienen test automatizado.
- [x] `cargo test -p l400` y `cargo test -p os400-tui` pasan.
- [x] La documentacion ya no describe `*CMD`/`*OUTQ` como futuro si el flujo existe.

## Fase 1: plataforma de comandos como fuente de verdad

Estado: **planificado**.

Objetivo: que `*CMD` gobierne prompt, validacion, ayuda, dispatch y bootstrap.

Trabajo:

- [ ] Definir version de schema para metadata `*CMD`.
- [ ] Mover validacion de parametros de listas manuales de `l400cmd` a
  `COMMAND_METADATA`.
- [ ] Generar o verificar `COMMAND_BINARIES` contra `COMMAND_METADATA`.
- [ ] Hacer que `DSPCMD` muestre:
  - parametros;
  - defaults;
  - autoridad;
  - ejemplos;
  - estado soportado (`stable`, `experimental`, `admin-only`).
- [ ] Agregar `WRKCMD` con filtro por nombre, autoridad y estado.
- [ ] Agregar tests que fallen si un comando despachado no tiene metadata o si
  metadata acepta parametros que el handler no entiende.

Criterio de cierre:

- Un parametro invalido falla antes del handler con CPF formal.
- F4, `DSPCMD`, `l400cmd` y bootstrap consumen la misma metadata.
- Agregar un comando nuevo requiere tocar una sola fuente declarativa principal.

## Fase 2: seguridad y politica

Estado: **planificado**.

Objetivo: que runtime y kernel tomen decisiones equivalentes y auditables.

Trabajo:

- [ ] Crear `user.l400.auth.manifest` con:
  - perfil;
  - UID;
  - grupos;
  - autoridad;
  - origen (`explicit`, `public`, `owner`);
  - version.
- [ ] Actualizar `GRTOBJAUT`/`RVKOBJAUT` para mantener manifest y formato plano.
- [ ] Aplicar autoridad en `file_open` para operaciones sensibles, no solo exec.
- [ ] Publicar estado de politica desde loader:
  - version runtime;
  - version eBPF;
  - modo efectivo;
  - brechas conocidas.
- [ ] Ampliar `DSPPOLICY` para mostrar diferencias entre runtime y eBPF.
- [ ] Agregar pruebas:
  - owner permitido;
  - usuario explicito permitido;
  - grupo permitido;
  - `*PUBLIC:*EXCLUDE` denegado;
  - tipo incorrecto;
  - modo `degraded` runtime-only.

Criterio de cierre:

- `CALL`, `CHKOBJAUT`, TUI y eBPF coinciden en los casos cubiertos.
- Todo denegado registra usuario, objeto, operacion, fuente y CPF si aplica.

## Fase 3: work management confiable

Estado: **planificado**.

Objetivo: que jobs Linux/400 sean unidades operativas persistentes, no solo
procesos observados.

Trabajo:

- [ ] Consolidar registro de jobs con transiciones validas de estado.
- [ ] Implementar `ENDING`, `ENDED`, `KILLED`, `FAILED` y razon de salida.
- [ ] Hacer `SBMJOB` transaccional:
  - crear job en `JOBQ`;
  - persistir comando y contexto;
  - ejecutar;
  - capturar stdout/stderr;
  - emitir spool/log.
- [ ] Agregar `WRKJOB` formal para detalle, log, spool y entorno.
- [ ] Exponer acciones TUI:
  - hold;
  - release;
  - end controlled;
  - end immediate;
  - display log.
- [ ] Hacer cgroups visibles como capacidad: activo, degradado o no disponible.

Criterio de cierre:

- Un job no puede figurar como terminado mientras su PID siga vivo.
- `WRKACTJOB`, `WRKJOBQ`, `WRKJOB` y TUI muestran estados consistentes.
- Un job batch genera log/spool recuperable tras reinicio si el backend lo
  permite.

## Fase 4: datos operativos y recuperacion

Estado: **planificado**.

Objetivo: subir PF/LF/DTAQ de demo robusta a almacenamiento administrativo
confiable.

Trabajo:

- [ ] Agregar comando `CHKOBJINT` o similar para verificar integridad de objetos:
  - xattrs requeridos;
  - backend presente;
  - schema PF;
  - LF apuntando a PF valido;
  - miembros source.
- [ ] Agregar repair best-effort para metadata recuperable.
- [ ] Versionar metadata de PF/LF/DTAQ.
- [ ] Mejorar backup/restore con validacion posterior automatica.
- [ ] Hacer `STRSQL` emitir CPF para errores parseables por `MONMSG`.
- [ ] Agregar demo administrativa completa:
  - PF de clientes;
  - LF por clave;
  - SQL query/update;
  - DTAQ de notificacion;
  - spool de reporte.

Criterio de cierre:

- Backup/restore conserva xattrs y datos y lo verifica con comandos Linux/400.
- PF/LF sobreviven ciclos insert/update/delete con indices coherentes.

## Fase 5: spool y `*OUTQ` de producto

Estado: **planificado**.

Objetivo: que salida batch y reportes tengan un ciclo de vida OS/400-style.

Trabajo:

- [ ] Definir metadata completa de `*OUTQ`.
- [ ] Normalizar spool file:
  - id;
  - job;
  - usuario;
  - comando/programa;
  - OUTQ;
  - estado;
  - timestamps;
  - contenido;
  - retencion.
- [ ] Conectar stdout/stderr de `SBMJOB` a spool por defecto.
- [ ] Permitir mover/cambiar estado de spool (`READY`, `HELD`, `SAVED`).
- [ ] Completar TUI de `WRKSPLF`/`WRKOUTQ` con filtros y acciones.

Criterio de cierre:

- Todo job batch produce una salida visible por `WRKSPLF`.
- El operador puede ver, retener, borrar y diagnosticar spool sin shell.

## Fase 6: experiencia TUI end-to-end

Estado: **planificado**.

Objetivo: validar la consola como interfaz primaria real.

Trabajo:

- [ ] Crear suite de smoke interactivo con terminal automatizado:
  - sign-on;
  - menu;
  - command line;
  - F4 prompt;
  - WRKOBJ;
  - STRPDM/SEU;
  - STRSQL;
  - WRKACTJOB;
  - WRKSPLF.
- [ ] Unificar ayuda contextual por pantalla desde metadata `*CMD`.
- [ ] Eliminar textos demo silenciosos cuando falta runtime real.
- [ ] Agregar barra de mensajes CPF comun.
- [ ] Revisar accesibilidad terminal:
  - ancho 80/132;
  - scroll;
  - foco;
  - errores largos.

Criterio de cierre:

- Un flujo "crear, editar, compilar, ejecutar, enviar batch, ver spool" corre en
  TUI bajo automatizacion.
- Las acciones destructivas siempre tienen confirmacion visual o `CONFIRM(*YES)`.

## Fase 7: toolchain y ciclo de desarrollo

Estado: **planificado**.

Objetivo: que desarrollar dentro de Linux/400 sea una experiencia completa.

Trabajo:

- [ ] Mejorar diagnosticos de `clc`:
  - linea/columna;
  - CPF asociado;
  - spool de compilacion.
- [ ] Extender CL prioritario:
  - parametros;
  - variables numericas;
  - `DOWHILE`/`DOUNTIL` si aplica;
  - `SNDPGMMSG`;
  - comandos de job/spool.
- [ ] Hacer `CRTCLPGM` y `CRTPGM` visibles desde PDM/SEU.
- [ ] Definir contrato de `*SRVPGM`:
  - si es objetivo o backlog;
  - metadata;
  - autoridad;
  - linking/carga.
- [ ] Reemplazar marcas simples de toolchain por manifest verificable.

Criterio de cierre:

- Un usuario crea un miembro CL, compila, ve errores o spool de exito y ejecuta
  el `*PGM` desde TUI.
- `CALL` rechaza programas sin manifest de toolchain valido.

## Fase 8: release, instalacion y soporte

Estado: **planificado**.

Objetivo: que cada RC produzca evidencia reproducible.

Trabajo:

- [ ] Dividir gates:
  - `dev-fast`;
  - `userspace`;
  - `kernel-optional`;
  - `install-qemu`;
  - `upgrade-restore`.
- [ ] Publicar artefactos y logs por RC.
- [ ] Hacer que `l400-support-report --write` genere un perfil adjuntable a
  issues.
- [ ] Agregar test de upgrade desde metadata version anterior.
- [ ] Documentar procedimientos:
  - instalacion;
  - backup;
  - restore;
  - downgrade no soportado;
  - modo rescue.

Criterio de cierre:

- Un RC responde: que se probo, en que host, que capacidades quedaron activas y
  como reproducir.
- QEMU install smoke valida persistencia de objetos de usuario tras reinicio.

## Backlog deliberado

No bloquea:

- compatibilidad binaria IBM i;
- emulacion 5250 completa;
- EBCDIC completo;
- TIMI;
- fork de kernel;
- implementacion amplia de todos los comandos historicos.

## Orden recomendado de PRs

1. Fix eBPF/runtime auth para `USER:*AUTH` y test `*PUBLIC:*EXCLUDE`.
2. Fix `ENDJOB` con espera real y estados `ENDING`/`ENDED`.
3. Tokenizer CL compartido para TUI y comandos con argumentos quoted.
4. Sincronizacion docs `PROJECT`/`object_policy` con estado actual.
5. Validacion `l400cmd` desde `COMMAND_METADATA`.
6. `WRKJOB` formal y job logs desde TUI.
7. Spool default para `SBMJOB`.
8. `CHKOBJINT` para integridad de objetos.
9. Smoke interactivo TUI automatizado.
10. RC evidence bundle en `test_release_rc.sh`.

## Gates permanentes

Gate rapido local:

```bash
cargo fmt --all --check
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
```

Gate de calidad antes de cerrar una fase:

```bash
cargo clippy -p l400 --all-targets -- -D warnings
./scripts/test/test_release_rc.sh
```

Gate de release candidate:

```bash
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

Smoke seguro para apagado/reinicio:

```bash
L400_PWRDWNSYS_DRY_RUN=1 cargo run -p l400 --bin l400cmd -- \
  PWRDWNSYS 'OPTION(*RESTART)' 'CONFIRM(*YES)'
```

## Definicion de "siguiente nivel"

El proyecto llega al siguiente nivel cuando las fases 0 a 3 estan cerradas y al
menos una ruta end-to-end TUI queda automatizada. En ese punto Linux/400 deja de
ser solo una base funcional y pasa a ser un sistema que puede operarse,
diagnosticarse y evolucionar con confianza.
