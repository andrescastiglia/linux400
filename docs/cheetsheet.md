# Linux/400 Cheatsheet de Comandos por Rol

Referencia rapida de comandos estilo AS/400 para operar, administrar y programar en Linux/400. Los comandos implementados pueden ejecutarse desde la linea de comandos de la TUI, como binarios/symlinks en `PATH`, o con `l400cmd <CMD> [parametros]`.

Formas de parametros aceptadas por el dispatcher actual:

```bash
CRTLIB QGPL
CRTLIB LIB(QGPL)
CRTLIB LIB=QGPL
```

## Estado

| Estado | Significado |
| --- | --- |
| Implementado | Existe en `l400cmd`, TUI o script actual. |
| Parcial | Existe, pero todavia cubre un subconjunto de AS/400. |
| Script | Existe como herramienta Linux/400 fuera del dispatcher de comandos. |
| Objetivo V1 | Debe agregarse para la operacion basica de Version 1. |
| Objetivo V2 | Debe agregarse para programacion ampliada, RPG o SQL avanzado. |

## Teclas TUI frecuentes

```text
F3=Exit/Save
F4=Prompt/Command line
F5=Refresh/Clear
F6=Create
F12=Cancel
Enter=Select/Run
```

En la linea de comandos, `F4` abre prompt por campos; `Tab`/`Shift-Tab` cambia parametro y `Enter` ejecuta.

## Rol: operador

El operador mantiene el sistema en marcha: revisa salud, sesiones, jobs, colas, spool, logs, backups basicos y apagado controlado.

### Sesion y navegacion

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `GO` | Implementado | Abre un menu por nombre. | `GO MAIN` |
| `SIGNOFF` | Implementado | Cierra la sesion actual. | `SIGNOFF` |
| `HELP` | Parcial | Muestra ayuda basica en la linea de comandos de la TUI. | `HELP` |
| `CALL` | Implementado | Ejecuta un programa catalogado resolviendo `CURLIB` y `LIBLIST`. | `CALL PGM(QGPL/MYPGM)` |

### Salud del sistema

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `WRKSYSSTS` | Implementado | Muestra carga, memoria, uptime y resumen de trabajos. | `WRKSYSSTS` |
| `WRKSYSVAL` | Implementado | Muestra valores de sistema relevantes para Linux/400. | `WRKSYSVAL` |
| `DSPLOG` | Implementado | Muestra mensajes recientes del sistema o QHST equivalente. | `DSPLOG` |
| `DSPAUD` | Implementado | Muestra eventos de auditoria recientes. | `DSPAUD` |
| `DSPPOLICY` | Implementado | Muestra modo de politica runtime/eBPF. | `DSPPOLICY` |
| `PWRDWNSYS` | Implementado | Apaga o reinicia con confirmacion y autoridad. | `PWRDWNSYS OPTION(*RESTART) CONFIRM(*YES)` |
| `WRKCFGSTS` | Objetivo V1 | Trabaja con estado de dispositivos/servicios configurados. | `WRKCFGSTS CFGTYPE(*DEV)` |
| `DSPMSG` | Objetivo V1 | Muestra mensajes de una cola de mensajes. | `DSPMSG MSGQ(QSYSOPR)` |
| `SNDMSG` | Objetivo V1 | Envia mensaje a usuario o cola. | `SNDMSG MSG('Backup listo') TOUSR(QSYSOPR)` |

