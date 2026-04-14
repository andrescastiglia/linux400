# Plan de Implementación Pendiente de Linux/400

Este documento reemplaza el plan actual y toma como base únicamente lo que quedó **sin implementar** en la nota de ejecución anterior. El objetivo ya no es describir todo Linux/400, sino ordenar el trabajo pendiente de la forma más conveniente según la arquitectura objetivo de [PROJECT.md](/home/user/Source/linux400/docs/PROJECT.md:1) y las restricciones técnicas de [KERNEL.md](/home/user/Source/linux400/docs/KERNEL.md:1).

## Alcance de este plan

Este plan cubre sólo lo pendiente:

- cierre del baseline soportado y validación E2E real
- migración de storage desde `sled` hacia ZFS + Berkeley DB
- enforcement real del object model en modo `full`
- modelo de memoria con LAM/TBI, `mmap` determinista y evaluación de DAX
- toolchain Linux/400 no basado en `stub`
- subsistemas y jobs reales más allá de demos
- perfiles, autorizaciones y administración real del sistema

No cubre lo ya implementado y validado recientemente:

- persistencia de `loader-status`
- exposición del estado del loader al userspace
- visualización del estado del loader en la TUI

## Análisis de conveniencia

### 1. Qué exige `PROJECT.md`

`PROJECT.md` hace cuatro apuestas arquitectónicas fuertes:

1. ZFS como frontera real del sistema de objetos.
2. Berkeley DB como backend integrado para `*FILE`, PF/LF y `*DTAQ`.
3. Enforcement del object model mediante metadata + runtime + kernel hooks.
4. Toolchain y ejecución de `*PGM` como artefactos nativos Linux/400.

Eso significa que `sled`, los `stub` del compilador CL y el enforcement parcial actual sólo pueden seguir existiendo como mecanismos de transición.

### 2. Qué restringe `KERNEL.md`

`KERNEL.md` no describe sólo optimizaciones; describe dependencias reales del diseño:

- `BPF LSM` es requisito para tipificación fuerte en kernel.
- `LAM`/`TBI` son requisito para el modelo de punteros etiquetados.
- `cgroups v2` es requisito para `QINTER`/`QBATCH`.
- `DAX` es deseable para aproximarse a SLS, pero depende fuertemente de la plataforma.
- `sched_ext` aparece como capacidad interesante, pero no es requisito mínimo para cerrar el sistema.

La consecuencia práctica es que el plan no debe avanzar como si todas esas capacidades estuvieran disponibles siempre. Hace falta separar:

- lo que es arquitectura objetivo
- lo que es baseline soportado
- lo que queda como aceleración o perfil avanzado

### 3. Decisiones de implementación

A partir de ese cruce entre `PROJECT.md` y `KERNEL.md`, este plan toma estas decisiones:

1. ZFS + xattrs dejan de ser “metadata opcional” y pasan a ser la representación primaria del sistema de objetos.
2. Berkeley DB pasa a ser el backend objetivo de `*FILE` y `*DTAQ`; `sled` queda sólo como backend transitorio de test/desarrollo.
3. El enforcement del kernel se implementa antes de ampliar CL o sumar más UX, porque `PROJECT.md` supone objetos seguros, no sólo objetos catalogados.
4. `sched_ext` se difiere: primero se cierra `cgroups v2` y el modelo de jobs; luego se evalúa scheduler BPF.
5. `DAX` no se toma como prerequisito de la primera convergencia arquitectónica. Primero se diseña y cierra el ABI de punteros etiquetados y el mapeo determinista; después se agrega una ruta acelerada con DAX para plataformas soportadas.
6. `clc` deja de crecer por cantidad de comandos hasta que exista formato de `*PGM` verificable y runtime real de ejecución.
7. `*USRPRF` y autorizaciones se implementan después de storage + object manager + enforcement, porque dependen de esas capas para no quedar en simples archivos decorados.

## Orden recomendado

El orden más conveniente para completar lo pendiente es:

1. Baseline soportado y validación E2E.
2. Storage objetivo y representación real de objetos.
3. Enforcement real del object manager.
4. Formato de `*PGM` y toolchain sin `stub`.
5. Subsistemas y jobs reales.
6. Perfiles y autorizaciones.
7. Memoria etiquetada avanzada y DAX como cierre de convergencia, con parte de diseño comenzando antes.

