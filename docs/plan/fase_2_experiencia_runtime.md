# Fase 2: Experiencia y Runtime

## Objetivo

Convertir el sistema booteable en una experiencia Linux/400 coherente.

## Alcance

- `scripts/runtime/l400-session.sh`
- `scripts/runtime/l400-console-autologin.sh`
- `os400-tui/`
- bootstrap de login y consola

## Trabajo

### 1. Flujo principal

- `tty1` debe entrar al flujo Linux/400
- `ttyS0` debe quedar disponible para testing y recovery
- `rescue` debe abrir shell sin ambigüedad

### 2. TUI como shell principal

- fortalecer el fallback a shell si falla la TUI
- asegurar que el arranque normal llegue al menú principal
- revisar accesos rápidos y navegación mínima para operación

### 3. Consistencia live/installed

- el comportamiento del login debe ser equivalente en live e instalado
- el usuario no debe caer en un shell genérico salvo en casos de recovery

## Entregables

- boot normal a TUI
- sesión Linux/400 consistente
- consola de recuperación separada

## Criterio de aceptación

- el sistema instalado arranca al TUI en `tty1`
- el acceso serial permite depuración sin romper la experiencia principal
