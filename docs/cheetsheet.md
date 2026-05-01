# Linux/400 Cheatsheet de Comandos

Referencia rapida de comandos estilo OS/400 para operar y administrar Linux/400. Los comandos pueden ejecutarse desde la linea de comandos de la TUI, como binarios/symlinks en `PATH`, o con `l400cmd <CMD> [parametros]`.

Formas de parametros aceptadas por el dispatcher actual:

```bash
CRTLIB QGPL
CRTLIB LIB(QGPL)
CRTLIB LIB=QGPL
```

## Sesion y navegacion

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `GO` | Abre un menu por nombre. | `GO MAIN` |
| `SIGNOFF` | Cierra la sesion actual. | `SIGNOFF` |
| `HELP` | Muestra ayuda basica en la linea de comandos de la TUI. | `HELP` |
| `CALL` | Ejecuta un programa catalogado resolviendo `CURLIB` y `LIBLIST`. | `CALL PGM(QGPL/MYPGM)` |

Teclas frecuentes en la TUI:

```text
F3=Exit/Save
F4=Prompt/Command line
F5=Refresh/Clear
F6=Create
F12=Cancel
Enter=Select/Run
```

En la linea de comandos, `F4` abre prompt por campos; `Tab`/`Shift-Tab` cambia parametro y `Enter` ejecuta.

## Sistema

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `WRKSYSSTS` | Muestra estado general: carga, memoria, uptime y resumen de trabajos. | `WRKSYSSTS` |
| `WRKSYSVAL` | Muestra valores de sistema relevantes para Linux/400. | `WRKSYSVAL` |
| `CHGSYSVAL` | Cambia un valor de sistema. | `CHGSYSVAL SYSVAL(QAUTOCFG) VALUE(*NO)` |
| `DSPLOG` | Muestra mensajes recientes del sistema o QHST equivalente. | `DSPLOG` |
| `PWRDWNSYS` | Solicita apagado o reinicio del sistema; accion real requiere root y confirmacion. | `PWRDWNSYS OPTION(*IMMED) CONFIRM(*YES)` |
| `l400-bootstrap` | Inicializa bibliotecas y objetos base del catalogo Linux/400. | `l400-bootstrap --root /l400` |
| `l400-support-report` | Reporta capacidades de plataforma: loader, BPF, cgroups, ZFS/xattrs. | `l400-support-report --write` |

## Bibliotecas

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `CRTLIB` | Crea una biblioteca `*LIB`. | `CRTLIB LIB(MYLIB)` |
| `DLTLIB` | Elimina una biblioteca. | `DLTLIB LIB(MYLIB)` |
| `WRKLIB` | Lista y trabaja con bibliotecas. | `WRKLIB LIB(Q*)` |
| `DSPLIB` | Muestra informacion de una biblioteca. | `DSPLIB LIB(QGPL)` |
| `ADDLIBLE` | Agrega una biblioteca a la library list de la sesion. | `ADDLIBLE LIB(QGPL)` |
| `RMVLIBLE` | Quita una biblioteca de la library list. | `RMVLIBLE LIB(QGPL)` |
| `CHGCURLIB` | Cambia la biblioteca actual. | `CHGCURLIB CURLIB(QGPL)` |
| `DSPLIBL` | Muestra la library list actual. | `DSPLIBL` |

## Objetos

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `WRKOBJ` | Lista objetos por nombre, biblioteca o tipo. | `WRKOBJ OBJ(QSYS/*ALL) OBJTYPE(*PGM)` |
| `DSPOBJD` | Muestra descripcion y metadatos de un objeto. | `DSPOBJD OBJ(QGPL/MYPGM) OBJTYPE(*PGM)` |
| `CRTDUPOBJ` | Duplica un objeto dentro de una biblioteca o hacia otra. | `CRTDUPOBJ OBJ(A) FROMLIB(QGPL) OBJTYPE(*PGM) TOLIB(TEST)` |
| `CPYOBJ` | Copia un objeto preservando metadatos Linux/400. | `CPYOBJ OBJ(QGPL/A) TOOBJ(QGPL/B)` |
| `DLTOBJ` | Elimina un objeto catalogado. | `DLTOBJ OBJ(QGPL/OLDPGM) OBJTYPE(*PGM) CONFIRM(*YES)` |
| `RNMOBJ` | Renombra un objeto. | `RNMOBJ OBJ(OLDPGM) NEWNAME(NEWPGM)` |
| `CHGOBJD` | Cambia texto u otros metadatos de objeto. | `CHGOBJD OBJ(QGPL/MYPGM) TEXT('Demo')` |
| `WRKOBJOWN` | Lista objetos por propietario. | `WRKOBJOWN USER(QSECOFR)` |

