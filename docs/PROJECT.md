# Linux/400: estado actual del proyecto

Este documento describe lo que existe hoy en el repositorio. La vision objetivo vive en `docs/KERNEL.md`; el trabajo que falta para llegar a esa vision vive en `docs/plan/implementation_plan.md`.

## Resumen

Linux/400 ya tiene una base funcional para una personalidad OS/400-style sobre Linux:

- runtime de objetos sobre filesystem y xattrs;
- tipos compartidos entre userspace y eBPF;
- PF/LF/DTAQ con backend `sled`;
- loader eBPF con modos `full`, `degraded` y `dev`;
- TUI de pantalla verde con sign-on, menu, comando, objetos, jobs, PDM, SEU, SQL y DTAQ;
- compilador CL y compilador C/400 que catalogan `*PGM`;
- comandos operativos via `l400cmd` y symlinks;
- scripts de userspace, initramfs, ISO, instalacion y smoke tests.

## Componentes

| Area | Estado actual |
| --- | --- |
| `libl400` | Core runtime: objetos, bibliotecas, PF/LF, DTAQ, source members, auth, auditoria, cgroups, storage y FFI. |
| `l400-ebpf-common` | Contrato `no_std` de tipos validos, version de politica y contadores compartidos. |
| `l400-ebpf` | Hooks LSM para `file_open`, `bprm_creds_from_file` y `bprm_check_security`. |
| `l400-loader` | Carga/adjunta eBPF, persiste estado y permite modos `full`, `degraded`, `dev`. |
| `os400-tui` | TUI con sign-on, menu principal, command line, object browser, work management, DTAQ, STRPDM, WRKMBRPDM, STRSEU y STRSQL. |
| `clc` | Parser CL con backend C por defecto; soporta variables, parametros, control de flujo, `MONMSG`, `CALL` y comandos runtime. |
| `c400c` | Compila C nativo y cataloga el resultado como `*PGM`. |
| Scripts release | Build userspace, Alpine rootfs, initramfs, ISO, instalador, QEMU smoke y support report. |

## Runtime de objetos

Los objetos viven bajo `L400_ROOT` (`/l400` por defecto) y se catalogan con xattrs.

| Tipo | Estado actual |
| --- | --- |
| `*LIB` | Directorio catalogado; soporte ZFS best-effort para datasets. |
| `*PGM` | ELF catalogado con atributo `C` o `CL`; toolchain marca version/firma simple. |
| `*FILE PF` | Storage `sled`, record length, schema, miembros y RRN/arrival sequence. |
| `*FILE LF` | Indice secundario sobre PF, mantenido por escrituras/borrados del PF. |
| `*FILE SRC` | Source file con miembros como archivos planos. |
| `*DTAQ` | Cola persistente FIFO con mensajes de longitud variable. |
| `*USRPRF` | Perfil administrable por comandos runtime. |
| `*CMD` | Comando promptable/documentable catalogado en QSYS. |
| `*OUTQ` | Output queue y spool basico para salida operativa. |
| `*SRVPGM` | Tipo reconocido; contrato de carga/linking pendiente. |

Metadatos principales actuales:

```text
user.l400.objtype
user.l400.objattr
user.l400.text
user.l400.owner
user.l400.owner_uid
user.l400.auth
user.l400.storage_backend
user.l400.record_len
user.l400.base_pf
```

## TUI

Flujo principal actual:

```text
SignOn -> MainMenu
        -> ObjectBrowser
        -> WorkManagement
        -> DataQueueViewer
        -> CommandLine
        -> SystemPanel
        -> STRPDM -> WRKMBRPDM -> STRSEU
                              -> STRSQL
```

Capacidades actuales:

- bloqueo de perfil `ROOT`;
- sesion con user profile, current library, library list, last message y job id;
- `SIGNOFF` limpia estado de sesion;
- `F4` en command line abre prompt por campos;
- opciones numericas en pantallas principales;
- confirmacion visual para borrado de objetos y terminacion de jobs;
- `WRKOBJ` puede abrir miembros, registros PF, descripcion y DTAQ;
- `WRKACTJOB` lista, filtra, muestra detalle y termina jobs;
- `STRSQL` ejecuta SQL interactivo, guarda historial, navega filas y desplaza columnas.

## Comandos actuales

El dispatcher `l400cmd` y los symlinks empaquetados cubren:

