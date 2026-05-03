# Plan de implementacion hacia Linux/400 Version 1

Fecha base: 2026-05-02
Objetivo: cerrar una Version 1 operable, instalable, mantenible y recuperable, con experiencia basica de administracion tipo AS/400.

Este plan reemplaza planes previos centrados solo en UI. La prioridad de V1 es que el sistema pueda usarse como entorno operacional basico: instalar, actualizar, aplicar PTFs, respaldar, restaurar, administrar usuarios, jobs, job queues, spool, bibliotecas, objetos y datos desde Linux/400.

## Principios de V1

- La TUI es la interfaz normal de operador; la shell queda para soporte/rescue.
- Todo flujo critico debe tener comando, pantalla y prueba.
- Cada accion destructiva requiere confirmacion y auditoria.
- Cada feature debe degradar con mensaje Linux/400 claro si falta soporte del host.
- El estado persistente de `/l400` y sus xattrs es parte central del producto.
- Los gates de release deben probar instalacion, upgrade, backup/restore y operacion basica.

## Alcance de V1

V1 debe incluir:

- instalacion live/install e instalada con persistencia;
- actualizacion y PTFs con precheck, apply, rollback y auditoria;
- backup/restore completo y verificable;
- administracion de usuarios y autoridades;
- administracion de bibliotecas, objetos y datos;
- work management basico: QINTER, QBATCH, job queues y logs;
- spool basico: output queues, spool files, estados y retencion;
- comandos y pantallas para operacion diaria;
- CL y C como lenguajes soportados de V1;
- PF/LF/DTAQ y SQL minimo operativo;
- soporte/rescue y reportes diagnosticos.

Fuera de V1:

- RPG;
- SQL avanzado como lenguaje de desarrollo completo;
- compatibilidad binaria IBM i;
- emulacion completa de 5250;
- service programs productivos completos si no son necesarios para V1.

## Fase 1: estabilizar base operacional

Estado: completada.

Objetivo: asegurar que lo ya implementado sea confiable como base de V1.

Tareas:

- [x] Revisar comandos actuales y clasificarlos como estable, experimental o stub.
- [x] Hacer que `DSPCMD`/`WRKCMD` muestren estado, autoridad, parametros y ejemplos de cada comando.
- [x] Garantizar que todos los comandos sensibles emitan status CPF o equivalente.
- [x] Unificar validacion de autoridad para create/change/delete/call/spool/jobs.
- [x] Asegurar que `l400-bootstrap` cree objetos base, `*OUTQ`, `*JOBQ`, perfiles y metadata versionada.
- [x] Expandir `CHKOBJINT` para `*OUTQ`, `*JOBQ`, `*USRPRF`, PF/LF/DTAQ y `*PGM`.
- [x] Agregar tests de regresion por comando critico en `libl400`.

Criterio de cierre:

- `cargo test -p l400`, `cargo test -p clc` y `cargo test -p os400-tui` pasan.
- Los comandos V1 tienen metadata visible.
- No hay stubs silenciosos en flujos V1.

## Fase 2: instalacion y primer arranque

Estado: completada.

Objetivo: que la instalacion sea repetible, diagnosticable y validada por gate.

Tareas:

- [x] Endurecer `install-linux400` para errores de disco, particion, EFI, rootfs y persistencia.
- [x] Agregar pantalla TUI de instalacion/resumen cuando el boot sea `install`.
- [x] Registrar en `/l400` version instalada, build id, metadata version y perfil de plataforma.
- [x] Validar que el primer arranque cree o repare objetos base sin borrar datos del operador.
- [x] Mejorar modo rescue con opciones: montar `/l400`, support report, upgrade check, restore y shell.
- [x] Hacer que `test_e2e_install_qemu.sh` verifique persistencia de objetos, usuarios, spool y jobs.

Criterio de cierre:

- `RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh` instala, reinicia y valida persistencia.
- El operador puede reconocer modo live/install/installed desde TUI o support report.

Commit: 4e5df62 - feat(phase2): Implement Phase 2 - Installation and first boot

## Fase 3: actualizacion y PTFs

Estado: completado (90% - TUI screen pendiente).

Objetivo: introducir mantenimiento versionado estilo PTF.

Tareas:

- [x] Definir formato de paquete PTF: manifiesto, version origen/destino, archivos, scripts, checksum y rollback.
- [x] Crear comando `DSPPTF` para listar PTFs aplicados y pendientes.
- [x] Crear comando `APYPTF` con `OPTION(*CHECK|*APPLY|*ROLLBACK)` y `CONFIRM`.
- [x] Integrar `l400-upgrade-check` como precheck obligatorio de `APYPTF`.
- [x] Expandir `l400-migrate` para migraciones idempotentes por version.
- [x] Auditar apply/rollback con usuario, fecha, build id y resultado.
- [ ] Agregar pantalla TUI de mantenimiento/PTF (Tarea 7 pendiente).
- [x] Agregar tests de PTF con paquete falso, apply, rollback y downgrade rechazado.

Criterio de cierre:

- Un PTF puede aplicarse y revertirse en entorno de prueba.
- El sistema bloquea downgrades de metadata sin restore.
- `DSPPTF` y support report muestran historial de mantenimiento.

Commit: 82c1e71 - feat(phase3): Implement Phase 3 - PTFs y Actualización (SERVICE option)

## Fase 4: backup, restore e integridad

Estado: pendiente.

Objetivo: convertir las recetas actuales de backup/restore en operacion Linux/400.

Tareas:

- [ ] Crear comandos `SAVLIB`, `SAVOBJ`, `SAVSYS` o equivalentes V1.
- [ ] Crear comandos `RSTLIB`, `RSTOBJ`, `RSTSYS` o equivalentes V1.
- [ ] Preservar xattrs, ownership Linux/400, auth manifest, PF/LF/DTAQ y spool cuando aplique.
- [ ] Soportar backend de backup por `rsync -aX`, `tar --xattrs` y, si existe ZFS, snapshot/send.
- [ ] Ejecutar `CHKOBJINT` despues de restore.
- [ ] Agregar pantalla TUI de backup/restore con progreso y resultado.
- [ ] Documentar procedimiento de restore desde rescue.
- [ ] Ampliar `test_l400_backup_restore.sh` con usuarios, autoridades, outq, spool y job logs.

Criterio de cierre:

- Backup completo de `/l400` restaura objetos, datos, xattrs y autorizaciones.
- Restore selectivo de biblioteca/objeto funciona en tests.
- La TUI muestra exito/falla y proximo paso operativo.

## Fase 5: usuarios, perfiles y autoridades

Estado: pendiente.

Objetivo: cerrar administracion de usuarios V1.

Tareas:

- [ ] Completar comandos dedicados `CRTUSRPRF`, `CHGUSRPRF`, `DLTUSRPRF`, `DSPUSRPRF`.
- [ ] Definir atributos V1 de perfil: status, UID, texto, clase, home/current library, grupos o perfiles suplementarios.
- [ ] Integrar cambio/validacion de password si el perfil se enlaza a PAM/Linux.
- [ ] Hacer que `WRKUSRPRF` use esos comandos en vez de acciones parciales.
- [ ] Aplicar autorizacion runtime a todos los comandos administrativos.
- [ ] Expandir auditoria `USRPRF_CHANGE`, grants, revokes y logins.
- [ ] Agregar tests de crear, deshabilitar, reactivar, borrar y denegar login/uso.

Criterio de cierre:

- Un administrador puede gestionar perfiles desde TUI.
- Autoridades sobre objetos se conservan en backup/restore.
- Denegados aparecen en auditoria y tienen mensaje operativo claro.

## Fase 6: work management y job queues

Estado: pendiente.

Objetivo: hacer que jobs y colas sean una herramienta operacional, no solo una demo.

Tareas:

- [ ] Formalizar `*JOBQ` como tipo valido en contrato comun si se decide mantenerlo como objeto kernel-visible.
- [ ] Crear/normalizar comandos `CRTJOBQ`, `DLTJOBQ`, `HLDJOBQ`, `RLSJOBQ`, `WRKJOBQ`.
- [ ] Persistir metadata de job queue y relacion con subsistema.
- [ ] Mejorar `SBMJOB` con usuario, jobq, prioridad, log y salida spool.
- [ ] Completar pantallas de job detail, job log y job queue.
- [ ] Manejar terminacion controlada vs inmediata con auditoria.
- [ ] Agregar tests de hold/release/end, jobs fallidos y salida spool.

Criterio de cierre:

- Jobs batch pueden enviarse, retenerse, liberarse, terminarse y auditarse.
- Los logs sobreviven lo necesario y son visibles por comando/TUI.
- Modo sin cgroups degrada de forma explicita.

## Fase 7: spool y output queues

Estado: pendiente.

Objetivo: cubrir la administracion basica de spool AS/400-style.

Tareas:

- [ ] Completar atributos de `*OUTQ`: status, retencion, routing, autoridad y texto.
- [ ] Generar spool files desde `SBMJOB`, comandos y reportes.
- [ ] Definir formato metadata de spool file: owner, job, outq, status, fecha, tamano, paginas/logicas.
- [ ] Implementar retencion/limpieza basica.
- [ ] Agregar comandos/pantallas para hold, release, save/delete/display spool files.
- [ ] Implementar writer/export minimo a archivo o stdout controlado.
- [ ] Agregar tests de outq, spool states, delete confirmado y restore.

Criterio de cierre:

- Un operador puede ver y administrar salida batch desde TUI.
- Spool participa en backup/restore cuando se elige incluirlo.
- Estados y autorizaciones son consistentes.

## Fase 8: datos y toolchain de V1

Estado: pendiente.

Objetivo: cerrar el flujo de desarrollo basico CL/C y datos administrativos.

Tareas:

- [ ] Integrar compilacion desde PDM/SEU con comandos `CRTCLPGM`, `CRTPGM` y mensajes de error.
- [ ] Ampliar tests CL para programas administrativos V1.
- [ ] Mejorar salida de compilacion y job log.
- [ ] Fortalecer PF/LF/DTAQ con errores CPF, integridad y concurrencia basica.
- [ ] Hacer que `STRSQL` pueda usarse sobre PF V1 con resultados navegables y errores claros.
- [ ] Mantener RPG y SQL avanzado documentados como V2, sin bloquear V1.

Criterio de cierre:

- Un usuario puede crear fuente, compilar CL/C, ejecutar y revisar logs sin shell.
- PF/LF/DTAQ soportan los flujos administrativos y demos V1.

## Fase 9: seguridad kernel y perfiles de plataforma

Estado: pendiente.

Objetivo: que `dev`, `degraded` y `full` sean comprensibles y testeables.

Tareas:

- [ ] Alinear `l400-ebpf-common` con tipos V1 definitivos.
- [ ] Expandir enforcement eBPF donde aporte proteccion real sin romper modo dev.
- [ ] Mejorar reportes de loader: BTF, kernel, cgroups, xattrs, artefacto eBPF y modo efectivo.
- [ ] Mostrar modo efectivo en TUI y support report.
- [ ] Crear pruebas e2e documentadas para perfil `full`.
- [ ] Definir politica de upgrade/PTF para artefacto eBPF.

Criterio de cierre:

- El operador sabe si el sistema esta protegido, degradado o en desarrollo.
- Los comandos sensibles no dependen exclusivamente de eBPF; runtime sigue validando.

## Fase 10: release candidate V1

Estado: pendiente.

Objetivo: convertir el sistema en un release candidate instalable y verificable.

Tareas:

- [ ] Actualizar `docs/KERNEL.md`, `docs/PROJECT.md`, README principal y README de componentes.
- [ ] Crear checklist de operacion V1: instalar, crear usuario, crear biblioteca, compilar, enviar job, revisar spool, backup, PTF, restore.
- [ ] Ejecutar gate rapido local.
- [ ] Ejecutar gate de release.
- [ ] Ejecutar QEMU install cuando el host lo permita.
- [ ] Generar support report y artefactos de release.
- [ ] Documentar limitaciones conocidas y caminos de rescue.

Criterio de cierre:

- Un operador puede completar el checklist V1 sin shell.
- Los gates reproducibles pasan.
- Las limitaciones restantes no afectan instalacion, mantenimiento ni recuperacion basica.

## Gates permanentes

Gate rapido local:

```bash
cargo fmt --all --check
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
```

Gate de calidad:

```bash
cargo clippy -p l400 --all-targets -- -D warnings
cargo clippy -p os400-tui --all-targets -- -D warnings
./scripts/test/test_release_rc.sh
```

Gate de release V1:

```bash
./scripts/test/test_objects_v1_demo.sh
./scripts/test/test_toolchain_v1_demo.sh
./scripts/test/test_workload_demo.sh
./scripts/test/test_loader_modes.sh
./scripts/test/test_l400_backup_restore.sh
./scripts/test/test_l400_upgrade_metadata.sh
./scripts/test/test_release_rc.sh
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

Pruebas que deben agregarse antes de cerrar V1:

- PTF apply/rollback;
- backup/restore con usuarios, autoridades, outq y spool;
- TUI de mantenimiento;
- job queue hold/release/end con salida spool;
- instalacion QEMU con persistencia de usuarios, objetos y spool;
- perfil `full` documentado con eBPF activo.