## Autorizaciones

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `DSPOBJAUT` | Muestra autorizaciones de un objeto. | `DSPOBJAUT OBJ(QGPL/MYPGM) OBJTYPE(*PGM)` |
| `CHKOBJAUT` | Verifica una autorizacion efectiva para usuario/comando. | `CHKOBJAUT OBJ(QGPL/MYPGM) USER(QPGMR) AUT(*USE)` |
| `CHKOBJINT` | Verifica metadatos minimos e integridad basica de un objeto. | `CHKOBJINT OBJ(QGPL/MYFILE)` |
| `GRTOBJAUT` | Otorga autorizacion sobre un objeto. | `GRTOBJAUT OBJ(QGPL/MYPGM) USER(QPGMR) AUT(*USE)` |
| `RVKOBJAUT` | Revoca autorizacion sobre un objeto. | `RVKOBJAUT OBJ(QGPL/MYPGM) USER(QPGMR)` |
| `DSPPOLICY` | Muestra la matriz runtime/eBPF de politica de seguridad. | `DSPPOLICY` |
| `DSPAUD` | Muestra los ultimos eventos auditados en QHST/DTAQ. | `DSPAUD` |
| `CHGOWN` | Cambia propietario logico de un objeto. | `CHGOWN OBJ(QGPL/MYFILE) OWNER(QPGMR)` |

Autoridades comunes:

```text
*USE
*CHANGE
*ALL
*EXCLUDE
```

## Usuarios y perfiles

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `WRKUSRPRF` | Lista, muestra, crea o desactiva perfiles de usuario Linux/400. | `WRKUSRPRF USRPRF(TESTUSR) ACTION(*CREATE)` |
| `CRTUSRPRF` | Crea un perfil de usuario Linux/400. | `CRTUSRPRF USRPRF(QPGMR)` |
| `CHGUSRPRF` | Cambia atributos de un perfil. | `CHGUSRPRF USRPRF(QPGMR) STATUS(*DISABLED)` |
| `DLTUSRPRF` | Elimina un perfil. | `DLTUSRPRF USRPRF(TESTUSR)` |
| `DSPUSRPRF` | Muestra detalle de un perfil. | `DSPUSRPRF USRPRF(QSECOFR)` |

## Trabajos y subsistemas

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `WRKACTJOB` | Lista trabajos activos o registrados; permite filtrar y ver detalle. | `WRKACTJOB SBS(QBATCH) STATUS(ACTIVE)` |
| `WRKACTJOB OPTION(*DETAIL)` | Muestra detalle de un trabajo por `PID` o `JOB`. | `WRKACTJOB JOB(MYJOB) OPTION(*DETAIL)` |
| `WRKACTJOB OPTION(*END)` | Termina un trabajo activo por `PID` o `JOB`. | `WRKACTJOB PID(1234) OPTION(*END)` |
| `SBMJOB` | Envia un comando ejecutable a batch. | `SBMJOB CMD(WRKSYSSTS) JOB(MYJOB) JOBQ(QBATCH)` |
| `WRKJOBQ` | Lista trabajos retenidos o en cola. | `WRKJOBQ` |
| `HLDJOB` | Retiene un trabajo por nombre o PID. | `HLDJOB JOB(MYJOB)` |
| `RLSJOB` | Libera un trabajo retenido. | `RLSJOB JOB(MYJOB)` |
| `WRKJOB` | Muestra detalle de un trabajo. | `WRKJOB JOB(1234/QSECOFR/MYJOB)` |
| `ENDJOB` | Finaliza un trabajo. | `ENDJOB PID(1234) CONFIRM(*YES)` |
| `WRKJOBQ` | Trabaja con colas de trabajos. | `WRKJOBQ JOBQ(QBATCH)` |
| `WRKSBS` | Lista subsistemas. | `WRKSBS` |
| `STRSBS` | Inicia un subsistema. | `STRSBS SBSD(QINTER)` |
| `ENDSBS` | Finaliza un subsistema. | `ENDSBS SBS(QBATCH) OPTION(*CNTRLD)` |

