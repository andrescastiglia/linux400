# Linux/400: objetivo general del proyecto

Este documento define la vision de producto de Linux/400. No describe el avance actual del repositorio; esa fotografia vive en `docs/PROJECT.md`. El plan para cerrar la Version 1 vive en `docs/plan/implementation_plan.md`.

## Vision

Linux/400 busca ofrecer una experiencia operativa inspirada en OS/400/IBM i sobre Linux: un sistema orientado a objetos, administrable desde pantalla verde, con comandos consistentes, bibliotecas, perfiles, autorizaciones, trabajos, spool, datos persistentes y herramientas de desarrollo integradas.

No se busca compatibilidad binaria con IBM i ni una emulacion historica completa. El objetivo es recrear el modelo operacional que hacia valioso al AS/400:

- objetos tipados como unidad primaria de administracion;
- bibliotecas y library list como contexto natural de trabajo;
- comandos promptables con ayuda, validacion y mensajes formales;
- pantalla verde como interfaz principal para operadores y administradores;
- jobs interactivos y batch visibles y controlables;
- perfiles, autorizaciones, auditoria y ownership consistentes;
- datos persistentes con PF, LF, DTAQ y SQL operativo;
- instalacion, mantenimiento, respaldo y recuperacion desde flujos guiados;
- degradacion explicita cuando el host Linux no ofrece una capacidad requerida.

La shell Linux debe quedar como herramienta de soporte, desarrollo interno o rescate, no como la interfaz normal de operacion.

## Experiencia objetivo

Una persona debe poder arrancar o instalar Linux/400, autenticarse con un perfil del sistema y operar desde un menu principal sin conocer rutas Unix. Desde ahi debe poder:

- administrar bibliotecas y objetos;
- administrar perfiles de usuario, permisos y auditoria;
- trabajar con jobs, job queues, subsistemas y salida de trabajos;
- administrar spool files y output queues;
- revisar estado del sistema, logs y modo de politica activa;
- crear y editar miembros fuente;
- compilar y ejecutar programas catalogados;
- trabajar con PF, LF, DTAQ y consultas SQL;
- aplicar mantenimiento, PTFs y migraciones de metadata;
- ejecutar backups/restores y validar integridad;
- apagar, reiniciar o dejar el sistema en modo rescue con autoridad adecuada.

## Version 1: operacion basica del sistema

La Version 1 debe entregar una plataforma operable de punta a punta, aunque todavia con un subconjunto controlado de comandos y lenguajes. La prioridad de V1 es administracion y operacion diaria de un sistema tipo AS/400.

Capacidades objetivo de V1:

- instalacion live/install y arranque instalado con persistencia de `/l400`;
- actualizacion por paquetes de mantenimiento o PTFs, con precheck, backup recomendado, apply, rollback y auditoria;
- backups y restores de objetos, bibliotecas y datos preservando xattrs y metadata Linux/400;
- administracion de usuarios, perfiles, estados, ownership y autorizaciones;
- administracion de bibliotecas, objetos, atributos, integridad y catalogo;
- work management basico: jobs interactivos, jobs batch, job queues, subsistemas base, hold/release/end y logs;
- spool basico: output queues, spool files, estados, visualizacion, cambio, borrado y retencion;
- comandos y pantallas para operacion normal desde TUI;
- PF, LF, DTAQ y SQL operativo suficiente para datos administrativos y demos reales;
- compilacion y ejecucion de CL y C como `*PGM`;
- auditoria de cambios sensibles, denegados, ejecuciones y operaciones de mantenimiento;
- soporte para modos `dev`, `degraded` y `full`, con mensajes claros al operador;
- rescue/support report para diagnostico y recuperacion.

El criterio de V1 no es cubrir todo IBM i, sino permitir que un operador instale, mantenga, use, respalde y recupere un Linux/400 basico sin depender de shell para el flujo normal.

## Version 2: desarrollo ampliado

La Version 2 debe ampliar el sistema desde una plataforma operable hacia un entorno de desarrollo mas completo.

Capacidades objetivo de V2:

- programacion no solo en CL y C, sino tambien RPG;
- SQL mas completo como lenguaje de datos y herramienta de desarrollo;
- integracion de RPG/SQL con PF/LF, DTAQ, `*SRVPGM` y programas catalogados;
- source files, PDM/SEU o reemplazos equivalentes mas productivos;
- compiladores con diagnosticos formales, listados, referencias cruzadas y ayudas;
- contratos estables para service programs, binding y llamadas entre lenguajes;
- pruebas de compatibilidad para aplicaciones de negocio no triviales.

## Modelo objetivo de objetos

Linux/400 debe presentar objetos con tipos reconocibles y metadata visible para comandos y pantallas:

| Tipo | Objetivo |
| --- | --- |
| `*LIB` | Biblioteca y contenedor logico de objetos. |
| `*PGM` | Programa ejecutable producido por toolchain Linux/400. |
| `*FILE` | PF, LF y source file. |
| `*DTAQ` | Data queue persistente para comunicacion y logs. |
| `*USRPRF` | Perfil de usuario administrable. |
| `*CMD` | Comando promptable y documentable. |
| `*SRVPGM` | Servicio/codigo compartido para programas. |
| `*OUTQ` | Cola de salida/spool. |
| `*JOBQ` | Cola de trabajos batch. |

Los objetos deben tener tipo, atributo, texto, owner, autorizaciones, auditoria, version de metadata y backend de almacenamiento. El operador debe ver esos datos mediante comandos y pantallas.

## Interfaz objetivo

La TUI debe comportarse como consola primaria:

- sign-on y sign-off reales;
- menu principal y menus de administracion;
- linea de comandos persistente;
- `F4` como prompt por campos con validacion;
- teclas F consistentes;
- opciones numericas por fila;
- confirmaciones para acciones destructivas;
- mensajes CPF o equivalentes Linux/400;
- indicadores claros de modo `dev`, `degraded` o `full`;
- ausencia de datos demo silenciosos cuando falta runtime real.

Pantallas minimas objetivo:

- `WRKOBJ`, `WRKLIB`, `DSPOBJD`;
- `WRKUSRPRF`, `DSPOBJAUT`, `GRTOBJAUT`, `RVKOBJAUT`;
- `WRKACTJOB`, `WRKJOB`, `WRKJOBQ`, subsistemas;
- `WRKSYSSTS`, `WRKSYSVAL`, `DSPLOG`;
- `WRKSPLF`, `WRKOUTQ`, `DSPSPLF`;
- `STRPDM`, `WRKMBRPDM`, `STRSEU`;
- `STRSQL`, visores PF/LF/DTAQ;
- instalacion, PTFs, backup/restore, soporte y rescue;
- politica/auditoria (`DSPPOLICY`, `DSPAUD`).

## Seguridad objetivo

La seguridad debe tener una fuente visible y operable:

- perfiles y owners;
- autorizaciones `*USE`, `*CHANGE`, `*ALL`, `*EXCLUDE`;
- fallback `*PUBLIC`;
- comandos de otorgar, revocar, mostrar y verificar autoridad;
- auditoria de denegados, ejecuciones y cambios sensibles;
- enforcement runtime para comandos sensibles;
- enforcement kernel para frontera de objetos, ejecucion de `*PGM` y acceso donde Linux pueda protegerlo;
- modo degradado explicito cuando el kernel no puede reforzar la politica.

## Plataforma objetivo

Linux/400 debe correr en tres perfiles:

| Perfil | Objetivo |
| --- | --- |
| `dev` | Desarrollo local sin depender de BPF/ZFS/root; todo debe ser testeable en userspace. |
| `degraded` | Sistema instalable y operable sin enforcement kernel completo; la TUI/reportes deben decirlo claramente. |
| `full` | BPF LSM activo, BTF disponible, cgroups v2, `/l400` persistente con xattrs y preferentemente ZFS `xattr=sa`. |

`/l400` es el estado persistente del sistema: bibliotecas, objetos, PF/LF/DTAQ, perfiles, logs y metadata. El backend recomendado es un dataset ZFS con `xattr=sa`; ext4/xfs con xattrs de usuario son fallback validos para desarrollo o modo degradado.

DAX no es requisito de V1 ni condicion de arquitectura. Si se incorpora algun dia, debe ser un perfil avanzado y opt-in para hardware PMEM/NVDIMM/CXL o dispositivos `fsdax`, con contrato propio de layout, flush, recovery, checksums, migracion y reporte en `l400-support-report`. No debe reemplazar el catalogo canonico `/l400` ni bloquear la meta V1.

## No objetivos

- Compatibilidad binaria con IBM i.
- Emulacion completa de 5250.
- Reimplementar TIMI, EBCDIC o todos los comandos historicos.
- Requerir un fork permanente del kernel.
- Ocultar o bloquear herramientas Linux nativas fuera de la frontera Linux/400.
- Requerir DAX, PMEM, CXL o storage byte-addressable para operar la Version 1.

## Definicion de sistema logrado

Linux/400 cumple su objetivo inicial cuando una persona puede instalarlo, entrar al menu principal y completar un ciclo operativo sin shell:

1. crear biblioteca y objetos;
2. crear/editar miembros fuente;
3. compilar CL/C a `*PGM`;
4. ejecutar programas interactivos y batch;
5. administrar usuarios y autorizaciones;
6. trabajar con PF/LF/DTAQ y SQL;
7. revisar jobs, logs, auditoria, spool y estado de sistema;
8. aplicar mantenimiento/PTF con precheck y rollback;
9. hacer backup/restore y verificar integridad;
10. reiniciar y conservar estado persistente.
