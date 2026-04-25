# Politica de objetos Linux/400

Version de politica activa: `phase3-v1`

Este documento describe el contrato real entre `libl400`, `l400-ebpf-common`, `l400-ebpf` y `l400-loader`. La politica actual no intenta implementar todo el modelo de autorizaciones de IBM i. Su objetivo es proteger la frontera basica de objetos tipados y evitar ejecuciones de objetos que no sean `*PGM`.

## Fuente de verdad

El tipo de objeto se guarda en:

```text
user.l400.objtype
```

Los tipos validos se definen en `l400-ebpf-common/src/lib.rs` y son compartidos por userspace y eBPF:

| Tipo | Prefijo eBPF | Uso actual |
| --- | --- | --- |
| `*PGM` | `*PGM` | Programa ELF catalogado, ejecutable si tiene atributo de toolchain valido. |
| `*FILE` | `*FIL` | PF, LF o source file. No ejecutable. |
| `*USRPRF` | `*USR` | Perfil de usuario Linux/400. No ejecutable. |
| `*LIB` | `*LIB` | Biblioteca/directorio catalogado. No ejecutable. |
| `*DTAQ` | `*DTA` | Data queue. No ejecutable. |
| `*CMD` | `*CMD` | Objeto comando futuro. No ejecutable en esta fase. |
| `*SRVPGM` | `*SRV` | Service program futuro. No ejecutable en esta fase. |
| `*OUTQ` | `*OUT` | Output queue futuro. No ejecutable. |

Agregar un tipo nuevo exige actualizar `l400-ebpf-common`, `libl400` y la matriz de politica eBPF.

## Metadatos de objeto

`libl400` usa estos xattrs principales:

| Atributo | Significado |
| --- | --- |
| `user.l400.objtype` | Tipo autoritativo (`*PGM`, `*FILE`, etc.). |
| `user.l400.objattr` | Atributo de objeto (`C`, `CL`, `PF`, `LF`, `SRC`, `DTAQ`, etc.). |
| `user.l400.text` | Texto descriptivo. |
| `user.l400.owner` | Propietario logico inicial. |
| `user.l400.owner_uid` | UID Linux efectivo que catalogo el objeto; usado por eBPF para excepcion de owner. |
| `user.l400.auth` | Autorizaciones runtime (`USER:*USE`, `*PUBLIC:*EXCLUDE`, etc.). |
| `user.l400.storage_backend` | Backend de PF/LF/DTAQ (`sled` o `berkeleydb`). |
| `user.l400.record_len` | Longitud de registro PF. |
| `user.l400.base_pf` | PF base de un LF. |

La politica eBPF actual lee `objtype`, `objattr`, `owner_uid` y una parte de `auth`. El runtime usa el resto para catalogo, storage y pantallas.

## Hooks eBPF activos

`l400-ebpf` instala tres hooks LSM cuando el loader esta activo:

| Hook | Funcion |
| --- | --- |
| `file_open` | Valida que una etiqueta Linux/400 sea conocida. |
| `bprm_creds_from_file` | Toma la decision primaria de ejecucion. |
| `bprm_check_security` | Confirma y consume la decision de ejecucion por PID. |

Si el loader no puede cargar estos hooks, el sistema puede continuar en modo `degraded` o `dev`, pero sin enforcement kernel activo.

## Matriz base

| Tipo | `file_open` | `exec` |
| --- | --- | --- |
| sin `user.l400.objtype` | permitido | permitido como binario Linux nativo |
| etiqueta desconocida | denegado | denegado |
| `*LIB` | permitido | denegado |
| `*PGM` | permitido | permitido solo si pasa reglas de ejecucion |
| `*FILE` | permitido | denegado |
| `*DTAQ` | permitido | denegado |
| `*USRPRF` | permitido | denegado |
| `*CMD` | permitido | denegado |
| `*SRVPGM` | permitido | denegado en esta fase |
| `*OUTQ` | permitido | denegado |

## Reglas de ejecucion `*PGM`

Un objeto `*PGM` puede ejecutarse si:

1. `user.l400.objtype` tiene prefijo `*PGM`.
2. `user.l400.objattr` indica salida valida de toolchain:
   - `C`
   - `CL`