### Jobs, colas y subsistemas

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `WRKACTJOB` | Implementado | Lista trabajos activos o registrados. | `WRKACTJOB SBS(QBATCH) STATUS(ACTIVE)` |
| `WRKJOB` | Implementado | Muestra detalle de un trabajo. | `WRKJOB JOB(MYJOB)` |
| `WRKJOBQ` | Implementado | Lista trabajos retenidos o en cola. | `WRKJOBQ JOBQ(QBATCH)` |
| `SBMJOB` | Implementado | Envia un comando a batch. | `SBMJOB CMD(WRKSYSSTS) JOB(MYSTS) JOBQ(QBATCH)` |
| `HLDJOB` | Implementado | Retiene un trabajo. | `HLDJOB JOB(MYJOB)` |
| `RLSJOB` | Implementado | Libera un trabajo retenido. | `RLSJOB JOB(MYJOB)` |
| `ENDJOB` | Implementado | Finaliza un trabajo con confirmacion. | `ENDJOB PID(1234) CONFIRM(*YES)` |
| `WRKSBS` | Objetivo V1 | Lista y trabaja con subsistemas. | `WRKSBS` |
| `STRSBS` | Objetivo V1 | Inicia un subsistema. | `STRSBS SBSD(QINTER)` |
| `ENDSBS` | Objetivo V1 | Finaliza un subsistema. | `ENDSBS SBS(QBATCH) OPTION(*CNTRLD)` |
| `HLDJOBQ` | Objetivo V1 | Retiene una cola de trabajos. | `HLDJOBQ JOBQ(QBATCH)` |
| `RLSJOBQ` | Objetivo V1 | Libera una cola de trabajos. | `RLSJOBQ JOBQ(QBATCH)` |

Subsistemas base:

```text
QINTER  Sesiones interactivas/TUI
QBATCH  Trabajos batch
```

### Spool y output queues

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `WRKSPLF` | Implementado | Lista spool files. | `WRKSPLF SELECT(*CURRENT)` |
| `DSPSPLF` | Implementado | Muestra un spool file. | `DSPSPLF FILE(QPRINT)` |
| `CHGSPLFA` | Implementado | Cambia atributos basicos de spool. | `CHGSPLFA FILE(QPRINT) STATUS(*HELD)` |
| `DLTSPLF` | Implementado | Elimina un spool file con confirmacion. | `DLTSPLF FILE(QPRINT) CONFIRM(*YES)` |
| `WRKOUTQ` | Implementado | Lista output queues. | `WRKOUTQ OUTQ(QPRINT)` |
| `CRTOUTQ` | Implementado | Crea una output queue `*OUTQ`. | `CRTOUTQ OUTQ(QGPL/QPRINT)` |
| `DLTOUTQ` | Implementado | Elimina una output queue. | `DLTOUTQ OUTQ(QGPL/QPRINT) CONFIRM(*YES)` |
| `HLDSPLF` | Objetivo V1 | Retiene un spool file. | `HLDSPLF FILE(QPRINT)` |
| `RLSSPLF` | Objetivo V1 | Libera un spool file. | `RLSSPLF FILE(QPRINT)` |
| `HLDOUTQ` | Objetivo V1 | Retiene una output queue. | `HLDOUTQ OUTQ(QGPL/QPRINT)` |
| `RLSOUTQ` | Objetivo V1 | Libera una output queue. | `RLSOUTQ OUTQ(QGPL/QPRINT)` |
| `STRPRTWTR` | Objetivo V1 | Inicia writer/exportador de salida. | `STRPRTWTR OUTQ(QGPL/QPRINT)` |
| `ENDWTR` | Objetivo V1 | Finaliza writer/exportador. | `ENDWTR WTR(QPRINT)` |

### Backup, restore y mantenimiento

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `l400-upgrade-check` | Script | Valida metadata, xattrs, persistencia y backup recomendado. | `l400-upgrade-check` |
| `l400-migrate` | Script | Aplica migraciones versionadas de `/l400`. | `l400-migrate` |
| `l400-support-report` | Script | Reporta plataforma, loader, BPF, cgroups y xattrs. | `l400-support-report --write` |
| `SAVSYS` | Objetivo V1 | Respalda el sistema Linux/400 completo. | `SAVSYS DEV('/backup/l400.tar')` |
| `SAVLIB` | Objetivo V1 | Respalda una biblioteca. | `SAVLIB LIB(QGPL) DEV('/backup/qgpl.tar')` |
| `SAVOBJ` | Objetivo V1 | Respalda objetos seleccionados. | `SAVOBJ OBJ(MYFILE) LIB(QGPL) DEV('/backup/obj.tar')` |
| `RSTSYS` | Objetivo V1 | Restaura un backup de sistema. | `RSTSYS DEV('/backup/l400.tar')` |
| `RSTLIB` | Objetivo V1 | Restaura una biblioteca. | `RSTLIB LIB(QGPL) DEV('/backup/qgpl.tar')` |
| `RSTOBJ` | Objetivo V1 | Restaura objetos seleccionados. | `RSTOBJ OBJ(MYFILE) LIB(QGPL) DEV('/backup/obj.tar')` |
| `DSPPTF` | Objetivo V1 | Lista PTFs aplicados o pendientes. | `DSPPTF` |
| `APYPTF` | Objetivo V1 | Aplica, verifica o revierte un PTF. | `APYPTF LICPGM(L400) SELECT(L4000001) OPTION(*APPLY)` |