La razón de este orden es simple:

- sin baseline soportado no hay manera seria de validar kernel/BPF/ZFS
- sin storage real no tiene sentido cerrar enforcement ni toolchain
- sin enforcement no tiene sentido llamar “Linux/400” al runtime de ejecución
- sin `*PGM` real no conviene expandir CL
- sin jobs reales no conviene sofisticar la TUI
- sin object manager completo no conviene cerrar autorizaciones

## Fase 1. Baseline Soportado y Validación E2E

### Objetivo

Cerrar el perfil mínimo soportado donde Linux/400 pueda ser validado de punta a punta.

### Decisiones

- QEMU UEFI pasa a ser la plataforma de validación obligatoria.
- El modo `full` sólo se declara soportado cuando BPF LSM, BTF, ZFS y cgroups v2 estén realmente operativos en esa plataforma.
- Los modos `degraded` y `dev` siguen existiendo, pero documentados como transición y no como objetivo final.

### Trabajo

- revalidar `scripts/test/test_e2e_install_qemu.sh`
- verificar live, instalación, reboot y boot desde disco
- documentar el perfil de kernel mínimo realmente usado por Linux/400
- documentar qué falla exactamente cuando el sistema cae a `degraded`
- crear una matriz corta de soporte basada en features reales:
  - `BPF LSM`
  - `BTF`
  - `ZFS xattr=sa`
  - `cgroups v2`
  - `LAM` o `TBI`

### Entregables

- criterio formal para `full`, `degraded` y `dev`
- validación E2E reproducible en QEMU UEFI
- documentación de plataforma soportada

### Criterio de aceptación

- instalación y boot instalado pasan en QEMU
- el sistema puede identificar con precisión cuándo está en `full`

## Fase 2. Storage Objetivo: ZFS + Berkeley DB

### Objetivo

Reemplazar la arquitectura transitoria actual por la arquitectura de datos definida en `PROJECT.md`.

### Decisiones

- `sled` queda encapsulado detrás de una abstracción de storage y deja de ser el camino principal.
- ZFS pasa a representar bibliotecas y objetos como estructura primaria del sistema.
- Berkeley DB se usa como backend para PF/LF y `*DTAQ`.
- Los miembros de PF y los índices de LF se modelan con las primitivas nativas de BDB, no con convenciones locales sobre `sled`.

### Trabajo

- introducir una capa `storage` en `libl400`
- mover PF/LF/DTAQ a una interfaz backend-agnostic
- implementar backend `sled` transitorio
- implementar backend Berkeley DB objetivo
- mapear:
  - PF a primary database
  - LF a secondary database
  - members a subdatabases
  - `*DTAQ` a queue o estructura equivalente compatible
- convertir las bibliotecas `*LIB` en representación soportada sobre ZFS
- documentar layout físico y xattrs por tipo

### Entregables

- API de storage desacoplada
- backend Berkeley DB funcional
- reglas de layout ZFS documentadas

### Criterio de aceptación

- `*FILE` PF/LF y `*DTAQ` funcionan en backend Berkeley DB
- `sled` ya no es obligatorio para el runtime soportado

## Fase 3. Object Manager y Enforcement Real

### Objetivo

Pasar del etiquetado básico a enforcement verificable de objetos Linux/400.

### Decisiones

- `file_open` no alcanza; el enforcement soportado debe cubrir al menos acceso y ejecución.
- `bprm_check_security` deja de ser stub y pasa a validar `*PGM`.
- la política se define desde el modelo de objetos, no desde casos sueltos del loader.

### Trabajo

- definir matriz de políticas por tipo:
  - `*LIB`
  - `*PGM`
  - `*FILE`
  - `*DTAQ`
  - `*USRPRF`
  - `*CMD`
- completar hooks eBPF/LSM necesarios
- conectar loader, runtime y object manager para que el estado `full` sea trazable
- validar casos permitidos y denegados reales
- registrar causas de denegación útiles para diagnóstico
- documentar la política efectiva en [object_policy.md](/home/user/Source/linux400/docs/object_policy.md:1)

