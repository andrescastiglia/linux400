# Plan de implementación: Linux/400 0.2.0 — Foco en experiencia AS/400 y usabilidad

Fecha: 2026-05-01  
Release: 0.2.0 (sin cambio de versión)  
Reemplaza todos los planes de implementación previos. Este plan inicia desde **Fase 1**, priorizando:
- Replicar fielmente la interfaz y experiencia de OS/400/AS/400
- Corregir críticos problemas de usabilidad (backspace, F4, navegación)
- Garantizar que los comandos sean funcionales y operativos

---

## Diagnóstico actual

### Fortalezas existentes
- Sign-on con autenticación PAM real y bloqueo de ROOT.
- Command line con historial y tokenizer CL.
- Object browser, work management, PDM/SEU/SQL básicos.
- PF/LF/DTAQ con backend `sled`, CL/C toolchain, loader eBPF.
- Smoke tests automatizados para TUI.

### Brechas críticas (prioridad alta)
| Área | Brecha |
| --- | --- |
| **Usabilidad básica** | Backspace no funciona en campos de entrada; F4 no muestra el prompt de parámetros de comandos; navegación con F3/F12 pierde contexto. |
| **Apariencia AS/400** | Menú principal y pantallas no se asemejan a OS/400: falta layout 5250 clásico, ruler lines, línea de comandos persistente inferior, mapeo estándar de teclas F. |
| **Comandos funcionales** | `PWRDWNSYS` no apaga/reinicia el sistema real (solo dry-run); varios comandos son stubs sin implementación operativa. |
| **Arquitectura TUI** | Screen trait síncrono, sin soporte para eventos asíncronos; stack de navegación limitado a un nivel. |
| **Widgets** | Falta de campos de entrada con longitud visible, tablas paginadas, popups de confirmación estándar. |

---

## Objetivo general
Que Linux/400 0.2.0 sea un sistema operativo con experiencia idéntica a OS/400, donde el operador pueda:
1. Autenticarse y operar exclusivamente desde la TUI tipo 5250.
2. Usar menús y comandos que replican fielmente el comportamiento de AS/400.
3. Contar con manejo de entrada correcto (backspace, F4, flechas) en todas las pantallas.
4. Ejecutar comandos 100% funcionales (ej. `PWRDWNSYS` apaga el sistema, `CRTPF` crea archivos reales).
5. Navegar sin pérdida de contexto, con feedback visual claro y mensajes CPF consistentes.

---

## Reglas de ejecución
- Prioridad absoluta a la fidelidad con la UI/UX de OS/400/AS/400.
- Todo campo de entrada debe soportar backspace, delete, navegación con flechas y auto-uppercase para nombres de objetos.
- F4 siempre abre el prompt de parámetros para comandos válidos, con descripción de tipos y validación.
- Todas las pantallas siguen el layout 5250: ruler lines, línea de comandos inferior, F-keys mapeados a funciones estándar.
- Los comandos deben tener implementación funcional real, no solo stubs (ej. `PWRDWNSYS` invoca `systemctl` o `/sbin/shutdown` con validación de autoría).
- `cargo test -p os400-tui` es el gate mínimo para cada PR, con tests específicos de usabilidad y fidelidad AS/400.
- Modos `dev`, `degraded` y `full` degradan con mensajes CPF, nunca con errores de Linux crudos.

---

## Fase 1: Fundación UI AS/400 y correcciones de usabilidad
Estado: **En progreso para 0.2.0**  
Objetivo: Establecer las bases de la interfaz tipo OS/400 y corregir los problemas de usabilidad críticos.

### 1.1 Corrección de manejo de entrada de texto
- [x] Backspace funciona correctamente en `InputField`, `CommandInput` y editor SEU.
- [x] Soporte para teclas Delete, Arrow Left/Right, Home/End en campos de texto.
- [x] Auto-uppercase en campos de nombres de objetos, comandos y rutas (como OS/400).
- [x] Test automatizado de entrada de texto para todos los widgets.