Flujo actual recomendado antes de upgrades:

```bash
l400-upgrade-check
rsync -aX /l400/ /backup/l400/
l400-migrate
```

## Rol: administrador

El administrador define usuarios, seguridad, objetos base, bibliotecas, valores de sistema, comandos catalogados y politica de plataforma.

### Usuarios y perfiles

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `WRKUSRPRF` | Parcial | Lista, muestra, crea o desactiva perfiles Linux/400. | `WRKUSRPRF USRPRF(TESTUSR) ACTION(*CREATE)` |
| `CRTUSRPRF` | Objetivo V1 | Crea un perfil de usuario. | `CRTUSRPRF USRPRF(QPGMR)` |
| `CHGUSRPRF` | Objetivo V1 | Cambia atributos de un perfil. | `CHGUSRPRF USRPRF(QPGMR) STATUS(*DISABLED)` |
| `DLTUSRPRF` | Objetivo V1 | Elimina un perfil. | `DLTUSRPRF USRPRF(TESTUSR)` |
| `DSPUSRPRF` | Objetivo V1 | Muestra detalle de un perfil. | `DSPUSRPRF USRPRF(QSECOFR)` |
| `CHGPWD` | Objetivo V1 | Cambia password de perfil. | `CHGPWD USRPRF(QPGMR)` |
| `WRKUSRJOB` | Objetivo V1 | Lista trabajos de un usuario. | `WRKUSRJOB USER(QPGMR)` |

### Autorizaciones y auditoria

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `DSPOBJAUT` | Implementado | Muestra autorizaciones de un objeto. | `DSPOBJAUT OBJ(QGPL/MYPGM) OBJTYPE(*PGM)` |
| `CHKOBJAUT` | Implementado | Verifica autorizacion efectiva. | `CHKOBJAUT OBJ(QGPL/MYPGM) USER(QPGMR) AUT(*USE)` |
| `GRTOBJAUT` | Implementado | Otorga autorizacion sobre un objeto. | `GRTOBJAUT OBJ(QGPL/MYPGM) USER(QPGMR) AUT(*USE)` |
| `RVKOBJAUT` | Implementado | Revoca autorizacion sobre un objeto. | `RVKOBJAUT OBJ(QGPL/MYPGM) USER(QPGMR)` |
| `CHKOBJINT` | Implementado | Verifica metadata e integridad basica de objeto. | `CHKOBJINT OBJ(QGPL/MYFILE)` |
| `DSPAUD` | Implementado | Muestra auditoria runtime. | `DSPAUD` |
| `DSPPOLICY` | Implementado | Muestra politica runtime/eBPF. | `DSPPOLICY` |
| `CHGOWN` | Objetivo V1 | Cambia owner logico de un objeto. | `CHGOWN OBJ(QGPL/MYFILE) OWNER(QPGMR)` |
| `WRKOBJOWN` | Objetivo V1 | Lista objetos por propietario. | `WRKOBJOWN USER(QSECOFR)` |
| `CHGAUD` | Objetivo V1 | Cambia atributos de auditoria. | `CHGAUD OBJ(QGPL/MYPGM) AUDLVL(*CHANGE)` |

Autoridades comunes:

```text
*USE
*CHANGE
*ALL
*EXCLUDE
```

