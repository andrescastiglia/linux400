# Plan de implementación: Linux/400 siguiente nivel — foco TUI

Fecha de corte: 2026-04-30.

Este plan reemplaza la hoja de ruta anterior. Todas las fases previas (0–8) se
consideran **finalizadas**. La base v1/0.2-pre ya entrega: objetos, comandos,
TUI con sign-on/menu/PDM/SEU/SQL/jobs/spool, PF/LF/DTAQ, CL/C toolchain,
loader eBPF, instalador y smoke tests.

El siguiente nivel convierte la TUI de un visor funcional en una **consola
operativa primaria** del sistema, tal como define `docs/KERNEL.md`:

> *"La shell Linux debe quedar como herramienta de soporte, instalación,
> desarrollo interno o rescue, no como interfaz principal del sistema."*

---

## Diagnóstico actual de la TUI

### Fortalezas existentes

- Sign-on con autenticación PAM real y bloqueo de ROOT.
- Menú principal con 12 opciones funcionales.
- Command line con historial, prompt F4, tokenizer CL con comillas.
- Object browser conectado a runtime con opciones 2/3/4/5/8.
- Work management con hold/release/end/detail/log y filtro por subsistema.
- PDM → WRKMBRPDM → STRSEU flujo completo de edición de fuentes.
- STRSQL interactivo con historial y scroll horizontal.
- Spool/OUTQ con tabla, filtro por estado, hold/release/save/delete.
- HelpBar con metadata de `*CMD` y CpfMessage unificada.
- Smoke tests automatizados en `phase6_tui_smoke.rs` (render 80/132 + flujo
  end-to-end).

### Brechas que frenan la consola operativa

| Área | Brecha |
| --- | --- |
| **Arquitectura** | Screen trait es síncrono (`handle_key` → `ScreenResult`); no hay forma de refrescar datos en background, recibir eventos asíncronos ni componer pantallas. |
| **Navegación** | Sin stack de pantallas real; el `previous_screen` es un solo `Option<ScreenId>` y pierde contexto en navegación profunda (ej. Menu→PDM→WRKMBRPDM→SEU→F3 vuelve a WRKMBRPDM pero un segundo F3 no vuelve a PDM). |
| **Pantallas faltantes** | `WRKLIB` dedicado, `DSPOBJD` con layout de campos (no texto plano), `WRKSYSVAL` editable, `DSPLOG` visual, `DSPPFM` con scroll horizontal real, visor de auth/autorizaciones interactivo. |
| **UX 5250** | No hay indicador de campo activo (cursor), los campos de entrada no tienen longitud visible, no hay subfile paging, no hay command line persistente en la parte inferior de cada pantalla. |
| **Command line global** | Cada pantalla implementa F4→CommandLine como navegación, pero el operador pierde contexto. OS/400 tiene la command line siempre visible. |
| **Estilo** | El estilo es funcional pero no imita el layout 5250 clásico: faltan ruler line, indicador de posición, separador de subfile, status bar con reloj/job/lib. |
| **Widgets** | Solo existe `HelpBar` y `CpfMessage` como widgets reutilizables. Faltan: campo de entrada con longitud, tabla paginada, popup de confirmación, subfile, indicador de modo. |
| **Resize** | `Event::Resize` se ignora. Paneles con layout fijo se rompen en terminales pequeñas. |
| **Tests** | Cobertura de render es buena, pero no hay tests de navegación profunda multi-pantalla, ni regression de layout. |
| **Duplicación** | `tokenize_cl_command` está duplicado en `cmd_line.rs` y `admin_views.rs`. |

---

## Objetivo

Que un operador pueda usar Linux/400 exclusivamente desde la TUI para:

1. Autenticarse y llegar al menú principal.
2. Navegar bibliotecas y objetos con opciones de contexto.
3. Crear, editar, compilar y ejecutar programas CL sin salir de la TUI.
4. Administrar jobs interactivos y batch con spool.
5. Operar PF/LF/DTAQ con SQL y comandos.
6. Administrar perfiles, autorizaciones y auditoría.
7. Ver estado de sistema, política y valores.
8. Ejecutar cualquier comando desde una línea de comandos persistente.
9. Apagar/reiniciar con `PWRDWNSYS` desde la interfaz.
10. Hacer todo lo anterior con feedback visual claro, confirmaciones
    apropiadas, y degradación explícita cuando falte runtime.