### 1.2 Implementación de F4 Prompt de comandos
- [x] Al presionar F4 en la línea de comandos o ejecutar un comando, se muestra el prompt de parámetros del *CMD correspondiente.
- [x] El prompt replica el layout de OS/400: descripción del parámetro, tipo, longitud, valores posibles y ayuda contextual.
- [x] Validación de parámetros al ejecutar el comando, con mensajes CPF de error específicos.
- [x] F4 funciona en todos los comandos catalogados en `COMMAND_METADATA`.

### 1.3 Rediseño del menú principal estilo OS/400
- [x] Rediseñar `MainMenu` para replicar el layout 5250 clásico:
  - Ruler line superior con nombre del sistema, usuario y fecha/hora.
  - Opciones numeradas con descripción corta y mapeo a F-keys.
  - Ruler line inferior con F-keys estándar (F3=Salir, F4=Prompt, F12=Cancelar, F24=More keys).
  - Línea de comandos persistente (`===>`) en la parte inferior de la pantalla.
- [x] Navegación coherente: F3 cierra sesión, F12 vuelve a la pantalla anterior, Enter ejecuta la opción seleccionada.
- [x] Opciones del menú mapeadas a comandos reales, no stubs.

### 1.4 Comandos funcionales: PWRDWNSYS
- [x] `PWRDWNSYS` ejecuta apagado (`poweroff`) o reinicio (`reboot`) real del sistema, validando privilegios de root/autoridad *ALLOBJ.
- [x] Confirmación obligatoria con `ConfirmDialog` estilo OS/400.
- [x] Opción `CONFIRM(*YES)` para ejecución no interactiva.
- [x] Variable `L400_PWRDWNSYS_DRY_RUN=1` mantiene el modo prueba para desarrollo.

### 1.5 Estilo 5250 base
- [x] Paleta de colores idéntica a OS/400: verde claro sobre fondo negro, campos activos en verde brillante, campos protegidos en verde apagado.
- [x] Ruler lines (`========`) en todas las pantallas, como AS/400.
- [x] Indicador de cursor visible (subrayado o inverso) en todos los campos de entrada.
- [x] Definir constantes de estilo en `style.rs` para mantener consistencia.

Criterio de cierre Fase 1:
- [x] Backspace y navegación de texto funcionan en todos los campos.
- [x] F4 muestra el prompt de parámetros para comandos *CMD válidos.
- [x] Menú principal replica el layout básico de OS/400.
- [x] `PWRDWNSYS` apaga/reinicia el sistema real con autorización válida.
- [x] `cargo test -p os400-tui` pasa sin regresiones, con tests de usabilidad y fidelidad AS/400.

---

## Fase 2: Navegación y pantallas operativas (próxima)
Estado: **Pendiente**  
Objetivo: Implementar stack de navegación real, pantallas faltantes y widgets reutilizables.

### 2.1 Stack de navegación
- [x] Reemplazar `previous_screen: Option<ScreenId>` con `Vec<NavEntry>` (stack LIFO).
- [x] F3/F12 hacen pop del stack, manteniendo contexto de retorno.
- [x] Límite de 16 entradas en el stack para evitar fugas.

### 2.2 Widgets reutilizables
- [x] `SubfileTable`: tabla paginada con scroll vertical, opciones numéricas por fila.
- [x] `ConfirmDialog`: popup modal estándar para acciones destructivas.
- [x] `StatusBar`: barra inferior con reloj, usuario, biblioteca actual, job y estado del sistema.

### 2.3 Pantallas faltantes
- [ ] `WRKLIB` dedicado con opciones de crear/borrar/renombrar bibliotecas.
- [ ] `DSPOBJD` con layout de campos estilo OS/400, no texto plano.
- [ ] `WRKSYSVAL` editable con F4 prompt para valores.

---

## Gates permanentes
Gate rápido local:
```bash
cargo fmt --all --check
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
```

Gate de calidad:
```bash
cargo clippy -p l400 --all-targets -- -D warnings
cargo clippy -p os400-tui --all-targets -- -D warnings
./scripts/test/test_release_rc.sh
```

Gate de comandos funcionales:
```bash
L400_PWRDWNSYS_DRY_RUN=1 cargo run -p l400 --bin l400cmd -- PWRDWNSYS 'OPTION(*RESTART)' 'CONFIRM(*YES)'
```