### Bibliotecas, objetos y comandos

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `CRTLIB` | Implementado | Crea una biblioteca `*LIB`. | `CRTLIB LIB(MYLIB)` |
| `DLTLIB` | Implementado | Elimina una biblioteca. | `DLTLIB LIB(MYLIB)` |
| `ADDLIBLE` | Implementado | Agrega biblioteca a la library list. | `ADDLIBLE LIB(QGPL)` |
| `CHGCURLIB` | Implementado | Cambia la biblioteca actual. | `CHGCURLIB CURLIB(QGPL)` |
| `WRKOBJ` | Implementado | Lista objetos por nombre, biblioteca o tipo. | `WRKOBJ OBJ(QSYS/*ALL) OBJTYPE(*PGM)` |
| `DSPOBJD` | Implementado | Muestra descripcion y metadata de objeto. | `DSPOBJD OBJ(QGPL/MYPGM) OBJTYPE(*PGM)` |
| `CPYOBJ` | Implementado | Copia objeto preservando metadata. | `CPYOBJ OBJ(QGPL/A) TOOBJ(QGPL/B)` |
| `DLTOBJ` | Implementado | Elimina un objeto catalogado. | `DLTOBJ OBJ(QGPL/OLDPGM) OBJTYPE(*PGM) CONFIRM(*YES)` |
| `RNMOBJ` | Implementado | Renombra un objeto. | `RNMOBJ OBJ(OLDPGM) NEWNAME(NEWPGM)` |
| `CHGOBJD` | Implementado | Cambia texto o atributos de objeto. | `CHGOBJD OBJ(QGPL/MYPGM) TEXT('Demo')` |
| `DSPCMD` | Implementado | Muestra metadata de un comando. | `DSPCMD CMD(WRKOBJ)` |
| `WRKCMD` | Implementado | Lista comandos catalogados por nombre/estado/autoridad. | `WRKCMD CMD(WRK*)` |
| `CRTCMD` | Parcial | Cataloga un comando `*CMD`. | `CRTCMD CMD(QSYS/MYCMD) TEXT('Demo')` |
| `WRKLIB` | Objetivo V1 | Lista y trabaja con bibliotecas. | `WRKLIB LIB(Q*)` |
| `DSPLIB` | Objetivo V1 | Muestra informacion de biblioteca. | `DSPLIB LIB(QGPL)` |
| `DSPLIBL` | Objetivo V1 | Muestra library list actual. | `DSPLIBL` |
| `RMVLIBLE` | Objetivo V1 | Quita biblioteca de la library list. | `RMVLIBLE LIB(QGPL)` |
| `CRTDUPOBJ` | Objetivo V1 | Duplica un objeto con semantica AS/400. | `CRTDUPOBJ OBJ(A) FROMLIB(QGPL) OBJTYPE(*PGM) TOLIB(TEST)` |
| `WRKOBJLCK` | Objetivo V1 | Muestra locks de objeto. | `WRKOBJLCK OBJ(QGPL/MYFILE) OBJTYPE(*FILE)` |

### Valores de sistema y plataforma

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `WRKSYSVAL` | Implementado | Lista valores de sistema conocidos. | `WRKSYSVAL` |
| `CHGSYSVAL` | Objetivo V1 | Cambia un valor de sistema. | `CHGSYSVAL SYSVAL(QAUTOCFG) VALUE(*NO)` |
| `l400-bootstrap` | Script | Inicializa bibliotecas y objetos base. | `l400-bootstrap --root /l400` |
| `l400-loader` | Script | Carga la politica eBPF LSM. | `l400-loader --mode full --once` |
| `l400-loader --mode degraded` | Script | Intenta cargar politica y continua si no puede. | `l400-loader --mode degraded --once` |
| `l400-loader --mode dev` | Script | Modo tolerante para desarrollo local. | `l400-loader --mode dev --once` |
| `l400-support-report` | Script | Muestra modo efectivo y capacidades. | `l400-support-report` |

## Rol: programador

El programador trabaja con miembros fuente, compilacion, programas, PF/LF, DTAQ, SQL y ejecucion interactiva/batch.