---

## Reglas de ejecución

- La TUI es el producto principal; cada mejora debe ser verificable visualmente.
- Todo widget nuevo debe funcionar a 80 y 132 columnas.
- La navegación debe preservar contexto de retorno (stack, no `Option`).
- No se usa `split_whitespace` para tokenizar CL; usar el tokenizer compartido.
- Cada pantalla nueva tiene test unitario de teclas + test de render a 80/132.
- `cargo test -p os400-tui` es el gate mínimo para cada PR.
- Los modos `dev`, `degraded` y `full` deben degradar con mensajes CPF, no
  con panics o texto Linux crudo.

---

## Fase 1: fundación de widgets y arquitectura TUI

Estado: **en progreso para 0.3-pre**.

Objetivo: construir la infraestructura que sostiene todas las pantallas
siguientes, eliminando deuda técnica.

### 1.1 Widget library (`widgets/`)

- [x] `InputField`: campo de texto con longitud visible, cursor, máscara de
  password, uppercase auto, validación por tipo (alpha, numeric, name, path).
- [x] `SubfileTable`: tabla paginada con scroll vertical, opciones numéricas
  por fila, highlight de selección, columnas con ancho configurable y auto-fit.
  Reemplaza la duplicación de `Table`+`TableState` en cada pantalla.
- [x] `ConfirmDialog`: popup modal de confirmación (Enter/F12) con texto
  configurable. Reemplaza `pending_delete`/`pending_action` ad-hoc.
- [x] `StatusBar`: barra inferior con reloj, user, current library, job,
  system name, último CPF. Presente en todas las pantallas.
- [x] `CommandInput`: línea de comandos embebida (1 línea) con autocompletado
  del catálogo `*CMD`, integrable en la parte inferior de cualquier pantalla.
- [x] `ModeIndicator`: badge que muestra `FULL`, `DEGRADED` o `DEV` con color
  semántico (verde/amarillo/rojo).

### 1.2 Refactor de `tokenize_cl_command`

- [x] Mover `tokenize_cl_command` y `extract_command_arg` a un módulo
  `cl_parser` en `os400-tui`.
- [x] Eliminar la duplicación entre `cmd_line.rs` y `admin_views.rs`.

### 1.3 Navigation stack

- [x] Reemplazar `previous_screen: Option<ScreenId>` con un
  `Vec<NavEntry>` que actúe como stack LIFO.
- [ ] `F3`/`F12` hacen pop del stack, no un hard-goto a `MainMenu`.
- [x] El stack se limita a ~16 entradas para evitar fugas.
- [x] Agregar `ScreenResult::back()` como alias de pop.

### 1.4 Resize handling

- [x] Propagar `Event::Resize` a la pantalla activa.
- [ ] Cada pantalla debe re-layout en resize sin perder estado.
- [ ] Test: render a 80×24, resize a 132×43, render de nuevo sin panic.

### 1.5 Estilo 5250 mejorado

- [x] Agregar variantes de color para campo activo, campo protegido, campo
  de alta intensidad, separador de subfile.
- [x] Definir constantes en `style.rs`:
  - `STYLE_INPUT_ACTIVE`, `STYLE_INPUT_PROTECTED`, `STYLE_SUBFILE_SEPARATOR`,
    `STYLE_STATUS_BAR`, `STYLE_MODE_FULL`, `STYLE_MODE_DEGRADED`,
    `STYLE_MODE_DEV`.

Criterio de cierre:

- [ ] Los 6 widgets tienen unit tests.
- [ ] `tokenize_cl_command` tiene una sola definición.
- [x] El navigation stack se prueba con un flujo de 4 niveles de profundidad.
- [x] `cargo test -p os400-tui` pasa sin regresiones.

---

## Fase 2: pantalla de sign-on y status bar global

Estado: **en progreso para 0.3-pre**.

Objetivo: que la primera impresión del sistema transmita identidad y que la
status bar sea omnipresente.

### 2.1 Sign-on mejorado

- [x] Usar `InputField` para User y Password.
- [x] Agregar campo `Current library` (default `QGPL`).
- [x] Agregar campo `Initial menu` (default `MAIN`) — visual solamente.
- [ ] Mostrar nombre de sistema, versión y modo (FULL/DEGRADED/DEV) en el
  panel de sign-on.
