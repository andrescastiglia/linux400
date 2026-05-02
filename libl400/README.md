# libl400

## Objetivo

`libl400` es el runtime central de Linux/400. Implementa objetos, bibliotecas, metadata xattr, PF/LF, DTAQ, source members, autorizaciones, auditoria, work management, spool basico, comandos runtime, FFI y utilidades de plataforma.

Su objetivo es ser la autoridad de userspace para todo lo que el operador ve como objetos y comandos Linux/400.

## Nivel de avance

Estado: **medio-alto**.

Ya existen objetos `*LIB`, `*PGM`, `*FILE`, `*DTAQ`, `*USRPRF`, `*CMD`, `*OUTQ` y reconocimiento de `*SRVPGM`; PF/LF/DTAQ usan backend `sled`; hay comandos `l400cmd`, jobs batch/interactivos, auditoria, autorizaciones, bootstrap y pruebas de demos V1.

Para plena funcionalidad faltan:

- completar administracion de usuarios con alta/baja/cambio, password y politicas desde comandos/TUI;
- hacer spool y output queues mas cercanos a AS/400, con retencion, estados, writers e impresoras/export;
- formalizar JOBQ/OUTQ/subsistemas como objetos operables completos;
- implementar contrato real de `*SRVPGM`;
- robustecer backup/restore, integridad, migraciones y PTFs como comandos de operacion;
- ampliar SQL, PF/LF y diagnosticos CPF para uso productivo.

## Metadata de objetos

`libl400` usa xattrs `user.l400.*` como contrato de catalogo. Los atributos principales son:

| Atributo | Significado |
| --- | --- |
| `user.l400.objtype` | Tipo autoritativo (`*PGM`, `*FILE`, etc.). |
| `user.l400.objattr` | Atributo de objeto (`C`, `CL`, `PF`, `LF`, `SRC`, `DTAQ`, etc.). |
| `user.l400.text` | Texto descriptivo. |
| `user.l400.owner` | Propietario logico. |
| `user.l400.owner_uid` | UID Linux asociado al owner/catalogador. |
| `user.l400.auth` | Autorizaciones planas runtime, por ejemplo `USER:*USE,*PUBLIC:*EXCLUDE`. |
| `user.l400.auth.manifest` | Manifest estructurado de autorizaciones. |
| `user.l400.storage_backend` | Backend de PF/LF/DTAQ (`sled` o `berkeleydb`). |
| `user.l400.record_len` | Longitud de registro PF. |
| `user.l400.base_pf` | PF base de un LF. |
| `user.l400.data.version` | Version de metadata/datos por objeto cuando aplica. |

## Autoridades runtime

Autoridades actuales:

| Autoridad | Uso |
| --- | --- |
| `*EXCLUDE` | Deniega. |
| `*USE` | Uso/lectura basica. |
| `*CHANGE` | Cambio. |
| `*ALL` | Control completo. |

Reglas base:

- permiso explicito del perfil gana;
- `*EXCLUDE` deniega;
- `*PUBLIC` actua como fallback;
- el owner puede tener `*ALL` implicito;
- sin owner ni permiso aplicable, se deniega.

Cuando `GRTOBJAUT` puede resolver un perfil Linux/400 a `*USRPRF`, tambien escribe una entrada espejo `UID:<uid>:*AUTH` para que eBPF tenga una clave estable en ejecucion.

## Contrato `*SRVPGM`

`*SRVPGM` es un tipo reconocido y catalogable, pero en V1 sigue siendo backlog de toolchain. Puede copiarse, auditarse y protegerse por autoridades, pero no es target de `CALL`.

Metadata requerida para catalogar un service program:

- `user.l400.objtype=*SRVPGM`
- `user.l400.objattr=SRVPGM`
- `user.l400.text`
- `user.l400.owner`
- `user.l400.auth`
- `user.l400.auth.manifest`

Metadata futura reservada:

- `user.l400.srvpgm.exports`
- `user.l400.srvpgm.imports`
- `user.l400.srvpgm.signature`

Crear o reemplazar un `*SRVPGM` requiere `*CHANGE`. Cargarlo o enlazarlo requerira `*USE` cuando exista el linker/runtime loader. Hasta entonces, `CALL` requiere un `*PGM` con metadata valida de toolchain.
