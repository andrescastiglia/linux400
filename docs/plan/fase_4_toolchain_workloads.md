# Fase 4: Toolchain y Workloads

## Objetivo

Hacer que compilación, ejecución y gestión de trabajos sean parte real de la v1.

## Alcance

- `cl_compiler/clc/`
- `c400_compiler/`
- `l400-loader/`
- `l400-ebpf/`
- `l400-ebpf-common/`
- `libl400/src/cgroup.rs`
- pantallas TUI relacionadas con jobs/comandos

## Trabajo

### 1. Toolchain dentro del sistema

- validar `clc` y `c400c` dentro del live e instalado
- definir subset soportado de CL para v1
- dejar ejemplos canónicos y pruebas mínimas

### 2. Runtime de programas

- compilar fuente -> emitir binario -> catalogar `*PGM` -> ejecutar
- revisar paths de runtime y dependencias en el sistema instalado

### 3. Loader y eBPF

- validar carga real del hook en el boot o temprano en userspace
- definir diferencias entre modo `full`, `degraded` y `dev`

### 4. Workloads y subsistemas

- confirmar funcionamiento de `qinter` y `qbatch`
- mostrar trabajos de manera visible desde TUI
- agregar una demo simple de batch

## Entregables

- programas CL y C/400 compilados desde el sistema
- un job interactivo y uno batch observables
- loader/eBPF documentados por modo de operación

## Criterio de aceptación

- un usuario puede compilar y ejecutar un programa simple dentro de Linux/400
- la separación entre carga interactiva y batch es visible y verificable