- [ ] Animación de arranque mínima: banner "Linux/400" con versión al
  iniciar la TUI, antes de mostrar sign-on (1 segundo).

### 2.2 Status bar global

- [x] Integrar `StatusBar` en `App::run()`, renderizando siempre la última
  fila del terminal.
- [x] Contenido: `System: L400  User: QPGMR  Lib: TESTLIB  Job: 12345  HH:MM:SS  [FULL]`.
- [x] Actualización del reloj cada segundo (usando tick en el event loop).
- [ ] Último CPF con severity (info/warning/error coloreado).

Criterio de cierre:

- [x] Sign-on usa widgets de la fase 1.
- [x] La status bar aparece en todas las pantallas.
- [ ] El banner de arranque se puede desactivar con `L400_NO_BANNER=1`.

---

## Fase 3: menú principal y navegación multinivel

Estado: **en progreso para 0.3-pre**.

Objetivo: que el menú principal sea un hub efectivo y que la navegación sea
predecible.

### 3.1 Menú principal enriquecido

- [ ] Reorganizar opciones con separadores visuales por categoría:
  - **Objetos**: 1=Bibliotecas, 2=Objetos, 3=Archivos
  - **Desarrollo**: 4=PDM, 5=SQL, 6=Command entry
  - **Operaciones**: 7=Jobs, 8=System status, 9=System values
  - **Administración**: 10=Users, 11=Spool, 12=Policy/Audit
  - **Sistema**: 90=PWRDWNSYS
- [x] Agregar `GO` con submenús: `GO MAIN`, `GO CMDOBJ`, `GO CMDSQL`,
  `GO CMDSYS`.
- [ ] La selección por teclado numérico ya funciona; agregar feedback
  visual de la opción pendiente en la status bar.

### 3.2 Command line embebida

- [x] Agregar `CommandInput` en la parte inferior del menú principal, justo
  encima de la help bar.
- [x] El operador puede escribir un comando directamente desde el menú sin
  necesidad de ir a opción 6.
- [x] Autocompletado de nombre de comando (Tab) usando `COMMAND_METADATA`.

### 3.3 Menus GO secundarios

- [x] `GO CMDOBJ` — menú de comandos de objetos.
- [x] `GO CMDSQL` — menú de SQL operativo.
- [x] `GO CMDSYS` — menú de sistema y seguridad.
- [x] Implementar como pantallas de menú reutilizando la estructura de
  `MainMenu`.

Criterio de cierre:

- [x] `GO MAIN` desde command line vuelve al menú principal.
- [x] `GO CMDOBJ` muestra un submenú funcional.
- [x] `90` desde menú principal navega a `PWRDWNSYS` (con confirmación).

---

## Fase 4: WRKLIB y object browser de producto

Estado: **pendiente**.

Objetivo: que la administración de bibliotecas y objetos sea fluida y
completa.

### 4.1 WRKLIB dedicado

- [ ] Nueva pantalla `WrkLib` con `SubfileTable`.
- [ ] Listar todas las bibliotecas bajo `L400_ROOT`.
- [ ] Opciones por fila:
  - 2=Cambiar current library
  - 3=Contenido (→ ObjectBrowser con esa library)
  - 4=Borrar (con confirmación)
  - 5=Descripción (→ DSPOBJD)
  - 7=Renombrar
  - 12=Crear nueva biblioteca
- [ ] Filtro por nombre (F17 o campo de filtro).
- [ ] Separar del `ObjectBrowser` actual: opción 1 del menú → WRKLIB,
  opción 2 → ObjectBrowser.

### 4.2 ObjectBrowser mejorado

- [ ] Migrar a `SubfileTable`.
- [ ] Agregar campo de filtro por nombre y tipo de objeto.
- [ ] Agregar opción 7=Renombrar (RNMOBJ con confirmación).
- [ ] Agregar opción 12=Crear objeto nuevo (→ prompt CRTLIB/CRTPF/CRTDTAQ
  según tipo seleccionado).
- [ ] Mostrar tamaño y fecha de creación si están disponibles.
- [ ] Soporte para cambiar de library sin volver al menú (campo Library en
  el header, editable con Tab).

