# Linux/400: estado actual del proyecto

Este documento describe que features estan implementadas hoy y que falta para alcanzar la vision de `docs/KERNEL.md`. El plan de trabajo para Version 1 esta en `docs/plan/implementation_plan.md`.

## Resumen actual

Linux/400 ya tiene una base funcional para una personalidad OS/400-style sobre Linux:

- runtime de objetos sobre filesystem y xattrs;
- tipos compartidos entre userspace y eBPF;
- PF/LF/DTAQ con backend `sled`;
- loader eBPF con modos `full`, `degraded` y `dev`;
- TUI de pantalla verde con sign-on, menu, comando, objetos, jobs, usuarios, spool, PDM, SEU, SQL y DTAQ;
- compilador CL y compilador C/400 que catalogan `*PGM`;
- comandos operativos via `l400cmd` y symlinks;
- scripts de userspace, initramfs, ISO, instalacion, upgrade check, migracion y smoke tests.

La foto general es: **runtime y demos V1 avanzados, operacion instalada y mantenimiento todavia incompletos**.

## Features implementadas

### Runtime de objetos

Implementado:

- `L400_ROOT` configurable (`/l400` por defecto);
- catalogacion por xattrs;
- objetos `*LIB`, `*PGM`, `*FILE`, `*DTAQ`, `*USRPRF`, `*CMD`, `*OUTQ`;
- reconocimiento de `*SRVPGM`;
- soporte best-effort de ZFS;
- bootstrap de bibliotecas y objetos base;
- integridad de objetos con `CHKOBJINT`;
- copy, rename, delete, change/display object description y authorities.

Metadatos principales actuales:

```text
user.l400.objtype
user.l400.objattr
user.l400.text
user.l400.owner
user.l400.owner_uid
user.l400.auth
user.l400.auth.manifest
user.l400.storage_backend
user.l400.record_len
user.l400.base_pf
user.l400.data.version
```

Falta:

- contrato completo de `*SRVPGM`;
- versionado/migracion mas robusta de metadata por objeto;
- comandos de restore selectivo por biblioteca/objeto;
- validacion de integridad mas profunda para todos los tipos;
- enforcement uniforme para operaciones sensibles fuera de los comandos felices.

### PF, LF, DTAQ y SQL

Implementado:

- `CRTPF`, `CRTLF`, `DSPPFM`, `CLRPFM`, `ADDPFM`, `WRTPFM`;
- PF con record length, schema basico, miembros y RRN/arrival sequence;
- LF como indice secundario mantenido por escrituras/borrados del PF;
- `CRTDTAQ`, `SNDDTAQ`, `RCVDTAQ`, `DSPDTAQ`;
- DTAQ persistente FIFO con mensajes de longitud variable;
- `STRSQL` con `SELECT`, `INSERT`, `UPDATE`, `DELETE` y `CREATE TABLE` minimo;
- tests y demos de PF/LF/DTAQ y backup/restore.

Falta:

- SQL mas completo, optimizacion y errores diagnosticos formales;
- constraints, conversiones y tipos de datos mas ricos;
- comandos de administracion de miembros mas cercanos a IBM i;
- herramientas de inspeccion/reparacion de indices LF;
- cobertura productiva de concurrencia y recuperacion ante fallos.

### Comandos

Implementado en `l400cmd` y symlinks:

```text
WRKSYSSTS WRKACTJOB WRKJOB WRKJOBQ HLDJOB RLSJOB ENDJOB WRKSYSVAL DSPLOG
DSPCMD WRKCMD CRTCMD WRKUSRPRF WRKSPLF WRKOUTQ CRTOUTQ DLTOUTQ
DSPSPLF CHGSPLFA DLTSPLF PWRDWNSYS
WRKOBJ CRTLIB DLTLIB ADDLIBLE CHGCURLIB RNMOBJ DLTOBJ CPYOBJ
DSPOBJD CHGOBJD DSPOBJAUT CHKOBJAUT GRTOBJAUT RVKOBJAUT
CHKOBJINT DSPPOLICY DSPAUD
CRTPGM CRTCLPGM CALL SBMJOB
STRPDM STRSEU STRSQL WRKMBRPDM DLTMBR CPYMBR CHGMBRD GO SIGNOFF
CRTPF CRTLF DSPPFM CLRPFM ADDPFM WRTPFM
CRTDTAQ SNDDTAQ RCVDTAQ DSPDTAQ
```