3. Si `user.l400.auth` contiene `*PUBLIC:*EXCLUDE` o `*PUBLIC:EXCLUDE`, el UID actual debe coincidir con `user.l400.owner_uid` o tener una entrada `UID:<uid>:*USE`/`UID:<uid>:*ALL`.

Si el objeto es `*PGM` pero no tiene atributo `C` o `CL`, se deniega como formato invalido. Esto es una marca minima de toolchain, no una firma criptografica.

## Reglas para binarios nativos

Los archivos sin `user.l400.objtype` siguen el camino nativo de Linux. Esto es deliberado:

- permite que el sistema base arranque;
- no rompe `/bin`, `/usr/bin`, herramientas de instalacion ni runtime;
- separa la personalidad Linux/400 del sistema Linux subyacente.

La politica solo se vuelve estricta cuando un archivo declara pertenecer al catalogo Linux/400 mediante xattr.

## Autorizaciones runtime

`libl400/src/auth.rs` soporta:

| Autoridad | Nivel |
| --- | --- |
| `*EXCLUDE` | Deniega. |
| `*USE` | Uso/lectura basica. |
| `*CHANGE` | Cambio. |
| `*ALL` | Control completo. |

Formato de `user.l400.auth`:

```text
USER:*USE,*PUBLIC:*EXCLUDE
```

Reglas runtime:

- el permiso explicito del usuario gana;
- `*EXCLUDE` deniega;
- `*PUBLIC` actua como fallback;
- el owner puede tener `*ALL` implicito;
- sin owner ni permiso aplicable, se deniega.

La ruta eBPF aplica una identidad minima en ejecucion: owner UID y entradas `UID:<uid>:*USE/*ALL` pueden superar el fallback `*PUBLIC:*EXCLUDE`. La matriz completa por perfil/grupo sigue viviendo en runtime para operaciones no ejecutables.

## Loader y modos de enforcement

`l400-loader` publica estado en:

```text
${L400_RUN_DIR:-/run/l400}/loader-status
```

Campos relevantes:

| Campo | Significado |
| --- | --- |
| `mode` | `full`, `degraded` o `dev`. |
| `protection_active` | `1` si los hooks estan activos. |
| `phase` | `starting`, `active`, `fallback`, `stopped`, etc. |
| `attached_hooks` | Hooks LSM adjuntados. |
| `policy_version` | Debe coincidir con `phase3-v1`. |
| `last_error` | Error de carga o adjunte si aplica. |

Modos:

- `full`: requiere enforcement activo o falla.
- `degraded`: intenta activar enforcement; si falla, continua sin proteccion kernel.
- `dev`: tolera entorno incompleto para desarrollo.

## Estadisticas

El mapa eBPF `L400_STATS` expone contadores:

| Constante | Significado |
| --- | --- |
| `STAT_OPEN_ALLOWED` | Aperturas permitidas de objetos Linux/400. |
| `STAT_DENIED_INVALID_TAG` | Accesos/exec denegados por etiqueta desconocida. |
| `STAT_EXEC_ALLOWED_NATIVE` | Ejecuciones nativas sin etiqueta permitidas. |
| `STAT_EXEC_ALLOWED_PGM` | Ejecuciones `*PGM` permitidas. |
| `STAT_EXEC_DENIED_WRONG_TYPE` | Exec denegado porque no era `*PGM`. |
| `STAT_EXEC_DECISION_MISSING` | `bprm_check_security` no encontro decision previa. |
| `STAT_EXEC_CHECK_ALLOWED` | Confirmaciones de exec permitido. |
| `STAT_EXEC_CHECK_DENIED` | Confirmaciones de exec denegado. |
| `STAT_EXEC_DENIED_INVALID_FORMAT` | `*PGM` sin atributo `C` o `CL`. |
| `STAT_EXEC_DENIED_EXCLUDE` | `*PGM` denegado por `*PUBLIC:*EXCLUDE`. |
| `STAT_EXEC_ALLOWED_OWNER` | `*PGM` permitido porque el UID actual es owner. |
| `STAT_EXEC_ALLOWED_USER_AUTH` | `*PGM` permitido por autoridad explicita `UID:<uid>`. |
| `STAT_OBJTYPE_BASE + n` | Conteo por tipo valido. |