### 4.3 DSPOBJD con layout de campos

- [ ] Reemplazar la vista de texto plano de `AdminCommandView::object_detail`
  por una pantalla dedicada con campos:
  - Object, Library, Type, Attribute, Owner, Text
  - Created, Changed, Last used
  - Size, Storage backend
  - Public authority, Auth manifest summary
- [ ] Opciones desde DSPOBJD: 2=Change text, 8=Authorities.

Criterio de cierre:

- [ ] WRKLIB permite crear y borrar una biblioteca desde TUI.
- [ ] ObjectBrowser soporta filtro funcional.
- [ ] DSPOBJD muestra campos reales de xattrs, no texto plano de l400cmd.

---

## Fase 5: PDM, SEU y ciclo de desarrollo integrado

Estado: **pendiente**.

Objetivo: que el ciclo crear→editar→compilar→ejecutar→debug sea fluido
y autocontenido en la TUI.

### 5.1 STRPDM mejorado

- [ ] Agregar opción de compilar directamente desde WRKMBRPDM (opción 14 o
  F14 = CRTCLPGM del miembro seleccionado).
- [ ] Agregar opción de ejecutar (opción 16 = CALL del PGM correspondiente).
- [ ] Mostrar resultado de compilación inline (popup o panel inferior).
- [ ] Agregar indicador de tipo de miembro (.CLP, .C, .TXT) con color.

### 5.2 STRSEU mejorado

- [ ] Agregar números de línea en el editor.
- [ ] Agregar indicador de línea/columna en la status bar del editor.
- [ ] Soporte para Find (F16) con highlight de ocurrencias.
- [ ] Soporte para Go To Line (F13).
- [ ] F14 desde SEU compila directamente el fuente actual.
- [ ] Mostrar errores de compilación con posición si están disponibles.
- [ ] Undo básico (un nivel) con Ctrl-Z.

### 5.3 Integración de compilación

- [ ] `F14` en WRKMBRPDM y SEU invoca `CRTCLPGM` o `CRTPGM` según el
  tipo de miembro.
- [ ] Si la compilación falla, mostrar spool de errores en un panel popup.
- [ ] Si la compilación tiene éxito, mensaje CPF informativo en la status bar.

Criterio de cierre:

- [ ] Un operador puede crear un miembro, editarlo, compilarlo con F14,
  ver errores, corregirlos y ejecutar el PGM — todo sin salir de
  PDM/SEU.
- [ ] Test automatizado para el flujo F14→compile→popup de resultado.

---

## Fase 6: STRSQL de producto

Estado: **pendiente**.

Objetivo: que el SQL interactivo sea una herramienta operativa real, no solo
un visor.

### 6.1 Mejoras de UX

- [ ] Syntax highlighting básico (keywords SQL en color, strings en otro).
- [ ] Autocompletado de nombres de tabla (Tab) leyendo PFs del catálogo.
- [ ] Multiline input: soporte para queries largas con continuation.
- [ ] Copiar resultado al clipboard (F18).
- [ ] Exportar resultado a spool (F19).

### 6.2 Mejoras funcionales

- [ ] `DESCRIBE TABLE` para mostrar schema de un PF.
- [ ] `SHOW TABLES` para listar PFs accesibles.
- [ ] Manejo de errores SQL con CPF y posición del error.
- [ ] Paginación de resultados para tablas grandes.

Criterio de cierre:

- [ ] `SHOW TABLES` lista PFs reales.
- [ ] Query con error muestra posición y CPF.
- [ ] Resultado exportable a spool.

---

## Fase 7: work management y spool de producto

Estado: **pendiente**.

Objetivo: que jobs y spool sean gestionables de forma completa desde la TUI.

### 7.1 WRKACTJOB mejorado

- [ ] Migrar a `SubfileTable`.
- [ ] Auto-refresh configurable (F21 para toggle, default 5s).
- [ ] Indicador visual de jobs activos vs totales.
- [ ] Filtro combinado por subsistema + usuario + estado.
- [ ] Opción 8=WRKJOB (detalle extendido del job seleccionado).

### 7.2 WRKJOB dedicado

- [ ] Nueva pantalla con tabs: Detail, Log, Spool, Call stack.
- [ ] Tab Detail: campos con name, user, PID, status, subsystem, command,
  timestamps, cgroup path.