Subsistemas base:

```text
QINTER  Sesiones interactivas/TUI
QBATCH  Trabajos batch
```

## Archivos PF/LF

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `CRTPF` | Crea un archivo fisico `*FILE PF`. | `CRTPF FILE(QGPL/CUSTOMERS) RCDLEN(128)` |
| `CRTLF` | Crea un archivo logico `*FILE LF` sobre un PF. | `CRTLF FILE(QGPL/CUSTBYNAME) SRCFILE(QGPL/CUSTOMERS)` |
| `DSPPFM` | Muestra miembros o registros de un PF. | `DSPPFM FILE(QGPL/CUSTOMERS)` |
| `CLRPFM` | Limpia un miembro de PF. | `CLRPFM FILE(QGPL/CUSTOMERS)` |
| `ADDPFM` | Agrega un miembro a un PF. | `ADDPFM FILE(QGPL/CUSTOMERS) MBR(JAN2026)` |
| `WRTPFM` | Escribe un registro en un PF por clave o con RRN automatico. | `WRTPFM FILE(QGPL/CUSTOMERS) KEY(C001) DATA(ALICE)` |
| `DLTMBR` | Elimina un miembro con confirmacion. | `DLTMBR FILE(QGPL/QCLSRC) MBR(OLD.CLP) CONFIRM(*YES)` |
| `CPYMBR` | Copia un miembro. | `CPYMBR FILE(QGPL/QCLSRC) MBR(A.CLP) TOMBR(B.CLP)` |
| `CHGMBRD` | Cambia texto de un miembro. | `CHGMBRD FILE(QGPL/QCLSRC) MBR(A.CLP) TEXT(Demo)` |
| `CPYF` | Copia registros entre archivos. | `CPYF FROMFILE(QGPL/A) TOFILE(QGPL/B)` |
| `RUNQRY` | Ejecuta una consulta simple sobre un archivo. | `RUNQRY QRYFILE(QGPL/CUSTOMERS)` |

## Source members y desarrollo

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `STRPDM` | Abre el Programming Development Manager. | `STRPDM` |
| `WRKMBRPDM` | Lista miembros de un source file. | `WRKMBRPDM FILE(QGPL/QCLSRC)` |
| `STRSEU` | Edita o muestra un miembro fuente. | `STRSEU FILE(QGPL/QCLSRC) MBR(HELLO.CLP)` |
| `STRSQL` | Abre SQL por stdin o ejecuta una sentencia `SELECT/INSERT/UPDATE/DELETE/CREATE TABLE`. | `STRSQL "SELECT * FROM QGPL/CUSTOMERS WHERE KEY='C001'"` |
| `CRTCLPGM` | Compila un miembro CL como `*PGM`. | `CRTCLPGM PGM(QGPL/HELLO) SRCFILE(QGPL/QCLSRC) SRCMBR(HELLO)` |
| `CRTPGM` | Cataloga o crea un objeto programa `*PGM`. | `CRTPGM PGM(QGPL/HELLO)` |
| `DLTPGM` | Elimina un programa. | `DLTPGM PGM(QGPL/HELLO)` |

Comandos de compilador disponibles desde shell de desarrollo:

```bash
clc --input tests/prueba.clp --output /l400/QSYS/HELLOCL
c400c --input tests/hola_mundo.c --output /l400/QSYS/HELLOC
```

## Data queues

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `CRTDTAQ` | Crea una data queue `*DTAQ`. | `CRTDTAQ DTAQ(QUSRSYS/QEZJOBLOG)` |
| `DLTDTAQ` | Elimina una data queue. | `DLTDTAQ DTAQ(QUSRSYS/QEZJOBLOG)` |
| `SNDDTAQ` | Envia un mensaje a una data queue. | `SNDDTAQ DTAQ(QUSRSYS/QEZJOBLOG) MSG(JobStarted)` |
| `RCVDTAQ` | Recibe un mensaje de una data queue. | `RCVDTAQ DTAQ(QUSRSYS/QEZJOBLOG) WAIT(0)` |
| `DSPDTAQ` | Muestra contenido de una data queue. | `DSPDTAQ DTAQ(QUSRSYS/QEZJOBLOG)` |

