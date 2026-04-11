# Análisis y Plan de Ejecución: Fase 7 — Frontend TUI (Green Screen) e Interfaces de Consola

## 1. Contexto y Objetivos

OS/400 presenta una interfaz de usuario característica basada en "Green Screens" - terminales de texto de alta densidad con caracteres ASCII/IBM que presentan menús, formularios y listas de trabajo. Linux/400 replica esta experiencia usando **Ratatui** (biblioteca TUI moderna en Rust) para crear una interfaz similar que mantiene la familiaridad del sistema IBM i mientras aprovecha las capacidades de Linux.

El objetivo de Fase 7 es:
1. Crear un menú principal interactivo que sirva como punto de entrada
2. Implementar paneles de gestión de trabajo (WRKACTJOB, etc.)
3. Soportar atajos de teclado clásicos (F3=Exit, F4=Prompt, F12=Cancel)
4. Integrar con los módulos existentes de libl400 (cgroup, dtaq, db)

## 2. Arquitectura de la TUI

### 2.1 Stack Tecnológico

```
┌─────────────────────────────────────┐
│           os400-tui (bin)           │
│  Ratatui + Crossterm (terminal I/O) │
├─────────────────────────────────────┤
│            libl400                  │
│  (cgroup, dtaq, db, lam, object)   │
└─────────────────────────────────────┘
```

### 2.2 Pantallas Principales

```
┌──────────────────────────────────────────────────────────┐
│ L400 Main Menu                                    QSYS   │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Work with objects                              System:  │
│                                                          │
│  Type options, press Enter.                        A01    │
│                                                          │
│  1. Work with libraries . . . . . . . . . . .   WRKLIB  │
│  2. Work with programs  . . . . . . . . . . .   WRKPGM  │
│  3. Work with files . . . . . . . . . . . . .  WRKOBJ  │
│  4. Work with jobs . . . . . . . . . . . . .   WRKACTJOB│
│  5. Data queues  . . . . . . . . . . . . . .  DSPDTAQ  │
│  6. Command entry . . . . . . . . . . . . . .  CMD      │
│                                                          │
│  10. System configuration  . . . . . . . . .  CFG       │
│                                                          │
├──────────────────────────────────────────────────────────┤
│ F3=Exit   F4=Prompt   F12=Cancel                       │
└──────────────────────────────────────────────────────────┘
```

### 2.3 Atajos de Teclado

| Tecla | Función | Descripción |
|-------|---------|-------------|
| F1 | Help | Mostrar ayuda contextual |
| F3 | Exit | Salir de la pantalla actual |
| F4 | Prompt | Abrir prompt de comando |
| F5 | Refresh | Actualizar datos en pantalla |
| F10 | Command | Mostrar línea de comandos |
| F11 | Toggle | Alternar vista (ej. datos/definición) |
| F12 | Cancel | Cancelar y volver atrás |
| Enter | Select | Ejecutar selección actual |
| PageUp/Down | Scroll | Navegar páginas de datos |

## 3. Módulos de Pantalla

### 3.1 MainMenu

```rust
pub struct MainMenu;
impl Screen for MainMenu {
    fn render(&mut self, frame: &mut Frame);
    fn handle_input(&mut self, key: KeyEvent) -> ScreenResult;
}
```

### 3.2 WorkManagement (WRKACTJOB)

Muestra todos los trabajos en el sistema:
- Tipo: INTERACTIVE (QINTER), BATCH (QBATCH), SYSTEM
- Estado: ACTIVE, WAIT, JOBQ
- Usuario, trabajo, subsistema

### 3.3 ObjectBrowser (WRKOBJ)

Explorador de objetos del sistema:
- Navegación de libraries (*LIB)
- Vista de objetos por tipo
- Operaciones: DSP, EDTOBJ, RMV

### 3.4 DataQueueViewer (DSPDTAQ)

Visor de colas de datos:
- Mensajes encolados
- Envío de mensajes
- Limpieza de cola

### 3.5 CommandLine

Línea de comandos estilo OS/400:
- Parsing de comandos CL
- Historial de comandos
- Autocompletado básico

## 4. API de Navegación

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenId {
    MainMenu,
    WorkManagement,
    ObjectBrowser,
    DataQueueViewer,
    CommandLine,
    Exit,
}

pub trait Screen {
    fn render(&mut self, frame: &mut Frame);
    fn handle_input(&mut self, key: KeyEvent) -> ScreenResult;
}

pub struct ScreenResult {
    pub next: Option<ScreenId>,
    pub data: Option<ScreenData>,
}

pub enum ScreenData {
    SelectedLibrary(String),
    SelectedObject(String),
    Command(String),
}
```

## 5. Diseño Visual (Green Screen Style)

### 5.1 Colores

```rust
const STYLE_HEADER: Style = Style::new()
    .on_blue()
    .white();

const STYLE_BORDER: Style = Style::new()
    .cyan();

const STYLE_SELECTION: Style = Style::new()
    .on_cyan()
    .black();

const STYLE_ERROR: Style = Style::new()
    .red();

const STYLE_HELP: Style = Style::new()
    .black()
    .on_white();
```

### 5.2 Tipografía

- Fuente: Monospace (Terminal)
- Caracteres: ASCII extendido
- Densidad: Alta (80 columnas)

## 6. Integración con libl400

```rust
use l400::{
    cgroup::{assign_to_workload, WorkloadType},
    dtaq::{crtdtaq, DataQueue},
    db::{create_pf, PhysicalFile},
    object::{list_objects, create_object},
};

fn display_work_management() -> Result<(), Error> {
    let jobs = get_all_jobs()?;
    let mut table = Vec::new();
    
    for job in jobs {
        let workload = get_current_workload()
            .unwrap_or(WorkloadType::Batch);
        table.push(vec![
            job.name.clone(),
            job.user.clone(),
            workload_to_str(workload),
        ]);
    }
    
    Ok(())
}
```

## 7. Estructura del Crate

```
os400-tui/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Module exports
│   ├── app.rs            # App state management
│   ├── screens/
│   │   ├── mod.rs
│   │   ├── main_menu.rs
│   │   ├── work_mgmt.rs
│   │   ├── object_browser.rs
│   │   ├── dtaq_viewer.rs
│   │   └── cmd_line.rs
│   ├── widgets/
│   │   ├── mod.rs
│   │   ├── table.rs
│   │   ├── form.rs
│   │   └── help_bar.rs
│   └── style.rs          # Green screen styles
└── README.md
```

## 8. Tests

```bash
cargo test -p os400-tui
cargo run --bin os400-tui  # Interactive test
```

## 9. Dependencias

```toml
[dependencies]
ratatui = "0.26"
crossterm = "0.27"
l400 = { path = "../libl400" }
anyhow = "1.0"
tokio = { version = "1", features = ["full"] }
```

## 10. Riesgos Asumidos

| Riesgo | Mitigación |
|--------|------------|
| Terminal sin soporte de colores | Detectar y usar modo monocromático |
| Pantalla pequeña | Scroll y paginación |
| Input no bloqueante | Usar `crossterm::event` con timeout |
| Caracteres no ASCII | UTF-8 con fallback a ASCII |

## 11. Métricas de Éxito

- [x] Menú principal visible con todas las opciones
- [x] Navegación entre pantallas funcional
- [x] F3/Cancel sale de pantallas correctamente
- [x] WRKACTJOB muestra información de trabajos
- [x] WRKOBJ permite navegar objetos
- [x] DSPDTAQ muestra mensajes de cola
- [x] Compilación sin errores
- [x] Tests pasando