- [ ] Tab Log: tail del log con scroll y F5=Refresh.
- [ ] Tab Spool: spools generados por este job.
- [ ] Navegación entre tabs con F11/F12.

### 7.3 SBMJOB desde TUI

- [ ] Agregar comando `SBMJOB` como opción de menú principal (o desde
  command line).
- [ ] Prompt con campos: CMD, JOB, JOBQ, USER.
- [ ] Feedback inmediato: "Job JOBNAME submitted to QBATCH".
- [ ] Redirección a WRKACTJOB mostrando el job recién enviado.

### 7.4 WRKSPLF/WRKOUTQ mejorado

- [ ] Migrar spool a `SubfileTable`.
- [ ] Visor de spool file con scroll horizontal y vertical.
- [ ] Opción de imprimir a stdout (para redirección).
- [ ] Filtro por job, usuario, fecha.

Criterio de cierre:

- [ ] Auto-refresh de WRKACTJOB funciona y se puede desactivar.
- [ ] WRKJOB muestra log en tiempo real.
- [ ] WRKSPLF permite visualizar spool con scroll completo.

---

## Fase 8: administración de seguridad y sistema

Estado: **pendiente**.

Objetivo: que el operador pueda administrar seguridad y configuración
desde la TUI.

### 8.1 WRKUSRPRF dedicado

- [ ] Nueva pantalla con `SubfileTable` listando perfiles de `L400_ROOT`.
- [ ] Opciones: 2=Crear, 3=Copiar, 4=Deshabilitar, 5=Mostrar detalle,
  7=Renombrar.
- [ ] Detalle de perfil: nombre, UID, estado, autoridades asignadas, last
  signon.

### 8.2 DSPOBJAUT / EDTOBJAUT interactivo

