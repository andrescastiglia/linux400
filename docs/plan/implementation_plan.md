# Plan de implementacion pendiente de Linux/400

Este plan parte del estado real del repo: ya existen runtime de objetos, TUI, compiladores, eBPF LSM, loader, demos y scripts de release. El foco ahora es convertir ese esqueleto funcional en un entorno OS/400-style operable: entrar al sistema, administrar objetos/trabajos/perfiles, desarrollar desde pantalla verde y conservar estado de forma persistente.

## Base ya implementada

- `libl400`: catalogo de objetos por xattrs, bibliotecas, PF/LF, DTAQ, source members, storage backend, cgroups, auth y FFI.
- `l400-ebpf-common`: contrato `no_std` de tipos y version de politica.
- `l400-ebpf`: hooks LSM `file_open`, `bprm_creds_from_file`, `bprm_check_security`.
- `l400-loader`: modos `full`, `degraded`, `dev` y `loader-status`.
- `os400-tui`: sign-on, menu principal, linea de comandos, objetos, trabajos, DTAQ, STRPDM, WRKMBRPDM, STRSEU, STRSQL.
- `clc`: parser CL y backend C con llamadas reales a `libl400`.
- `c400c`: compilacion C a ELF catalogado como `*PGM`.
- Release/live: userspace, initramfs, ISO, instalador textual y smoke tests.

## Objetivo de la siguiente etapa

Que un usuario pueda arrancar una ISO o instalacion, autenticarse como perfil Linux/400, entrar al menu principal y operar el sistema sin shell para las tareas minimas:

- crear y navegar bibliotecas;
- crear/listar/renombrar/borrar objetos;
- editar y compilar un fuente CL/C;
- ejecutar programas `*PGM`;
- enviar/listar jobs batch;
- revisar logs, estado del loader, jobs y storage;
- preservar `/l400` entre reinicios.

## Fase 1: Persistencia y bootstrap del sistema

**Estado:** implementada en base. Queda pendiente validacion end-to-end en una instalacion real/QEMU tras reinicio.

**Problema:** la experiencia OS/400-style depende de que existan bibliotecas y objetos base. Ademas, el flujo instalado todavia puede montar `/l400` como `tmpfs`, suficiente para demos pero no para sistema real.

Trabajo:

- [x] Crear comando o rutina idempotente `l400-bootstrap`.
- [x] Provisionar `QSYS`, `QGPL`, `QUSRSYS`, `QTEMP` y `QGPL/QCLSRC`.
- [x] Crear objetos base: `QEZJOBLOG *DTAQ`, source file `QCLSRC *FILE SRC`, perfiles iniciales y placeholders de comandos.
- [x] Decidir backend persistente para `/l400`:
  - preferido: ZFS dataset con `xattr=sa`;
  - fallback: ext4/xfs con xattrs, marcado como modo no-ZFS.
- [x] Actualizar `install_linux400.sh` e initramfs para no reemplazar `/l400` instalado por `tmpfs`.
- [x] Agregar validacion visible en `l400-support-report`.
- [ ] Validar en QEMU/instalacion real que el estado sobrevive a reboot.

Criterio de aceptacion:

- tras reiniciar una instalacion, `WRKOBJ`, `STRPDM` y `WRKMBRPDM QGPL/QCLSRC` muestran estado persistente;
- `l400-support-report --write` indica backend de `/l400` y si es persistente.

## Fase 2: Comandos minimos de operacion y administracion

**Estado:** implementada en base. Queda pendiente validacion end-to-end desde ISO/TUI real y pantallas de confirmacion dedicadas para acciones destructivas.

**Problema:** el dispatcher existe, pero varios comandos son listados basicos o no estan empaquetados.

Trabajo:

- [x] Empaquetar `sbmjob` y publicar symlink `SBMJOB`.
- [x] Unificar `WRKACTJOB` y `WRKSYSSTS` para leer `L400_RUN_DIR/jobs`, no rutas de catalogo inconsistentes.
- [x] Completar filtros de `WRKOBJ OBJ(...) OBJTYPE(...) LIB(...)`.
- [x] Agregar comandos de objetos minimos:
  - `DLTOBJ`
  - `CPYOBJ`
  - `DSPOBJD`
  - `CHGOBJD`
- [x] Agregar comandos de autorizacion:
  - `DSPOBJAUT`
  - `GRTOBJAUT`
  - `RVKOBJAUT`
- [x] Convertir `PWRDWNSYS OPTION(*IMMED|*RESTART)` en accion real cuando corre como root, con confirmacion explicita `CONFIRM(*YES)`.
- [x] Hacer `WRKUSRPRF` accionable: crear, listar, mostrar y desactivar perfiles Linux/400.
- [x] Ejecutar comandos no interactivos desde la linea de comandos de la TUI via `l400cmd`.
- [ ] Agregar confirmaciones visuales dedicadas en TUI para comandos destructivos.

