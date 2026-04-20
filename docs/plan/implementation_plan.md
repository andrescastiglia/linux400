# Plan de Implementación: STRPDM, STRSEU, STRSQL y WRKMBRPDM

## Objetivo

Implementar los cuatro entornos interactivos del cheatsheet como pantallas TUI reales dentro de `os400-tui`, siguiendo el patrón existente (`Screen` trait + `ScreenId`):

- **STRPDM** — Programming Development Manager: navegador de objetos de desarrollo (bibliotecas → archivos fuente → miembros).
- **STRSEU** — Source Entry Utility: editor de texto minimalista para miembros de archivos fuente.
- **STRSQL** — Interactive SQL: intérprete de sentencias SQL sobre archivos `*FILE` PF/LF del runtime.
- **WRKMBRPDM** — Work with Members in PDM: lista y gestiona miembros dentro de un archivo fuente específico.

Estos cuatro comandos también deben emitir código real en el codegen de `clc` (en lugar del mensaje de "no disponible en modo batch").

---

## Análisis del patrón existente

Cada pantalla sigue este contrato:

```
os400-tui/src/screens/
  ├── mod.rs           ← ScreenId enum + Screen trait
  ├── main_menu.rs     ← ejemplo de pantalla con estado + navegación
  ├── object_browser.rs ← ejemplo de tabla con TableState
  └── work_mgmt.rs     ← ejemplo de datos en vivo del runtime
```

**Para agregar una pantalla nueva se necesita:**
1. Crear `os400-tui/src/screens/<nombre>.rs` implementando `Screen`.
2. Agregar la variante en `ScreenId`.
3. Registrar el módulo en `mod.rs`.
4. Agregar la construcción en `main.rs` (switch de pantallas activas).
5. Conectar la navegación desde donde corresponda (menú, `CommandLine`, etc.).

---

## Propuesta de flujo de navegación

```
MainMenu
  └─ [7] STRPDM ──→ PdmBrowser (lista bibliotecas)
                        └─ Enter sobre lib ──→ WrkMbrPdm (lista archivos fuente)
                                                   └─ F15/Enter ──→ Strseu (editor de miembro)
                                                   └─ F16       ──→ Strsql (SQL interactivo)
CommandLine
  ├─ STRPDM  ──→ PdmBrowser
  ├─ STRSEU  ──→ Strseu  (requiere parámetro FILE/MBR)
  ├─ STRSQL  ──→ Strsql
  └─ WRKMBRPDM → WrkMbrPdm (requiere FILE)
```

---

## Trabajo por componente

---

### Componente 1 — Routing y ScreenId

#### [MODIFY] `os400-tui/src/screens/mod.rs`

Agregar variantes al enum `ScreenId`:

```rust
PdmBrowser,    // STRPDM
WrkMbrPdm,     // WRKMBRPDM
StrSeu,        // STRSEU
StrSql,        // STRSQL
```

#### [MODIFY] `os400-tui/src/main.rs`

Instanciar las nuevas pantallas en el dispatcher de `ScreenId`.

#### [MODIFY] `os400-tui/src/screens/main_menu.rs`

- Agregar opción `7` al menú principal: `"Programming Development Manager"` → `STRPDM`.
- Conectar `handle_option("7")` → `ScreenId::PdmBrowser`.

#### [MODIFY] `os400-tui/src/screens/cmd_line.rs`

Reconocer los nuevos comandos en la línea de comandos: `STRPDM`, `STRSEU`, `STRSQL`, `WRKMBRPDM`.

---

### Componente 2 — STRPDM (`pdm_browser.rs`)

#### [NEW] `os400-tui/src/screens/pdm_browser.rs`

**Descripción:** Lista las bibliotecas catalogadas en el root de L400. Permite al usuario seleccionar una para navegar sus archivos fuente con `WRKMBRPDM`.

**Estado interno:**
```rust
pub struct PdmBrowser {
    libraries: Vec<String>,
    state: ListState,
}
```