### Entregables

- política de objetos documentada
- loader `full` verificable
- pruebas E2E de enforcement

### Criterio de aceptación

- existe al menos un flujo real donde el kernel permita y deniegue según tipo de objeto
- `*PGM` sólo ejecuta cuando cumple la política definida

## Fase 4. Formato de `*PGM` y Toolchain Real

### Objetivo

Cerrar la ejecución de programas Linux/400 como artefactos soportados y no como ELF genéricos catalogados.

### Decisiones

- antes de crecer en CL, se define un formato lógico de `*PGM`.
- `clc` deja de depender del `stub` como camino soportado.
- `c400c` y `clc` convergen en el mismo contrato de `*PGM`.

### Trabajo

- definir metadata obligatoria de `*PGM`
- definir cómo se marca origen de toolchain y validez del binario
- integrar esa metadata con xattrs y/o cabecera propia
- adaptar loader y runtime para validar el contrato de `*PGM`
- migrar `clc` desde generación `stub` a backend real con subset explícito
- mantener un subset CL corto pero verdadero:
  - resolución de objetos
  - mensajes
  - operaciones básicas del runtime

### Entregables

- especificación de `*PGM`
- `clc` sin flujo soportado basado en `stub`
- validación de carga de `*PGM`

### Criterio de aceptación

- `clc` y `c400c` generan `*PGM` aceptados por el runtime soportado
- el kernel/runtime pueden distinguir un ELF Linux genérico de un `*PGM` Linux/400 válido

## Fase 5. Subsistemas y Jobs Reales

### Objetivo

Reemplazar demos/fallbacks por operación real de `QINTER` y `QBATCH`.

### Decisiones

- `cgroups v2` se toma como mecanismo obligatorio.
- `sched_ext` queda fuera del camino crítico.
- primero se cierra observabilidad y control de jobs; luego se evalúa scheduler BPF.

### Trabajo

- formalizar el job registry persistente
- implementar cola mínima de batch o equivalente a `SBMJOB`
- asegurar que la TUI muestre jobs reales por defecto
- definir estados de job consistentes
- validar herencia de recursos entre sesiones interactivas y batch
- alinear parámetros con la intención de `PROJECT.md`:
  - prioridad de `QINTER`
  - límites de memoria
  - pesos de CPU e I/O

### Entregables

- jobs interactivos y batch reales
- TUI sin dependencia primaria de datos simulados
- política de subsistemas documentada

### Criterio de aceptación

- puede enviarse y observarse un job batch real
- la separación `QINTER`/`QBATCH` es visible y medible

## Fase 6. `*USRPRF`, Autorizaciones y Administración

### Objetivo

Agregar la capa operativa que convierte el sistema de objetos en una personalidad OS/400 usable.

### Decisiones

- `*USRPRF` no se implementa como simple archivo decorado; debe vincularse con identidad Linux y permisos Linux/400.
- las autorizaciones se definen sobre bibliotecas, objetos y comandos, no sólo sobre archivos del host.

### Trabajo

- modelar `*USRPRF` con metadata y representación soportada
- definir sincronización o mapping con `/etc/passwd`
- implementar autorizaciones por:
  - biblioteca
  - objeto
  - comando
- exponer operaciones mínimas de administración en TUI y/o CL

### Entregables

- perfiles utilizables
- autorizaciones básicas operativas
- administración mínima del sistema

### Criterio de aceptación

- un usuario Linux/400 puede tener permisos diferenciados sobre bibliotecas y objetos
- la TUI y el runtime respetan esas autorizaciones

## Fase 7. Memoria Etiquetada y Persistencia de Referencias

### Objetivo

Cerrar la convergencia con la parte más ambiciosa de `PROJECT.md` sin bloquear antes las capas fundamentales.

### Decisiones

- esta fase se divide en diseño obligatorio e implementación progresiva.
- `LAM`/`TBI` sí forman parte de la arquitectura objetivo.
- `DAX` se trata como aceleración avanzada y no como requisito de la primera convergencia.

### Trabajo

#### Etapa A. Diseño y ABI