```text
WRKSYSSTS WRKACTJOB WRKJOB WRKJOBQ HLDJOB RLSJOB ENDJOB WRKSYSVAL DSPLOG
DSPCMD WRKCMD CRTCMD WRKUSRPRF WRKSPLF WRKOUTQ CRTOUTQ DLTOUTQ
DSPSPLF DLTSPLF PWRDWNSYS
WRKOBJ CRTLIB DLTLIB ADDLIBLE CHGCURLIB RNMOBJ DLTOBJ CPYOBJ
DSPOBJD CHGOBJD DSPOBJAUT CHKOBJAUT GRTOBJAUT RVKOBJAUT
CHKOBJINT DSPPOLICY DSPAUD
CRTPGM CRTCLPGM CALL SBMJOB
STRPDM STRSEU STRSQL WRKMBRPDM DLTMBR CPYMBR CHGMBRD GO SIGNOFF
CRTPF CRTLF DSPPFM CLRPFM ADDPFM WRTPFM
CRTDTAQ SNDDTAQ RCVDTAQ DSPDTAQ
```

`PWRDWNSYS` exige confirmacion explicita y root para ejecutar la accion real; `L400_PWRDWNSYS_DRY_RUN=1` permite smoke seguro.

## Work management

Estado actual:

- subsistemas base `QINTER` y `QBATCH`;
- registro de jobs en `L400_RUN_DIR/jobs`;
- estados `JOBQ`, `HELD`, `ACTIVE`, `ENDING`, `ENDED`, `COMPLETED`, `FAILED`, `KILLED`;
- salida/log por job batch;
- cgroups v2 best-effort;
- modo degradado si cgroups no estan disponibles;
- `SBMJOB` integrado como binario/comando.

## PF/LF/DTAQ y SQL

Estado actual:

- `CRTPF`, `CRTLF`, `DSPPFM`, `CLRPFM`, `ADDPFM`, `WRTPFM`;
- schema PF basico con campos y claves;
- LF actualizado automaticamente al escribir/borrar;
- DTAQ con `CRTDTAQ`, `SNDDTAQ`, `RCVDTAQ`, `DSPDTAQ`;
- `STRSQL` soporta `SELECT`, `INSERT`, `UPDATE`, `DELETE` y `CREATE TABLE` minimo;
- salida batch/stdin y pantalla interactiva.

## Toolchain

Estado actual de `clc`:

- parser Pest;
- AST con comandos, condiciones, `IF/ELSE`, `DO/ENDDO`, `MONMSG`;
- variables `DCL`, `CHGVAR`;
- parametros de programa;
- backend C por defecto;
- backend LLVM bajo feature;
- link contra `libl400`;
- catalogacion final como `*PGM`;
- `MONMSG` consulta estado CPF formal del runtime.

Estado actual de `c400c`:

- compila C con `clang`/`cc`;
- enlaza con `libl400`;
- cataloga como `*PGM`;
- marca toolchain en xattrs.

## Seguridad y politica

Estado actual:

- matriz runtime en `auth.rs` para `READ`, `CHANGE`, `EXECUTE`, `ADMIN`;
- autorizaciones `*USE`, `*CHANGE`, `*ALL`, `*EXCLUDE`;
- owner implicito con autoridad elevada;
- `CALL` verifica autoridad antes de ejecutar;
- auditoria en `QSYS/QHST` y, si existe, `QUSRSYS/QEZJOBLOG`;
- `DSPPOLICY`, `DSPAUD`, `DSPOBJAUT`, `CHKOBJAUT`, `GRTOBJAUT`, `RVKOBJAUT`;
- eBPF valida tipo, formato de `*PGM`, `*PUBLIC:*EXCLUDE`, owner UID y autoridad explicita `UID:<uid>`.
- `GRTOBJAUT` conserva grants por perfil (`QPGMR:*USE`) y agrega entradas espejo `UID:<uid>:*USE` cuando puede resolver el `*USRPRF`, para mantener paridad runtime/eBPF.

## Plataforma y release

Estado actual:

- build userspace con `scripts/build/build_userspace.sh`;
- build distribucion/ISO;
- instalador textual;
- initramfs live/install;
- `l400-support-report`;
- `test_release_rc.sh` como gate minimo;
- QEMU install smoke disponible con `RUN_E2E_INSTALL=1`.

Comandos de validacion usados como base:

```bash
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
./scripts/test/test_release_rc.sh
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

## Limitaciones actuales

Estas limitaciones no son el plan; son la foto actual del sistema:

- la compatibilidad IBM i es semantica y parcial, no binaria;
- `*SRVPGM` esta reconocido pero aun no tiene contrato de carga/linking;
- las marcas de toolchain son simples y no equivalen a firma criptografica;
- el enforcement eBPF de autoridad cubre ejecucion, pero no todo `file_open` por perfil/grupo;
- los tests interactivos de TUI son mayormente unitarios/smoke, no una suite 5250 completa;
- la validacion QEMU depende de host con QEMU/OVMF disponible.