## Opciones TUI utiles

| Pantalla | Opcion/tecla | Accion |
| --- | --- | --- |
| `WRKOBJ` | `3` | Muestra registros PF con `DSPPFM`. |
| `WRKOBJ` | `4` | Solicita confirmacion visual y borra el objeto seleccionado. |
| `WRKOBJ` | `8` | Abre visor de `*DTAQ`. |
| `WRKACTJOB` | `4` o `F10` | Solicita confirmacion visual y termina el job seleccionado. |
| `WRKACTJOB` | `9` | Muestra las ultimas lineas del log del job. |
| `ObjectDetail` | `2/3/4/8` | Cambia texto, copia, borra con confirmacion o muestra autorizaciones. |
| `PolicyAudit` | `1/2/0` | Filtra denegados, cambios de usuarios o vuelve a todos los eventos. |
| `SpoolOutq` | `5` | Muestra el primer spool file disponible. |
| `STRSQL` | `F7/F8` | Desplaza columnas de resultados. |

## Output queues y spool

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `WRKOUTQ` | Lista o trabaja con output queues. | `WRKOUTQ OUTQ(QPRINT)` |
| `CRTOUTQ` | Crea una output queue `*OUTQ`. | `CRTOUTQ OUTQ(QGPL/QPRINT)` |
| `DLTOUTQ` | Elimina una output queue. | `DLTOUTQ OUTQ(QGPL/QPRINT)` |
| `WRKSPLF` | Lista spool files. | `WRKSPLF SELECT(*CURRENT)` |
| `DSPSPLF` | Muestra un spool file. | `DSPSPLF FILE(QPRINT)` |
| `DLTSPLF` | Elimina un spool file. | `DLTSPLF FILE(QPRINT)` |

## Loader y politica de objetos

| Comando | Descripcion | Ejemplo |
| --- | --- | --- |
| `l400-loader` | Carga la politica eBPF LSM. | `l400-loader --mode full --once` |
| `l400-loader --mode degraded` | Intenta cargar politica y continua si no puede. | `l400-loader --mode degraded --once` |
| `l400-loader --mode dev` | Modo tolerante para desarrollo. | `l400-loader --mode dev --once` |
| `l400-support-report` | Muestra modo efectivo y capacidades de enforcement. | `l400-support-report` |
| `l400-upgrade-check` | Valida version metadata, xattrs, persistencia y backup recomendado. | `l400-upgrade-check` |
| `l400-migrate` | Aplica migraciones versionadas de `/l400`. | `l400-migrate` |

## Operacion diaria guiada

1. Iniciar sesion en la TUI y entrar a `GO MAIN`.
2. Revisar salud con `WRKSYSSTS` y `l400-support-report --write`.
3. Trabajar objetos con `WRKOBJ`, usando `5=Display`, `8=Authorities` y `4=Delete`.
4. Revisar jobs con `WRKACTJOB`; usar `9=Log` para diagnosticar batch.
5. Revisar salidas con `WRKSPLF` o la opcion de menu `Spool files`.
6. Antes de upgrade: `l400-upgrade-check`, backup con `rsync -aX`, luego `l400-migrate`.

## Variables de entorno utiles

| Variable | Descripcion | Ejemplo |
| --- | --- | --- |
| `L400_ROOT` | Root logico de objetos. | `export L400_ROOT=/l400` |
| `L400_RUN_DIR` | Estado runtime: jobs, loader-status, support-profile. | `export L400_RUN_DIR=/run/l400` |
| `L400_CURLIB` | Biblioteca actual. | `export L400_CURLIB=QGPL` |
| `L400_LIBLIST` | Lista de bibliotecas de busqueda. | `export L400_LIBLIST=QGPL:QSYS` |
| `L400_STORAGE_BACKEND` | Backend para PF/LF/DTAQ. | `export L400_STORAGE_BACKEND=sled` |
| `L400_BPF_PATH` | Ruta al artefacto eBPF. | `export L400_BPF_PATH=/opt/l400/hooks/l400-ebpf` |