Falta:

- separar claramente comandos estables, experimentales y stubs;
- ampliar `CRTUSRPRF`, `CHGUSRPRF`, `DLTUSRPRF`, `DSPUSRPRF` como comandos dedicados;
- comandos formales de backup/restore y PTF;
- comandos completos de subsistemas, job queues y writers;
- catalogo CPF consistente para errores y ayudas.

### TUI

Implementado:

- sign-on estilo OS/400 con sesion Linux/400;
- bloqueo de perfil `ROOT`;
- current library, library list, last message y job id;
- menu principal y command line persistente;
- `F4` prompt por campos para comandos catalogados;
- opciones numericas y confirmaciones visuales;
- object browser, detalles de objetos, PF/DTAQ viewers;
- `WRKACTJOB`, detalle y terminacion de jobs;
- pantallas de usuarios, spool/outq, logs y politica;
- `STRPDM`, `WRKMBRPDM`, `STRSEU`, `STRSQL`;
- tests smoke `cargo test -p os400-tui`.

Falta:

- pantallas de instalacion/mantenimiento/PTF/backup/restore;
- pantallas completas de job queues, subsistemas y spool writers;
- ayuda por campo y mensajes CPF mas exhaustivos;
- pruebas interactivas mas profundas para permisos, navegacion y degradacion;
- eliminar o marcar explicitamente cualquier fallback demo.

### Toolchain CL/C

Implementado:

- `clc` con parser Pest, AST, backend C por defecto y backend LLVM opcional;
- variables, parametros, `IF/ELSE`, `DO/ENDDO`, `MONMSG` y `CALL`;
- integracion con estado CPF formal del runtime;
- compilacion y catalogacion final como `*PGM`;
- `c400c` para compilar C nativo, enlazar `libl400` y catalogar `*PGM`;
- marcas simples de toolchain en xattrs.

Falta:

- mayor cobertura de CL;
- diagnosticos/listados de compilacion mas cercanos a IBM i;
- firma fuerte o attestacion criptografica del toolchain;
- integracion de compilacion como flujo primario desde PDM/SEU;
- RPG y SQL como lenguajes de desarrollo de Version 2.

### Work management

Implementado:

- subsistemas base `QINTER` y `QBATCH`;
- jobs en `L400_RUN_DIR/jobs`;
- estados `JOBQ`, `HELD`, `ACTIVE`, `ENDING`, `ENDED`, `COMPLETED`, `FAILED`, `KILLED`;
- `SBMJOB`, `WRKACTJOB`, `WRKJOB`, `WRKJOBQ`, `HLDJOB`, `RLSJOB`, `ENDJOB`;
- salida/log por job batch;
- cgroups v2 best-effort y modo degradado si no estan disponibles.

Falta:

- modelar `*JOBQ` como objeto operativo completo;
- subsistemas configurables mas alla de base;
- scheduling, prioridad, clases y control de recursos mas fino;
- job logs con formato y retencion mas completos;
- writers y enlace formal entre jobs, spool y output queues.

### Usuarios, seguridad y auditoria

Implementado:

- objetos `*USRPRF`;
- `WRKUSRPRF` con listado/creacion/desactivacion basica;
- matriz runtime para `READ`, `CHANGE`, `EXECUTE`, `ADMIN`;
- autorizaciones `*USE`, `*CHANGE`, `*ALL`, `*EXCLUDE`;
- owner implicito con autoridad elevada;
- `GRTOBJAUT`, `RVKOBJAUT`, `DSPOBJAUT`, `CHKOBJAUT`;
- `CALL` verifica autoridad antes de ejecutar;
- auditoria en `QSYS/QHST` y, si existe, `QUSRSYS/QEZJOBLOG`;
- `DSPPOLICY` y `DSPAUD`;
- eBPF valida tipo, formato `*PGM`, `*PUBLIC:*EXCLUDE`, owner UID y entradas `UID:<uid>`.

