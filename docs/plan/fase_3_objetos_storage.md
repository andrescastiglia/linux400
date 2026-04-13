# Fase 3: Objetos y Storage V1

## Objetivo

Cerrar una implementación v1 usable del modelo de objetos de Linux/400.

## Decisión de V1

Para v1, el backend operativo actual se toma como base real del producto:

- `sled` como storage funcional de objetos de datos
- ZFS y xattrs como capa de metadata/enforcement en modo soportado

Berkeley DB queda como línea posible de evolución futura, no como prerequisito para cerrar v1.

## Alcance

- `libl400/src/object.rs`
- `libl400/src/db.rs`
- `libl400/src/dtaq.rs`
- integración con `lam`, `cgroup`, compiladores y TUI

## Trabajo

### 1. Modelo de objetos

- endurecer creación, listado, lookup y borrado de objetos
- formalizar tipos soportados en v1
- dejar reglas claras para `*LIB`, `*PGM`, `*FILE`, `*DTAQ`, `*USRPRF`

### 2. Librerías y catálogos

- definir layout y convenciones de bibliotecas
- asegurar que los compiladores cataloguen correctamente los `*PGM`

### 3. Storage usable

- validar PF/LF/DTAQ en flujo real
- revisar persistencia y errores de borde
- escribir demo funcional de objetos

### 4. Enforcement básico

- alinear xattrs, loader y runtime
- validar al menos un caso real de acceso permitido y denegado

## Entregables

- demo de objetos v1
- storage funcional documentado
- enforcement básico reproducible

## Criterio de aceptación

- desde el sistema instalado se pueden crear/listar/usar objetos Linux/400
- existe al menos una prueba funcional de tipado fuerte
