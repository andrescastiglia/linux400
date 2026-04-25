# Plan para alcanzar el objetivo de Linux/400

Este documento lista lo que falta para que el estado actual descrito en `docs/PROJECT.md` alcance la vision definida en `docs/KERNEL.md`.

No es un historial de fases ya cerradas. Es una hoja de ruta viva de brechas pendientes.

## Estado base

El proyecto ya tiene una base operable: runtime de objetos, TUI, comandos, PF/LF/DTAQ, compiladores, eBPF LSM, loader y scripts de release. La siguiente etapa debe convertir esa base en un sistema mas coherente, durable, administrable y cercano al modo de trabajo OS/400.

## Fase 1: Instalacion y persistencia de sistema

**Estado:** pendiente de endurecimiento.

Objetivo: una instalacion debe conservar `/l400`, perfiles, objetos, jobs/logs relevantes y configuracion entre reinicios, con reporte claro de backend y modo de plataforma.

Trabajo:

- [ ] Ejecutar y mantener `RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh` como gate regular en infraestructura con QEMU/OVMF.
- [ ] Validar persistencia no solo de objetos base, sino tambien de bibliotecas/miembros/archivos creados por usuario antes del reboot.
- [ ] Documentar y probar rollback/backup/restore de `/l400`.
- [ ] Validar ZFS `xattr=sa` cuando el backend sea ZFS.
- [ ] Definir migraciones versionadas de metadatos para cambios futuros de xattrs/schema.

Criterio:

- una instalacion puede crear objetos de usuario, reiniciar y verlos desde TUI;
- `l400-support-report --write` clasifica correctamente `dev`, `degraded` o `full`;
- hay procedimiento probado de backup/restore de `/l400`.

## Fase 2: TUI de operacion completa

**Estado:** parcialmente implementada; falta profundidad interactiva.

Objetivo: operar el sistema normal sin shell, con pantallas consistentes, prompt completo y acciones seguras.

Trabajo:

- [ ] Convertir mas comandos en pantallas dedicadas, no solo salida de `SystemPanel`.
- [ ] Agregar tests de navegacion TUI de extremo a extremo sobre terminal simulada.
- [ ] Mejorar prompt `F4` con tipos de parametro, valores permitidos, ayuda por campo y errores CPF.
- [ ] Agregar confirmaciones visuales para todas las acciones destructivas, no solo las principales.
- [ ] Completar `WRKSPLF`/`WRKOUTQ` como pantalla real.
- [ ] Agregar mensajes de estado uniformes entre pantallas.

Criterio:

- un operador puede crear, editar, compilar, ejecutar, auditar y borrar desde TUI;
- las teclas F y opciones numericas se comportan igual en todas las pantallas;
- los comandos destructivos nunca se ejecutan por accidente.

## Fase 3: Modelo de objetos mas rico

**Estado:** base implementada; falta semantica de IBM i mas amplia.

Objetivo: que los objetos no sean solo archivos etiquetados, sino unidades administrables con metadatos, versionado y politica consistente.

Trabajo:

- [ ] Profundizar `*CMD` como objeto promptable con definicion de parametros.
- [ ] Definir comportamiento real de `*SRVPGM`.
- [ ] Completar `*OUTQ` y spool.
- [ ] Agregar owner semantico Linux/400 separado de UID Linux cuando corresponda.
- [ ] Agregar version de formato de objeto y migraciones.
- [ ] Reemplazar marca simple de toolchain por manifest o firma verificable.

Criterio:

- `DSPOBJD`, autorizaciones, auditoria y TUI muestran metadatos consistentes;
- agregar un tipo nuevo obliga a actualizar contrato, runtime, eBPF, comandos y docs.

## Fase 4: Datos PF/LF/DTAQ y SQL

**Estado:** funcional minimo; falta robustez de datos.

Objetivo: soportar flujos administrativos y demos de datos con semantica mas cercana a OS/400.

Trabajo:

- [ ] Validacion estricta de schema PF por tipo, longitud y claves.
- [ ] Multiples miembros PF con comandos completos de seleccion/copia/limpieza.
- [ ] LF con claves compuestas y criterios de seleccion.
- [ ] SQL con parser mas completo, errores CPF y vistas paginadas mas ricas.
- [ ] DTAQ con atributos configurables y visor TUI con acciones.
- [ ] Tests de consistencia PF/LF ante update/delete mas amplios.

Criterio:

