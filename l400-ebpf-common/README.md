# l400-ebpf-common

## Objetivo

`l400-ebpf-common` define el contrato `no_std` compartido entre userspace y el programa eBPF. Centraliza constantes de version, tipos de objetos validos y estructuras que deben mantenerse estables entre `libl400`, `l400-loader` y `l400-ebpf`.

Su objetivo es evitar divergencias entre lo que el runtime cataloga y lo que el kernel permite.

## Nivel de avance

Estado: **alto para el alcance actual**.

El crate ya concentra los tipos Linux/400 validos y los identificadores usados por el LSM. Es pequeno, portable y adecuado para `no_std`.

Para plena funcionalidad faltan:

- versionado formal de ABI/politica para upgrades y PTFs;
- pruebas de compatibilidad hacia atras cuando cambien objetos o estructuras;
- documentar reglas para agregar nuevos tipos de objeto;
- expandir el contrato si el enforcement kernel cubre mas autorizaciones o eventos.

## Contrato de objetos

El tipo autoritativo de un objeto Linux/400 vive en:

```text
user.l400.objtype
```

Los tipos validos se definen en `src/lib.rs` y deben mantenerse sincronizados con `libl400`, `l400-ebpf` y `l400-loader`.

| Tipo | Prefijo eBPF | Uso |
| --- | --- | --- |
| `*PGM` | `*PGM` | Programa ELF catalogado. Ejecutable solo si pasa reglas de formato y autoridad. |
| `*FILE` | `*FIL` | PF, LF o source file. No ejecutable. |
| `*USRPRF` | `*USR` | Perfil de usuario Linux/400. No ejecutable. |
| `*LIB` | `*LIB` | Biblioteca/directorio catalogado. No ejecutable. |
| `*DTAQ` | `*DTA` | Data queue. No ejecutable. |
| `*CMD` | `*CMD` | Comando promptable/documentable. No ejecutable. |
| `*SRVPGM` | `*SRV` | Service program futuro. No ejecutable como `CALL` en V1. |
| `*OUTQ` | `*OUT` | Output queue/spool. No ejecutable. |
| `*JOBQ` | `*JOB` | Cola de trabajos batch. No ejecutable. |

Agregar un tipo nuevo requiere:

- agregarlo a `VALID_OBJ_TYPES`;
- actualizar validacion/catalogacion en `libl400`;
- actualizar reglas eBPF si el tipo debe tener tratamiento especial;
- agregar pruebas de compatibilidad y documentar su autoridad minima.
