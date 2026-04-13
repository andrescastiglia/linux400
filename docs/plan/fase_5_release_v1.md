# Fase 5: Release Candidate V1

## Objetivo

Empaquetar una v1 reproducible, demostrable y documentada.

## Alcance

- artefactos de build
- documentación de instalación/uso
- matriz de soporte
- demo de producto

## Trabajo

### 1. Perfil de soporte

- fijar plataforma oficial v1
- definir qué significa `full`, `degraded` y `dev`
- documentar prerequisitos de kernel, ZFS, BPF y QEMU

### 2. Documentación operativa

- instalación desde ISO
- primer boot
- recovery
- uso de TUI
- uso de compiladores
- demo de objetos

### 3. Validación final

- smoke tests del sistema live
- smoke tests del sistema instalado
- checklist de release

### 4. Demo v1

- boot
- instalación
- ingreso al TUI
- compilación de un programa
- uso de un objeto o cola de datos

## Entregables

- ISO RC de Linux/400 v1
- documentación de instalación y operación
- checklist de release

## Criterio de aceptación

- una tercera persona puede reproducir la demo de v1 en QEMU siguiendo la documentación del repo