- [ ] Nueva pantalla que muestra la matriz de autorización de un objeto.
- [ ] Columnas: User, Authority, Origin (explicit/public/owner).
- [ ] Opciones: 1=Grant (*USE/*CHANGE/*ALL), 4=Revoke.
- [ ] Operaciones ejecutan `GRTOBJAUT`/`RVKOBJAUT` vía runtime.

### 8.3 WRKSYSVAL editable

- [ ] Nueva pantalla que lista system values.
- [ ] Opción 2=Change para valores editables.
- [ ] Opción 5=Display para ver valor actual y descripción.

### 8.4 DSPLOG visual

- [ ] Pantalla dedicada para `QHST` y `QEZJOBLOG`.
- [ ] Filtro por fecha, severidad, tipo de evento.
- [ ] Scroll y refresh.
- [ ] Colores por severidad.

### 8.5 DSPPOLICY mejorado

- [ ] Mostrar estado de enforcement por tipo de objeto.
- [ ] Mostrar versión de política eBPF vs runtime.
- [ ] Indicador de brechas conocidas.
- [ ] Filtros: auth denied, user changes, object changes, all.

Criterio de cierre:

- [ ] WRKUSRPRF permite crear y desactivar un perfil desde TUI.
- [ ] DSPOBJAUT muestra matriz real y permite grant/revoke.
- [ ] DSPLOG muestra entradas reales con filtro funcional.

---

## Fase 9: datos operativos — PF/LF/DTAQ viewers

Estado: **pendiente**.

Objetivo: que los visores de datos sean herramientas operativas, no solo
dumps de texto.

### 9.1 DSPPFM de producto

- [ ] Pantalla dedicada (no SystemPanel genérico).
- [ ] Scroll horizontal para registros anchos.
- [ ] Headers de columna basados en schema PF.
- [ ] Filtro por campo / valor.
- [ ] Opción de editar registro inline (experimental).
- [ ] Indicador de RRN y count total.

### 9.2 DSPDTAQ mejorado

- [ ] Auto-refresh para DTAQ activas.
- [ ] Mostrar timestamp, longitud, y primeros N bytes de cada mensaje.
- [ ] Opción de enviar mensaje (SNDDTAQ) desde la misma pantalla.

### 9.3 Visor de LF

- [ ] Mostrar registros del PF base ordenados por el índice del LF.
- [ ] Indicar nombre del PF base y campos de clave.

Criterio de cierre:

- [ ] DSPPFM muestra columnas con headers del schema.
- [ ] DSPDTAQ permite enviar y recibir desde la misma pantalla.

---

## Fase 10: polish, accesibilidad y testing

Estado: **pendiente**.

Objetivo: que la TUI sea robusta, accesible y verificable.

### 10.1 Accesibilidad

- [ ] Soporte para terminales de 80, 132 y anchos intermedios.
- [ ] Truncamiento inteligente de columnas con "..." en lugar de overflow.
- [ ] Tab order predecible en todas las pantallas.
- [ ] Indicador visual de foco (campo activo siempre distinguible).

### 10.2 Mensajería CPF consistente

- [ ] Toda acción destructiva emite CPF en la status bar.
- [ ] Toda acción exitosa emite CPF informativo.
- [ ] Errores de runtime muestran CPF + texto descriptivo, nunca stack traces.
- [ ] Los mensajes se loguean en `QHST` si el runtime está disponible.

### 10.3 Testing comprehensivo

- [ ] Ampliar `phase6_tui_smoke.rs` con:
  - Flujo WRKLIB → crear library → ObjectBrowser en esa library.
  - Flujo STRSEU → F14 compile → ver resultado.
  - Flujo SBMJOB → WRKSPLF → visualizar spool.
  - Flujo WRKUSRPRF → crear perfil → DSPOBJAUT → grant.
- [ ] Tests de regresión de layout: snapshot de render a 80×24 comparado
  con golden files.
- [ ] Test de navigation stack: 5 niveles de profundidad, F12 back x5 vuelve
  al origen.
- [ ] Benchmark de startup: la TUI debe arrancar en <500ms sin runtime.

### 10.4 PWRDWNSYS desde TUI

- [x] Comando `PWRDWNSYS` accesible desde menú (opción 90) y command line.
- [x] Confirmación obligatoria con `ConfirmDialog`.
- [x] Requiere usuario con autoridad `*ALLOBJ` o `QSECOFR`.
- [x] `L400_PWRDWNSYS_DRY_RUN=1` para smoke seguro.

Criterio de cierre:

- [ ] Todos los tests de smoke pasan.
- [ ] Render a 80×24 no tiene overflow visual.
- [x] PWRDWNSYS funciona con dry-run desde TUI.

---

## Backlog deliberado

No bloquea este plan:

- Emulación 5250 real (protocolo de red).
- Compatibilidad binaria IBM i.
- EBCDIC completo.
- Multi-sesión simultánea (screen splitting).
- Temas de color configurables (nice-to-have futuro).
- Editor SEU con columnas de secuencia (nice-to-have).

---

## Orden recomendado de PRs

1. Widget library: `InputField`, `SubfileTable`, `ConfirmDialog`.
2. Navigation stack + refactor `tokenize_cl_command`.
3. `StatusBar` global + `ModeIndicator`.
4. Sign-on con widgets nuevos + banner de arranque.
5. `WRKLIB` dedicado.
6. `ObjectBrowser` migrado a `SubfileTable` + filtros.
7. `DSPOBJD` con layout de campos.
8. SEU con números de línea + Find.
9. F14 compilación integrada en PDM/SEU.
10. STRSQL con highlight y autocompletado.
11. WRKACTJOB con auto-refresh + WRKJOB dedicado.
12. WRKSPLF con visor de spool completo.
13. WRKUSRPRF + EDTOBJAUT interactivo.
14. DSPPFM con headers de schema + scroll horizontal.
15. DSPLOG visual + DSPPOLICY mejorado.
16. PWRDWNSYS desde TUI.
17. Testing comprehensivo + golden snapshots.

---

## Gates permanentes

Gate rápido local:

```bash
cargo fmt --all --check
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
```

Gate de calidad antes de cerrar una fase:

```bash
cargo clippy -p l400 --all-targets -- -D warnings
cargo clippy -p os400-tui --all-targets -- -D warnings
./scripts/test/test_release_rc.sh
```

Gate de release candidate:

```bash
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

Smoke seguro para apagado/reinicio:

```bash
L400_PWRDWNSYS_DRY_RUN=1 cargo run -p l400 --bin l400cmd -- \
  PWRDWNSYS 'OPTION(*RESTART)' 'CONFIRM(*YES)'
```