Criterio de aceptacion:

- todos los comandos anteriores funcionan desde TUI command line y desde symlink shell-friendly;
- los comandos destructivos tienen confirmacion o modo explicitamente irreversible;
- hay tests para parser de parametros y rutas felices/error.

## Fase 3: Library list, perfil y sesion

**Estado:** implementada en base. Queda pendiente validacion interactiva completa en ISO/TUI y evolucionar el contexto hacia una sesion multi-job mas rica.

**Problema:** `ADDLIBLE` y `CHGCURLIB` usan variables de entorno del proceso; la TUI no mantiene todavia una sesion rica estilo OS/400.

Trabajo:

- [x] Introducir `SessionContext` en `os400-tui` con:
  - user profile;
  - current library;
  - library list;
  - last message/status;
  - job id.
- [x] Persistir library list de sesion en `L400_RUN_DIR/sessions`.
- [x] Mostrar current library real en el header del menu.
- [x] Hacer que `WRKOBJ`, `STRSQL`, `STRSEU` y `WRKMBRPDM` usen el contexto de sesion.
- [x] Mapear perfil Linux/400 a usuario Linux sin permitir operar como `root`.
- [ ] Validar en ISO/TUI real que sign-off limpia la sesion y que las pantallas se refrescan como operador espera.

Criterio de aceptacion:

- `CHGCURLIB QGPL` cambia lo que muestran pantallas y comandos sin reiniciar la TUI;
- `ADDLIBLE` afecta resolucion de objetos en esa sesion;
- sign-off limpia job/sesion.

## Fase 4: Work management real

**Estado:** implementada en base. Queda pendiente validacion interactiva en ISO/TUI y endurecer terminacion/logs para entornos multiusuario reales.

**Problema:** hay cgroups y job registry, pero faltan colas, opciones y administracion.

Trabajo:

- [x] Integrar `SBMJOB CMD(...) JOB(...) JOBQ(...)` al dispatcher.
- [x] Registrar jobs en estados `JOBQ`, `ACTIVE`, `COMPLETED`, `FAILED`.
- [x] Agregar acciones de `WRKACTJOB`: ver detalle, terminar job, refrescar, filtrar por subsystem.
- [x] Agregar descripcion de subsistemas:
  - `QINTER`
  - `QBATCH`
- [x] Hacer que `os400-tui` y jobs batch registren comando, usuario, timestamps y salida/log.
- [x] Exponer cgroup params desde pantalla de sistema.
- [ ] Validar acciones de terminacion y detalle desde TUI dentro de la ISO.

Criterio de aceptacion:

- `SBMJOB` ejecuta un comando en background, aparece en `WRKACTJOB` y termina con estado correcto;
- la TUI sigue usable si cgroups no estan disponibles, mostrando modo degradado.

## Fase 5: Archivos PF/LF/DTAQ mas cercanos a OS/400

**Problema:** PF/LF/DTAQ existen, pero el modelo de datos es minimo (`KEY/DATA`).

**Estado:** implementada en base. Quedan como pendientes de endurecimiento la demo TUI completa, validacion mas estricta del esquema de campos y compatibilidad mas rica con multiples miembros/backends.

Trabajo:

- [x] Definir metadata de esquema para PF:
  - record length;
  - campos;
  - tipo/longitud;
  - texto;
  - keyed fields.
- [x] Agregar miembros PF reales y comandos:
  - `CRTPF`
  - `CRTLF`
  - `DSPPFM`
  - `CLRPFM`
  - `ADDPFM`
  - `WRTPFM` como equivalente minimo para carga operativa.
- [x] Mantener LF automaticamente al escribir/borrar registros de PF.
- [x] Agregar RRN/arrival sequence de forma explicita.
- [x] Extender DTAQ:
  - `CRTDTAQ`
  - `SNDDTAQ`
  - `RCVDTAQ`
  - `DSPDTAQ`
  - wait time y mensajes de longitud variable.

Criterio de aceptacion:

- [x] una demo por comandos crea PF con esquema, inserta registros y consulta por LF;
- [ ] mostrar el flujo PF/LF desde TUI;
- [x] DTAQ puede enviarse/recibirse desde comando batch;
- [ ] mostrar DTAQ desde TUI.

## Fase 6: STRSQL utilizable

**Problema:** `STRSQL` soporta `SELECT` minimo, suficiente para demo pero no para administracion.

Trabajo:

- Reemplazar parseo manual por parser SQL pequeño o crate dedicado.
- Soportar:
  - `SELECT` con columnas, `WHERE`, `ORDER BY`, `LIMIT`;
  - `INSERT`;
  - `UPDATE`;
  - `DELETE`;
  - `CREATE TABLE` como alias a `CRTPF` cuando sea razonable.
- Mostrar errores con codigo/mensaje estilo pantalla verde.
- Agregar paginacion horizontal/vertical de resultados.

Criterio de aceptacion:

- `STRSQL "SELECT * FROM QGPL/CUSTOMERS WHERE KEY='C001'"` funciona igual en TUI y batch;
- INSERT/UPDATE/DELETE actualizan PF y LF de forma consistente.

## Fase 7: Compilador CL y toolchain

**Problema:** `clc` compila comandos simples, pero falta lenguaje de control real.

Trabajo:

- Extender grammar/AST:
  - variables `DCL`;
  - `CHGVAR`;
  - `IF/THEN/ELSE`;
  - `DO/ENDDO`;
  - `MONMSG`;
  - `CALL`;
  - parametros de programa.
- Generar C o LLVM con control de flujo real.
- Agregar resolucion de objetos por library list.
- Catalogar source y program objects de forma uniforme.
- Definir comando `CRTCLPGM` o mapear `CRTPGM` a flujo de compilacion.

Criterio de aceptacion:

- un CL de ejemplo crea biblioteca, cambia curlib, compila/llama programa, maneja error con `MONMSG`;
- tests unitarios cubren parser y codegen de cada estructura nueva.

## Fase 8: TUI OS/400-style completa para operaciones minimas

**Problema:** las pantallas existen, pero faltan opciones numericas y prompt F4 real.

Trabajo:

- Implementar prompt F4 por comando con campos editables.
- Agregar columna `Opt` accionable en ObjectBrowser, WRKACTJOB, WRKMBRPDM y DTAQ.
- Unificar barra de ayuda y mensajes de estado.
- Agregar pantallas:
  - `WRKLIB`
  - `DSPOBJD`
  - `WRKUSRPRF`
  - `WRKSYSSTS`
  - `WRKSYSVAL`
  - `WRKSPLF` o spool/outq minimo si se decide incluir `*OUTQ`.
- Evitar datos fallback silenciosos: si no hay runtime real, mostrar "sin catalogo" o "modo demo" explicitamente.

Criterio de aceptacion:

- un operador puede administrar el sistema minimo desde TUI sin shell;
- las teclas F y opciones se comportan de forma consistente entre pantallas.

## Fase 9: Seguridad, auditoria y politica kernel

**Problema:** la politica eBPF valida tipos y ejecucion, pero la autorizacion de objetos todavia vive principalmente en runtime.

Trabajo:

- Definir una matriz de autorizaciones por objeto/comando.
- Agregar auditoria en `QHST`/DTAQ para:
  - acceso denegado;
  - ejecucion de `*PGM`;
  - cambios de autorizacion;
  - cambios de perfil.
- Evaluar como pasar identidad/autoridad al eBPF sin complejidad excesiva.
- Firmar o marcar toolchain output mas robustamente que `objattr=C|CL`.
- Agregar comandos de verificacion de politica.

Criterio de aceptacion:

- `*PUBLIC:*EXCLUDE` se respeta de forma consistente en runtime y ejecucion;
- los denegados quedan visibles en logs/TUI.

## Fase 10: Release, CI y matriz de plataformas

Trabajo:

- CI para:
  - `cargo test -p l400`;
  - `cargo test -p clc`;
  - `cargo test -p os400-tui`;
  - smoke scripts;
  - build userspace;
  - build eBPF cuando el toolchain este disponible.
- QEMU smoke install obligatorio antes de RC.
- Matriz de soporte:
  - dev sin BPF/ZFS;
  - degraded con cgroups pero sin eBPF;
  - full con BPF LSM, BTF, ZFS `xattr=sa`.
- Documentar upgrade/migration de `/l400`.

Criterio de aceptacion:

- `./scripts/test/test_release_rc.sh` queda como gate minimo;
- `RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh` valida instalacion y persistencia.

## Prioridad recomendada

1. Persistencia `/l400` + bootstrap.
2. Unificar job registry y empaquetar `SBMJOB`.
3. Completar comandos administrativos minimos.
4. SessionContext real en TUI.
5. PF/LF/DTAQ con comandos operativos.
6. CL con control de flujo.
7. Seguridad/autorizacion integrada con eBPF.

La razon de este orden es practica: primero hay que poder arrancar, conservar estado y administrar lo minimo desde el menu. Despues vale la pena profundizar compatibilidad semantica de archivos, SQL, CL y enforcement.