- definir ABI de tagging para x86_64 y arm64
- definir qué bits se usan y cómo interactúan con el runtime
- definir contrato entre pointer tagging y tipos de objeto
- definir esquema de mapeo determinista por UUID/identidad de objeto

#### Etapa B. Runtime básico

- activar `LAM` o `TBI` por proceso donde el kernel lo soporte
- introducir primitivas de `mmap` determinista
- validar round-trip de referencias persistidas

#### Etapa C. Perfil avanzado

- evaluar DAX sobre plataformas y storage soportados
- definir si DAX entra como perfil “advanced/full+” o como requisito futuro de una v2 arquitectónica

### Entregables

- diseño formal del ABI de memoria
- runtime mínimo de tagging y mapping
- evaluación técnica de DAX

### Criterio de aceptación

- existe una implementación funcional de punteros etiquetados en plataformas soportadas
- el sistema puede re-mapear objetos de forma determinista con contrato estable

## Dependencias entre fases

- Fase 1 habilita validación seria de Fases 2 a 7.
- Fase 2 es prerequisito de Fases 3, 4 y 6.
- Fase 3 es prerequisito de Fase 4.
- Fase 4 y Fase 5 pueden avanzar parcialmente en paralelo una vez cerradas Fases 2 y 3.
- Fase 6 depende de Fases 2 y 3.
- Fase 7 puede iniciar por diseño desde el principio, pero su implementación fuerte no debe interrumpir el cierre de Fases 2 a 6.

## Priorización resumida

Prioridad alta:

- Fase 1
- Fase 2
- Fase 3
- Fase 4

Prioridad media:

- Fase 5
- Fase 6

Prioridad estratégica, no bloqueante al inicio:

- Fase 7

## Criterio de cierre de este plan

Este plan se considera cumplido cuando:

1. Linux/400 tiene plataforma soportada y validación E2E reproducible.
2. Los objetos viven sobre ZFS con backend Berkeley DB para PF/LF y `*DTAQ`.
3. El object manager tiene enforcement real en modo `full`.
4. `clc` y `c400c` generan `*PGM` válidos para Linux/400.
5. `QINTER` y `QBATCH` son subsistemas reales, no sólo demos.
6. `*USRPRF` y autorizaciones funcionan como capa de operación del sistema.
7. Existe diseño cerrado e implementación inicial realista del modelo de memoria etiquetada.

## Nota de ejecución

Pendiente real después de esta ejecución:

- Fase 4:
  - definir e implementar formato soportado de `*PGM`
  - eliminar el camino soportado basado en `stub` dentro de `clc`
- Fase 5:
  - implementar jobs y batch reales más allá de demos y registros locales
  - dejar `QINTER`/`QBATCH` operando como subsistemas reales del runtime
- Fase 6:
  - modelar `*USRPRF` como objeto operativo real
  - implementar autorizaciones por biblioteca, objeto y comando
- Fase 7:
  - definir e implementar `LAM`/`TBI` con `mmap` determinista por objeto
  - evaluar integración real de DAX como perfil avanzado

Notas sobre lo pendiente:

- la validación E2E live -> install -> reboot -> boot instalado en QEMU UEFI ya quedó resuelta
- la clasificación de plataforma (`full`/`degraded`/`dev`) ya existe; el contrato de enforcement de Fase 3 ahora también queda validado por `support-profile` contra hooks y versión de política, y la ejecución live de `full` depende de una plataforma con `BPF LSM`, BTF y capacidad real de adjuntar mapas/hooks
- en Fase 2 ya existe capa de storage explícita, backend Berkeley DB real integrado, PF/LF/DTAQ operando por defecto sobre Berkeley DB y compatibilidad transitoria con `sled`
- en este corte queda consolidada la creación de bibliotecas sobre datasets ZFS como camino por defecto cuando el root vive sobre ZFS o se define `L400_ZFS_DATASET_PREFIX`; `L400_ZFS_CREATE_DATASETS` queda como opt-out explícito
- en este corte Fase 3 queda cerrada a nivel de implementación con política documentada, hook de ejecución real y trazabilidad adicional en `loader-status`
- el mayor faltante pasa a ser la convergencia de formato `*PGM`, ejecución soportada sin `stub` y subsistemas reales