**Comportamiento:**
- Al cargar: llama `list_objects(resolve_l400_root())` y filtra tipo `*LIB`.
- `Enter` sobre una biblioteca → `ScreenResult::goto(ScreenId::WrkMbrPdm)` pasando el nombre de la lib en `data`.
- `F5` refresca la lista.
- `F3`/`F12` → `ScreenId::MainMenu`.

**Layout:**
```
╔═ STRPDM - Programming Development Manager ══════════════╗
║  Select library and press Enter. F5=Refresh              ║
╠══════════════════════════════════════════════════════════╣
║  > QGPL                                                  ║
║    MYLIB                                                  ║
║    QSYS                                                   ║
╠══════════════════════════════════════════════════════════╣
║ F3=Exit  F5=Refresh  F12=Cancel  Enter=Select            ║
╚══════════════════════════════════════════════════════════╝
```

---

### Componente 3 — WRKMBRPDM (`wrk_mbr_pdm.rs`)

#### [NEW] `os400-tui/src/screens/wrk_mbr_pdm.rs`

**Descripción:** Lista los miembros de un archivo fuente específico dentro de una biblioteca. Permite editar un miembro con SEU (F15) o entrar a SQL (F16).

**Estado interno:**
```rust
pub struct WrkMbrPdm {
    library: String,
    file: String,
    members: Vec<MemberInfo>,
    state: TableState,
}

pub struct MemberInfo {
    pub name: String,
    pub type_: String,     // CLP, RPGLE, SQLRPGLE, etc.
    pub text: String,
}
```

**Comportamiento:**
- Al cargar: llama `list_members(lib_path, file_name)` (nueva función en `libl400/src/db.rs` o `object.rs`).
- `Enter`/`F15` sobre un miembro → `ScreenId::StrSeu` con `data = "LIB/FILE/MBR"`.
- `F16` → `ScreenId::StrSql`.
- `F3`/`F12` → `ScreenId::PdmBrowser`.
- `F6` → crear nuevo miembro (interacción con prompt inline o mediante `CommandLine`).

**Layout:**
```
╔═ WRKMBRPDM - Work with Members ═══════════════════════════╗
║  File: QGPL/QCLSRC    F5=Refresh  F6=Create  F16=STRSQL   ║
╠═══════════════════════════════════════════════════════════╣
║  Mbr         Type      Text                               ║
║  ──────────────────────────────────────────────────────   ║
║  HELLO       CLP       Hello world program                ║
║  DEMO        CLP       Demo CL program                    ║
╠═══════════════════════════════════════════════════════════╣
║ F3=Exit  F5=Refresh  F6=Create  F15=Edit  F16=STRSQL      ║
╚═══════════════════════════════════════════════════════════╝
```

**Nueva función en libl400:** `list_members(lib: &Path, file: &str) -> Result<Vec<MemberInfo>, ObjectError>` — escanea el sub-directorio o la sled/BDB database correspondiente al archivo fuente.

---

### Componente 4 — STRSEU (`str_seu.rs`)

#### [NEW] `os400-tui/src/screens/str_seu.rs`

**Descripción:** Editor de texto minimalista estilo SEU para miembros CLP/RPGLE. Soporta edición básica de líneas, guardado con `F3`, y resalta números de línea al estilo OS/400.

**Estado interno:**
```rust
pub struct StrSeu {
    member_path: PathBuf,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_offset: usize,
    modified: bool,
}
```

**Comportamiento:**
- Al cargar: lee el contenido del miembro desde disco (o crea vacío si es nuevo).
- Edición de texto con teclas de movimiento y escritura.
- `F3` → guarda y vuelve a `ScreenId::WrkMbrPdm`.
- `F12` → descarta cambios y vuelve.
- `F5` → recarga desde disco.
- Números de línea al margen izquierdo (formato OS/400: `0001.00`).

**Layout:**
```
╔═ STRSEU - Source Entry Utility ══ QGPL/QCLSRC/HELLO.CLP ╗
║  Columns 1-72        Browse/Copy Mode  F3=Save  F12=Exit  ║
╠═══════════════════════════════════════════════════════════╣
║ 0001.00 PGM                                               ║
║ 0002.00     SNDPGMMSG MSG('Hello from L400!')             ║
║ 0003.00 ENDPGM                                            ║
║_                                                          ║
╠═══════════════════════════════════════════════════════════╣
║ F3=Save  F5=Reload  F12=Cancel                            ║
╚═══════════════════════════════════════════════════════════╝
```