- PF/LF sobreviven operaciones CRUD repetidas sin perder indices;
- STRSQL y comandos PF/LF muestran errores recuperables y auditables.

## Fase 5: CL, C y entorno de desarrollo

**Estado:** compilacion real implementada; falta profundidad de lenguaje y tooling.

Objetivo: que el desarrollo desde pantalla verde sea una experiencia completa para programas simples y administrativos.

Trabajo:

- [ ] Ampliar CL con mas comandos soportados nativamente.
- [ ] Implementar codigos CPF consistentes en todos los comandos runtime.
- [ ] Profundizar `MONMSG` con rangos/genericidad y flujo de error mas fiel.
- [ ] Mejorar diagnosticos de compilacion y listado de errores en TUI.
- [ ] Definir uso real o retiro del backend LLVM experimental.
- [ ] Integrar objetos `*CMD` y prompts con programas CL.

Criterio:

- un fuente CL administrativo puede compilar, manejar errores, llamar programas y operar objetos sin shell;
- errores de runtime pueden ser capturados por `MONMSG` de forma predecible.

## Fase 6: Seguridad, autorizaciones y auditoria

**Estado:** runtime y eBPF cubren la base; falta convergencia completa.

Objetivo: una politica unica y visible para usuario, owner, grupos, comandos y kernel.

Trabajo:

- [ ] Aplicar autorizaciones por perfil/grupo/owner tambien en `file_open`, no solo en ejecucion.
- [ ] Unificar representacion `USER`, `UID`, owner y grupos.
- [ ] Auditar todos los comandos sensibles con codigo, usuario, objeto y resultado.
- [ ] Agregar comandos de consulta historica de auditoria con filtros.
- [ ] Endurecer `*PGM` con firma/manifest de toolchain.
- [ ] Tests e2e para denegados runtime y denegados eBPF.

Criterio:

- un denegado se ve igual desde comando, TUI, auditoria y soporte;
- `full` refuerza en kernel lo que `degraded` aplica en runtime.

## Fase 7: Work management y subsistemas

**Estado:** base funcional; falta modelo operativo mas completo.

Objetivo: jobs y subsistemas deben parecer entidades Linux/400 administrables, no solo procesos listados.

Trabajo:

- [ ] Colas de trabajo reales (`JOBQ`) y politicas de despacho.
- [ ] Subsistemas configurables mas alla de `QINTER`/`QBATCH`.
- [ ] Logs por job con vista TUI y comandos.
- [ ] Estados de job mas completos y terminacion controlada.
- [ ] Integracion mas fuerte con cgroups v2 y limites por perfil/subsistema.
- [ ] Modo degraded claro cuando cgroups no esten disponibles.

Criterio:

- `SBMJOB` puede encolar, ejecutar, registrar salida y aparecer en pantallas;
- un operador puede diagnosticar y terminar jobs sin shell.

## Fase 8: Release y soporte de plataforma

**Estado:** pipeline base implementado; falta disciplina de release continua.

Objetivo: cada RC debe demostrar build, smoke, instalacion, persistencia y reporte de soporte.

Trabajo:

- [ ] Ejecutar QEMU install smoke en CI o runner dedicado.
- [ ] Publicar artefactos con matriz `dev/degraded/full`.
- [ ] Validar build eBPF cuando haya toolchain disponible.
- [ ] Mantener documentacion de upgrade/migration de `/l400`.
- [ ] Agregar reportes de compatibilidad de kernel, BTF, BPF LSM, cgroups y ZFS.
- [ ] Agregar pruebas de instalacion con datos persistentes de usuario.

Criterio:

- un RC no se declara listo sin QEMU install smoke;
- los usuarios pueden saber antes de operar si estan en `dev`, `degraded` o `full`.

## Prioridad recomendada

1. Gate QEMU con persistencia de datos de usuario.
2. TUI dedicada para operaciones aun encapsuladas en `SystemPanel`.
3. Seguridad unificada usuario/grupo/owner en runtime y eBPF.
4. CPF consistente en comandos runtime y `MONMSG`.
5. `*OUTQ`, spool y `*CMD` como objetos completos.
6. PF/LF/SQL mas estrictos.
7. Firma/manifest de toolchain.

La prioridad sigue una logica operativa: primero asegurar que el sistema instala y conserva estado; despues mejorar la experiencia sin shell; luego profundizar compatibilidad, seguridad y datos.
