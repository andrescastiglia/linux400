# l400-ebpf

## Objetivo

`l400-ebpf` contiene el programa eBPF LSM basado en Aya. Su objetivo es reforzar en kernel la frontera de objetos Linux/400, especialmente la ejecucion de `*PGM` y la validacion de tipos y metadata que no deben depender solo de convenciones de userspace.

El programa comparte contrato con `l400-ebpf-common` y es cargado por `l400-loader` en modo `full`, `degraded` o `dev`.

## Nivel de avance

Estado: **medio**.

Ya existen hooks LSM para rutas de apertura/ejecucion, validacion de tipos, formato `*PGM`, politica basica de autoridad y contadores/estado compartidos. El loader puede degradar cuando el host no permite enforcement completo.

Para plena funcionalidad faltan:

- ampliar enforcement por perfil/grupo en mas operaciones de archivo;
- cubrir mas caminos de acceso a objetos, no solo ejecucion;
- auditoria kernel/userspace mas completa y correlacionable;
- pruebas e2e en perfil `full` con kernel, BTF, cgroups y xattrs reales;
- endurecer el contrato ante metadata corrupta, ausente o manipulada fuera del runtime.

## Politica de objetos

La politica kernel protege la frontera de objetos tipados. Los archivos sin `user.l400.objtype` siguen el camino nativo de Linux para no romper `/bin`, `/usr/bin`, instalacion ni herramientas base. La politica se vuelve estricta cuando un archivo declara pertenecer a Linux/400 mediante xattr.

Hooks LSM activos cuando el loader logra adjuntar el programa:

| Hook | Funcion |
| --- | --- |
| `file_open` | Valida que una etiqueta Linux/400 sea conocida. |
| `bprm_creds_from_file` | Toma la decision primaria de ejecucion. |
| `bprm_check_security` | Confirma y consume la decision de ejecucion por PID. |

Matriz base:

| Tipo | `file_open` | `exec` |
| --- | --- | --- |
| sin `user.l400.objtype` | permitido | permitido como binario Linux nativo |
| etiqueta desconocida | denegado | denegado |
| `*PGM` | permitido | permitido solo si pasa reglas de ejecucion |
| `*LIB`, `*FILE`, `*DTAQ`, `*USRPRF`, `*CMD`, `*SRVPGM`, `*OUTQ`, `*JOBQ` | permitido | denegado |

Un `*PGM` puede ejecutarse si:

1. `user.l400.objtype` identifica `*PGM`.
2. `user.l400.objattr` indica salida valida de toolchain (`C` o `CL` en V1).
3. Si `user.l400.auth` contiene `*PUBLIC:*EXCLUDE`, el UID actual es owner (`user.l400.owner_uid`) o tiene entrada `UID:<uid>:*USE`/`UID:<uid>:*ALL`.

`objattr=C|CL` es una marca minima de toolchain, no una firma criptografica. Una fase posterior debe reemplazarla o complementarla con manifest/firma verificable.

## Contadores

El mapa `L400_STATS` expone contadores para aperturas, ejecuciones nativas, ejecuciones `*PGM`, tipos desconocidos, formato invalido, `*PUBLIC:*EXCLUDE`, owner permitido y autoridad explicita por UID. `l400-loader` los imprime cuando corre con enforcement activo.

## Brechas

- aplicar permisos por usuario/owner/grupo tambien en mas caminos de `file_open`;
- correlacionar auditoria kernel con auditoria runtime;
- endurecer firma/manifest de toolchain;
- definir reglas de carga de `*SRVPGM`;
- mantener tests e2e para tipo incorrecto, formato invalido, owner UID y `*PUBLIC:*EXCLUDE`.