### Desarrollo interactivo

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `STRPDM` | Implementado | Abre el Programming Development Manager. | `STRPDM` |
| `WRKMBRPDM` | Implementado | Lista miembros de un source file. | `WRKMBRPDM FILE(QGPL/QCLSRC)` |
| `STRSEU` | Implementado | Edita o muestra un miembro fuente. | `STRSEU FILE(QGPL/QCLSRC) MBR(HELLO.CLP)` |
| `STRSQL` | Parcial | Abre SQL o ejecuta `SELECT/INSERT/UPDATE/DELETE/CREATE TABLE` minimo. | `STRSQL "SELECT * FROM QGPL/CUSTOMERS"` |
| `DLTMBR` | Implementado | Elimina un miembro con confirmacion. | `DLTMBR FILE(QGPL/QCLSRC) MBR(OLD.CLP) CONFIRM(*YES)` |
| `CPYMBR` | Implementado | Copia un miembro. | `CPYMBR FILE(QGPL/QCLSRC) MBR(A.CLP) TOMBR(B.CLP)` |
| `CHGMBRD` | Implementado | Cambia texto de un miembro. | `CHGMBRD FILE(QGPL/QCLSRC) MBR(A.CLP) TEXT(Demo)` |
| `DSPFD` | Objetivo V1 | Muestra descripcion de archivo. | `DSPFD FILE(QGPL/CUSTOMERS)` |
| `DSPFFD` | Objetivo V1 | Muestra descripcion de campos. | `DSPFFD FILE(QGPL/CUSTOMERS)` |

### Compilacion y programas

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `CRTCLPGM` | Implementado | Compila un miembro CL como `*PGM`. | `CRTCLPGM PGM(QGPL/HELLO) SRCFILE(QGPL/QCLSRC) SRCMBR(HELLO)` |
| `CRTPGM` | Implementado | Cataloga o crea un objeto programa `*PGM`. | `CRTPGM PGM(QGPL/HELLO)` |
| `CALL` | Implementado | Ejecuta un programa catalogado. | `CALL PGM(QGPL/HELLO)` |
| `SBMJOB` | Implementado | Ejecuta un comando/programa en batch. | `SBMJOB CMD(CALL PGM(QGPL/HELLO)) JOB(HELLO)` |
| `DLTPGM` | Objetivo V1 | Elimina un programa con semantica AS/400. | `DLTPGM PGM(QGPL/HELLO)` |
| `DSPPGM` | Objetivo V1 | Muestra descripcion de programa. | `DSPPGM PGM(QGPL/HELLO)` |
| `CRTRPGPGM` | Objetivo V2 | Compila RPG como `*PGM`. | `CRTRPGPGM PGM(QGPL/INVOICE) SRCFILE(QGPL/QRPGSRC)` |
| `CRTSQLRPGI` | Objetivo V2 | Compila RPG con SQL embebido. | `CRTSQLRPGI OBJ(QGPL/INVOICE) SRCFILE(QGPL/QRPGLESRC)` |
| `CRTSQLCI` | Objetivo V2 | Compila C con SQL embebido. | `CRTSQLCI OBJ(QGPL/MYSQLC) SRCFILE(QGPL/QCSRC)` |

Compiladores disponibles desde shell de desarrollo:

```bash
clc --input tests/prueba.clp --output /l400/QSYS/HELLOCL
c400c --input tests/hola_mundo.c --output /l400/QSYS/HELLOC
```