`l400-loader` imprime estos contadores cuando corre en modo activo.

## Comandos y pantallas relacionadas

Estado actual:

- `WRKOBJ`: muestra catalogo de objetos y desde TUI permite `DSPPFM`, `DSPOBJD`, `DSPDTAQ` y `DLTOBJ` con confirmacion.
- `CRTLIB`, `DLTLIB`, `RNMOBJ`, `CRTPGM`, `DLTOBJ`, `CPYOBJ`, `CHGOBJD`: gestion de objetos.
- `DSPOBJAUT`, `CHKOBJAUT`, `GRTOBJAUT`, `RVKOBJAUT`, `DSPPOLICY`, `DSPAUD`: administracion de politica y auditoria.
- `STRPDM`, `WRKMBRPDM`, `STRSEU`, `STRSQL`: flujo de desarrollo.
- `WRKACTJOB`, `WRKSYSSTS`: estado del sistema y trabajos, con detalle y terminacion confirmada desde TUI.
- `l400-support-report`: clasifica plataforma, loader, BPF, cgroups y xattrs/ZFS.

La fase 9 agrega una matriz runtime minima:

| Comando | Operacion | Autoridad requerida |
| --- | --- | --- |
| `CALL` | `EXECUTE` | `*USE` |
| `DSPOBJD`, `DSPOBJAUT`, `WRKOBJ`, `WRKLIB` | `READ` | `*USE` |
| `DSPPFM`, `DSPDTAQ` | `READ` | `*USE` |
| `WRTPFM`, `SNDDTAQ`, `RCVDTAQ`, `CPYOBJ` | `CHANGE` | `*CHANGE` |
| `GRTOBJAUT`, `RVKOBJAUT`, `CHGOBJD`, `DLTOBJ`, `CLRPFM` | `ADMIN` | `*ALL` |

`CALL` verifica esta matriz antes de ejecutar un `*PGM`; por lo tanto `*PUBLIC:*EXCLUDE` bloquea tanto el runtime como la politica eBPF basica.

## Auditoria

Los eventos sensibles se escriben en `QSYS/QHST` y, si existe, tambien en `QUSRSYS/QEZJOBLOG *DTAQ`.

Eventos actuales:

- `ACCESS_DENIED`: denegados de `CALL` y `CHKOBJAUT`;
- `PGM_EXEC`: ejecucion de `*PGM`;
- `AUTH_CHANGE`: `GRTOBJAUT` y `RVKOBJAUT`;
- `USRPRF_CHANGE`: creacion/desactivacion de perfiles.

`DSPAUD` muestra los ultimos eventos y `DSPPOLICY` documenta la matriz efectiva.

## Diagnostico rapido

```bash
cat "${L400_RUN_DIR:-/run/l400}/loader-status"
l400-support-report --write
getfattr -n user.l400.objtype /l400/QSYS/OBJ 2>/dev/null
getfattr -n user.l400.objattr /l400/QSYS/OBJ 2>/dev/null
getfattr -n user.l400.auth /l400/QSYS/OBJ 2>/dev/null
```

Para probar rutas de loader sin requerir BPF disponible:

```bash
cargo run -p l400-loader -- --mode dev --once
cargo run -p l400-loader -- --mode degraded --once
```

## Brechas de politica

1. Aplicar permisos por usuario/owner/grupo tambien en `file_open`.
2. Reemplazar `objattr=C|CL` por firma o manifest de toolchain mas robusto.
3. Definir si `*SRVPGM` sera cargable como dependencia de `*PGM` y bajo que reglas.
4. Mantener tests e2e para `*PUBLIC:*EXCLUDE`, tipo incorrecto, owner UID y formato invalido.

## Criterio de avance

La politica se considera lista para la siguiente etapa cuando:

- `full` falla si no hay enforcement y `degraded/dev` informan claramente su estado;
- todos los `*PGM` generados por `clc` y `c400c` ejecutan con atributos correctos;
- `*FILE`, `*DTAQ`, `*LIB`, `*USRPRF`, `*CMD`, `*SRVPGM` y `*OUTQ` no ejecutan;
- `*PUBLIC:*EXCLUDE` impide ejecucion de `*PGM`;
- la TUI y `l400-support-report` muestran estado y errores de politica sin requerir shell.
