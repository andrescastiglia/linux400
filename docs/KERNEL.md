# Linux/400: objetivo del sistema

Este documento define el objetivo de producto de Linux/400. No describe el estado actual del repositorio ni una lista de modulos implementados; esa fotografia vive en `docs/PROJECT.md`. La brecha entre esta vision y el estado actual vive en `docs/plan/implementation_plan.md`.

## Vision

Linux/400 busca ofrecer una forma de trabajo tipo OS/400/IBM i sobre Linux: el usuario entra a una pantalla de sign-on, opera desde menus y comandos de pantalla verde, administra objetos y trabajos, desarrolla programas y conserva el estado del sistema sin depender de una shell Unix para las tareas normales.

No se busca compatibilidad binaria con IBM i ni reproducir cada detalle historico. El objetivo es recrear el modelo operativo:

- sistema orientado a objetos, no a rutas de archivos visibles para el operador;
- bibliotecas y library list como contexto natural de trabajo;
- comandos consistentes, promptables y administrables;
- pantalla verde como interfaz primaria;
- jobs interactivos y batch visibles y controlables;
- perfiles, autorizaciones, auditoria y politica explicita;
- almacenamiento persistente y recuperable;
- degradacion clara cuando una capacidad de plataforma no esta disponible.

## Experiencia objetivo

Un operador debe poder arrancar una ISO o una instalacion, autenticarse con un perfil Linux/400 y llegar directamente a un menu principal. Desde ahi debe poder:

- navegar y administrar bibliotecas;
- crear, copiar, renombrar, describir y borrar objetos;
- administrar perfiles y autorizaciones;
- trabajar con jobs interactivos y batch;
- revisar estado de sistema, logs, auditoria y politica activa;
- editar miembros fuente;
- compilar CL/C y ejecutar programas catalogados;
- trabajar con PF, LF y DTAQ;
- usar SQL operativo contra archivos Linux/400;
- apagar o reiniciar el sistema con confirmacion y autoridad adecuada;
- cerrar sesion sin dejar estado interactivo colgado.

La shell Linux debe quedar como herramienta de soporte, instalacion, desarrollo interno o rescue, no como interfaz principal del sistema.

## Modelo objetivo de objetos

El sistema debe presentar una frontera de objetos Linux/400 con tipos reconocibles:

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

Los objetos deben tener metadatos de tipo, atributo, texto, owner, autorizaciones, auditoria y backend de almacenamiento. El operador debe ver esos metadatos mediante comandos y pantallas, no inspeccionando xattrs manualmente.

## Interfaz objetivo

La TUI debe comportarse como la consola primaria del sistema:

- sign-on y sign-off reales;
- menu principal y menus de trabajo;
- linea de comandos persistente en la sesion;
- `F4` como prompt por campos con validacion;
- teclas F consistentes;
- opciones numericas por fila;
- confirmaciones visuales para acciones destructivas;
- mensajes de estado claros;
- ausencia de datos demo silenciosos cuando falta runtime real.

Las pantallas minimas objetivo son:

- `WRKOBJ` / `WRKLIB`;
- `DSPOBJD`;
- `WRKUSRPRF`;
- `WRKACTJOB`;
- `WRKSYSSTS`;
- `WRKSYSVAL`;
- `WRKSPLF` / `WRKOUTQ`;
- `STRPDM`;
- `WRKMBRPDM`;
- `STRSEU`;
- `STRSQL`;
- visores PF/LF/DTAQ;
- politica/auditoria (`DSPPOLICY`, `DSPAUD`, autorizaciones).

## Work management objetivo

Linux/400 debe exponer trabajos como unidad operativa:

- jobs interactivos (`QINTER`);
- jobs batch (`QBATCH`);
- estado `JOBQ`, `ACTIVE`, `COMPLETED`, `FAILED` y terminado;
- comando ejecutado, usuario, timestamps, salida/log y subsistema;
- envio batch por comando;
- terminacion controlada;
- degradacion visible cuando cgroups o aislamiento no estan disponibles.

La implementacion puede usar procesos Linux, cgroups y archivos de runtime, pero el operador debe ver trabajos Linux/400.

## Datos objetivo

PF/LF/DTAQ deben ser suficientes para operar demos y flujos administrativos reales:

- PF con record length, miembros, campos, claves, RRN y arrival sequence;
- LF como indice mantenido automaticamente sobre PF;
- comandos para crear, limpiar, agregar miembros, escribir y visualizar;
- DTAQ con mensajes de longitud variable, espera y lectura FIFO;
- SQL sobre PF con consultas y DML basico;
- persistencia entre reinicios en instalacion real.

## Toolchain objetivo

El entorno debe permitir desarrollar sin salir de Linux/400:

- source files y miembros;
- edicion desde TUI;
- compilacion CL y C;
- catalogacion como `*PGM`;
- resolucion por current library y library list;
- errores formales estilo CPF para que `MONMSG` y auditoria tengan semantica util;
- marca o firma de toolchain verificable antes de ejecutar.

## Seguridad objetivo

La politica de seguridad debe tener una fuente visible y operable:

- perfiles y owners;
- autorizaciones `*USE`, `*CHANGE`, `*ALL`, `*EXCLUDE`;
- fallback `*PUBLIC`;
- comandos de otorgar, revocar, mostrar y verificar autoridad;
- auditoria de denegados, ejecuciones y cambios sensibles;
- enforcement runtime para todos los comandos sensibles;
- enforcement kernel para la frontera de objetos y ejecucion de `*PGM`;
- modo degradado explicito cuando el kernel no puede reforzar la politica.

La meta no es meter IBM i dentro del kernel, sino usar el kernel para reforzar aquello que Linux puede proteger mejor: ejecucion, acceso a objetos tipados, aislamiento de procesos y observabilidad.

## Plataforma objetivo

Linux/400 debe correr en tres perfiles:

| Perfil | Objetivo |
| --- | --- |
| `dev` | Desarrollo local sin depender de BPF/ZFS/root; todo debe ser testeable en user space. |
| `degraded` | Sistema instalable y operable sin enforcement kernel completo; la TUI/reportes deben decirlo claramente. |
| `full` | BPF LSM activo, BTF disponible, cgroups v2, `/l400` persistente con xattrs y preferentemente ZFS `xattr=sa`. |

La plataforma completa debe poder instalarse, reiniciar y conservar `/l400`. El gate de release debe probar instalacion, arranque instalado y persistencia.

## No objetivos

- Compatibilidad binaria con IBM i.
- Emulacion completa de 5250.
- Reimplementar TIMI, EBCDIC o todos los comandos historicos.
- Requerir un fork permanente del kernel.
- Bloquear herramientas Linux nativas fuera de la frontera Linux/400.

## Definicion de sistema logrado

El objetivo se considera alcanzado cuando una persona puede instalar o arrancar Linux/400, entrar al menu principal y completar un ciclo operativo completo sin shell:

1. crear biblioteca y source file;
2. crear/editar miembro CL;
3. compilarlo a `*PGM`;
4. ejecutar el programa;
5. enviar un job batch;
6. revisar jobs/logs/auditoria;
7. crear PF/LF/DTAQ y operar datos;
8. administrar autorizaciones;
9. reiniciar y verificar persistencia de `/l400`.
