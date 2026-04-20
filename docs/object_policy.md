# Política de Objetos Linux/400

Versión de política activa: `phase3-v1`

## Matriz base

| Tipo | `file_open` | `exec` (`bprm_creds_from_file` + `bprm_check_security`) |
| --- | --- | --- |
| `*LIB` | permitido si el tag es válido | denegado |
| `*PGM` | permitido si el tag es válido | permitido |
| `*FILE` | permitido si el tag es válido | denegado |
| `*DTAQ` | permitido si el tag es válido | denegado |
| `*USRPRF` | permitido si el tag es válido | denegado |
| `*CMD` | permitido si el tag es válido | denegado |
| `*SRVPGM` | permitido si el tag es válido | denegado en esta fase |
| `*OUTQ` | permitido si el tag es válido | denegado |

## Reglas operativas

- Si un archivo no tiene `user.l400.objtype`, el acceso y la ejecución siguen por el camino nativo Linux.
- Si el archivo tiene `user.l400.objtype` con prefijo desconocido, el LSM deniega acceso y ejecución.
- La ejecución de objetos Linux/400 sólo está soportada para `*PGM` en esta fase.
- `bprm_creds_from_file` toma la decisión primaria de ejecución sobre el `file*` real.
- `bprm_check_security` consume esa decisión y deja trazabilidad del enforcement aplicado.

## Diagnóstico

El mapa `L400_STATS` expone contadores para:

- aperturas permitidas
- etiquetas inválidas denegadas
- ejecución nativa permitida
- ejecución de `*PGM` permitida
- ejecución denegada por tipo incorrecto
- ejecuciones sin decisión previa
- confirmaciones y denegaciones en `bprm_check_security`

El `loader-status` persistido por `l400-loader` publica además:

- `attached_hooks`
- `policy_version`
- `last_error`