### Archivos PF/LF y datos

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `CRTPF` | Implementado | Crea un archivo fisico `*FILE PF`. | `CRTPF FILE(QGPL/CUSTOMERS) RCDLEN(128)` |
| `CRTLF` | Implementado | Crea un archivo logico `*FILE LF` sobre un PF. | `CRTLF FILE(QGPL/CUSTBYNAME) SRCFILE(QGPL/CUSTOMERS)` |
| `DSPPFM` | Implementado | Muestra miembros o registros de un PF. | `DSPPFM FILE(QGPL/CUSTOMERS)` |
| `CLRPFM` | Implementado | Limpia un miembro de PF. | `CLRPFM FILE(QGPL/CUSTOMERS) CONFIRM(*YES)` |
| `ADDPFM` | Implementado | Agrega un miembro a un PF. | `ADDPFM FILE(QGPL/CUSTOMERS) MBR(JAN2026)` |
| `WRTPFM` | Implementado | Escribe un registro por clave o RRN automatico. | `WRTPFM FILE(QGPL/CUSTOMERS) KEY(C001) DATA(ALICE)` |
| `CPYF` | Objetivo V1 | Copia registros entre archivos. | `CPYF FROMFILE(QGPL/A) TOFILE(QGPL/B)` |
| `RUNQRY` | Objetivo V1 | Ejecuta una consulta simple sobre archivo. | `RUNQRY QRYFILE(QGPL/CUSTOMERS)` |
| `OPNQRYF` | Objetivo V2 | Abre vista/query estilo AS/400. | `OPNQRYF FILE((QGPL/CUSTOMERS)) QRYSLT('KEY *EQ C001')` |

### Data queues

| Comando | Estado | Descripcion | Ejemplo |
| --- | --- | --- | --- |
| `CRTDTAQ` | Implementado | Crea una data queue `*DTAQ`. | `CRTDTAQ DTAQ(QUSRSYS/QEZJOBLOG)` |
| `SNDDTAQ` | Implementado | Envia un mensaje a una data queue. | `SNDDTAQ DTAQ(QUSRSYS/QEZJOBLOG) MSG(JobStarted)` |
| `RCVDTAQ` | Implementado | Recibe un mensaje desde una data queue. | `RCVDTAQ DTAQ(QUSRSYS/QEZJOBLOG) WAIT(0)` |
| `DSPDTAQ` | Implementado | Muestra contenido de una data queue. | `DSPDTAQ DTAQ(QUSRSYS/QEZJOBLOG)` |
| `DLTDTAQ` | Objetivo V1 | Elimina una data queue. | `DLTDTAQ DTAQ(QUSRSYS/QEZJOBLOG)` |

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

## Variables de entorno utiles

| Variable | Descripcion | Ejemplo |
| --- | --- | --- |
| `L400_ROOT` | Root logico de objetos. | `export L400_ROOT=/l400` |
| `L400_RUN_DIR` | Estado runtime: jobs, loader-status, support-profile. | `export L400_RUN_DIR=/run/l400` |
| `L400_CURLIB` | Biblioteca actual. | `export L400_CURLIB=QGPL` |
| `L400_LIBLIST` | Lista de bibliotecas de busqueda. | `export L400_LIBLIST=QGPL:QSYS` |
| `L400_STORAGE_BACKEND` | Backend para PF/LF/DTAQ. | `export L400_STORAGE_BACKEND=sled` |
| `L400_BPF_PATH` | Ruta al artefacto eBPF. | `export L400_BPF_PATH=/opt/l400/hooks/l400-ebpf` |

## Operacion diaria guiada por rol

Operador:

1. Entrar a `GO MAIN`.
2. Revisar salud con `WRKSYSSTS`, `DSPLOG` y `DSPPOLICY`.
3. Revisar jobs con `WRKACTJOB` y `WRKJOBQ`.
4. Revisar salidas con `WRKSPLF` y `WRKOUTQ`.
5. Antes de mantenimiento: `l400-upgrade-check` y backup con `rsync -aX`.

Administrador:

1. Crear bibliotecas con `CRTLIB`.
2. Gestionar perfiles con `WRKUSRPRF` y, cuando esten disponibles, `CRTUSRPRF`/`CHGUSRPRF`.
3. Asignar autoridad con `GRTOBJAUT` y revisar con `DSPOBJAUT`.
4. Revisar auditoria con `DSPAUD`.
5. Verificar integridad con `CHKOBJINT`.

Programador:

1. Crear o abrir source members con `STRPDM`, `WRKMBRPDM` y `STRSEU`.
2. Crear PF/LF con `CRTPF` y `CRTLF`.
3. Compilar CL con `CRTCLPGM`.
4. Ejecutar con `CALL` o enviar a batch con `SBMJOB`.
5. Consultar datos con `STRSQL` y revisar resultados con `DSPPFM`.