---

### Componente 5 — STRSQL (`str_sql.rs`)

#### [NEW] `os400-tui/src/screens/str_sql.rs`

**Descripción:** Intérprete SQL interactivo que ejecuta sentencias sobre los archivos PF/LF catalogados en `libl400`. Usa el backend Berkeley DB existente.

**Estado interno:**
```rust
pub struct StrSql {
    input: String,
    history: Vec<String>,
    results: Vec<Vec<String>>,
    columns: Vec<String>,
    error: Option<String>,
    scroll: usize,
}
```

**Comportamiento:**
- El usuario escribe una sentencia SQL en el área de entrada inferior.
- `Enter` ejecuta la consulta contra `libl400::db`.
- Los resultados se muestran en una tabla ratatui con `TableState`.
- `F3`/`F12` → vuelve al origen (PdmBrowser o MainMenu).
- `F5` limpia los resultados.
- Soporta inicialmente: `SELECT * FROM <file>`, `SELECT <cols> FROM <file> WHERE <cond>`.

**Layout:**
```
╔═ STRSQL - Interactive SQL ════════════════════════════════╗
║  Type SQL statement and press Enter.                       ║
╠═══════════════════════════════════════════════════════════╣
║  COL1       COL2       COL3                               ║
║  ──────────────────────────────────────────────────────   ║
║  AAAAA      00001      Lorem ipsum                        ║
║  BBBBB      00002      Dolor sit amet                     ║
╠═══════════════════════════════════════════════════════════╣
║ SQL> SELECT * FROM QGPL/MYFILE_                           ║
╠═══════════════════════════════════════════════════════════╣
║ F3=Exit  F5=Clear  F12=Cancel                             ║
╚═══════════════════════════════════════════════════════════╝
```

**Parseo SQL mínimo:** regex o split básico para `SELECT`, `FROM`, `WHERE`. No se requiere un parser SQL completo; el backend real de consultas ya está en `libl400::db::PhysicalFile::read_all / LogicalFile`.

---

### Componente 6 — codegen `clc` para los nuevos comandos

#### [MODIFY] `cl_compiler/clc/src/compiler.rs`

Reemplazar el bloque de fallback "no disponible en modo batch" por llamadas reales:

```c
"STRPDM"    → l400_strpdm();
"STRSEU"    → l400_strseu(FILE, MBR);
"STRSQL"    → l400_strsql();
"WRKMBRPDM" → l400_wrkmbrpdm(FILE);
```

#### [MODIFY] `libl400/src/ffi_commands.rs`

Agregar:
- `l400_strpdm()` → imprime lista de bibliotecas (modo batch).
- `l400_strseu(file, mbr)` → imprime contenido del miembro.
- `l400_strsql()` → modo batch: acepta SQL de stdin.
- `l400_wrkmbrpdm(file)` → lista miembros del archivo.

---

## Criterio de aceptación

- Las cuatro pantallas TUI se navegan desde el menú principal (`7=STRPDM`) y desde `CommandLine`.
- `STRSEU` permite editar y guardar un miembro `.clp` que luego puede compilarse con `clc`.
- `STRSQL` ejecuta `SELECT * FROM <file>` y muestra resultados en tabla.
- `WRKMBRPDM` lista miembros y permite abrir `STRSEU` sobre ellos.
- Los tests de `cargo test -p os400-tui` y `cargo test -p clc` pasan.

---

## Notas

- `STRSEU` no necesita ser un editor vi/emacs. Con soporte de `Insert`/`Delete` char, cursor libre y guardado F3 es suficiente para la primera iteración.
- `STRSQL` inicialmente sólo soporta `SELECT`. `INSERT`/`UPDATE`/`DELETE` vienen en una iteración posterior.
- Los miembros de archivos fuente se almacenan como archivos planos dentro del directorio del objeto `*FILE` (o como entradas nombradas en la sled/BDB del PF). El naming convention es `<MEMBER>` dentro de `<LIB>/<FILE>/`.