Falta:

- ciclo completo de perfiles: crear, cambiar, deshabilitar, borrar, password, grupos y politicas;
- autorizacion aplicada de forma mas uniforme en todos los comandos;
- auditoria mas estructurada y consultable por rango, usuario, objeto y evento;
- paridad mas amplia entre runtime auth y eBPF;
- pantallas administrativas completas para perfiles y auditoria.

### Spool y output queues

Implementado:

- tipo `*OUTQ`;
- `CRTOUTQ`, `DLTOUTQ`, `WRKOUTQ`;
- `WRKSPLF`, `DSPSPLF`, `CHGSPLFA`, `DLTSPLF`;
- directorio base de spool `QUSRSYS/QSPL` o `L400_SPOOL_DIR`;
- estados basicos de spool files.

Falta:

- writers, impresoras/exportadores y routing real;
- retencion automatica y limpieza programada;
- ownership/autorizacion por spool file;
- relacion mas formal entre job output y spool file;
- pantallas y comandos mas completos de administracion de cola de salida.

### eBPF, loader y politica kernel

Implementado:

- crate comun `no_std` con tipos validos y version de politica;
- hooks LSM para `file_open`, `bprm_creds_from_file` y `bprm_check_security`;
- loader con modos `full`, `degraded` y `dev`;
- persistencia de estado para TUI/reportes;
- tests de loader y build eBPF opcional.

Falta:

- cobertura mayor de operaciones de archivo por perfil/grupo;
- auditoria kernel/userspace mas correlacionada;
- e2e regular en perfil `full`;
- upgrade/PTF con rollback del artefacto eBPF;
- documentacion operativa de fallos BTF/kernel/permisos.

### Instalacion, release, backup y upgrade

Implementado:

- build userspace;
- build de rootfs Alpine, initramfs e ISO;
- instalador textual y `install-linux400`;
- autologin segun modo live/install/installed;
- support report;
- `l400-upgrade-check`;
- `l400-migrate` de metadata;
- backup/restore validado por `rsync -aX`;
- gate `test_release_rc.sh`;
- QEMU install smoke con `RUN_E2E_INSTALL=1`.

Falta:

- flujo de PTFs real: paquete, precheck, apply, rollback, auditoria;
- comandos/TUI para backup/restore y mantenimiento;
- restore selectivo y verificacion posterior guiada;
- instalacion endurecida para mas escenarios de disco/EFI/error;
- pruebas regulares de upgrade desde versiones anteriores;
- modo rescue mas documentado y operativo desde menu.

## Componentes

| Componente | Estado resumido |
| --- | --- |
| `libl400` | Runtime principal; avanzado para demos V1, faltan operaciones productivas completas. |
| `l400-ebpf-common` | Contrato compartido estable para alcance actual. |
| `l400-ebpf` | Enforcement kernel parcial; necesita ampliar cobertura y e2e full. |
| `l400-loader` | Modos y estado implementados; falta operacion instalada/monitoreada. |
| `os400-tui` | Experiencia interactiva amplia; faltan pantallas de mantenimiento y administracion profunda. |
| `clc` | CL funcional parcial; falta cobertura completa y diagnosticos. |
| `c400c` | C nativo catalogado como `*PGM`; falta contrato runtime mas completo. |
| `scripts` | Build/install/test base; falta flujo formal PTF/backup/restore operacional. |

## Validacion actual

Gate rapido local:

```bash
cargo fmt --all --check
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
```

Smoke/release:

```bash
./scripts/test/test_objects_v1_demo.sh
./scripts/test/test_toolchain_v1_demo.sh
./scripts/test/test_workload_demo.sh
./scripts/test/test_loader_modes.sh
./scripts/test/test_release_rc.sh
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

Limitaciones de validacion:

- el e2e QEMU depende del host;
- el perfil `full` requiere kernel/BTF/cgroups/xattrs adecuados;
- no hay suite interactiva equivalente a una 5250 completa;
- algunos flujos administrativos existen como scripts o comandos, pero no como experiencia TUI cerrada.
