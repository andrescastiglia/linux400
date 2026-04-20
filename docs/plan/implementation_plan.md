# Plan de Implementación: Comandos CL del Cheatsheet

## Objetivo

Implementar los comandos del cheatsheet de OS/400 (`docs/cheetsheet.md`) como programas CL reales (`*.clp`), compilables con `clc` y ejecutables como objetos `*PGM` de Linux/400.

Esto implica extender el compilador CL en tres frentes:

1. **Gramática y AST**: soportar la sintaxis completa de los comandos del cheatsheet.
2. **Codegen C**: generar código C real por cada comando (en lugar de emitir `l400_sndpgmmsg` genérico de fallback).
3. **Runtime `libl400`**: exponer las funciones C que implementan la semántica de cada comando.

---

## Análisis del estado actual

El compilador `clc` ya:
- Parsea un subset básico de CL con gramática Pest.
- Genera código C vía backend (`generate_c_backend`).
- Sólo reconoce `PGM`, `ENDPGM` y `SNDPGMMSG` con semántica real. Todo lo demás emite un fallback de warning.
- Enlaza contra `libl400` y estampa el binario resultante como `*PGM` en ZFS.

**El gap principal** es que los comandos del cheatsheet no tienen representación en `codegen.rs` ni en `libl400`.

---

## Comandos a implementar

| Comando       | Categoría              | Acción                                               |
|---------------|------------------------|------------------------------------------------------|
| `WRKSYSSTS`   | Gestión de sistema     | Muestra CPU, ASP y jobs activos                      |
| `WRKACTJOB`   | Gestión de jobs        | Lista jobs activos del job registry                  |
| `WRKSYSVAL`   | Valores de sistema     | Muestra/modifica valores de configuración            |
| `DSPLOG`      | Logs                   | Muestra mensajes del log del sistema                 |
| `WRKUSRPRF`   | Perfiles               | Gestiona `*USRPRF` (listar, crear, eliminar)         |
| `PWRDWNSYS`   | Control sistema        | Apaga o reinicia el sistema                          |
| `WRKOBJ`      | Objetos                | Busca y lista objetos del catálogo                   |
| `CRTLIB`      | Bibliotecas            | Crea una biblioteca (`*LIB`)                         |
| `DLTLIB`      | Bibliotecas            | Elimina una biblioteca                               |
| `ADDLIBLE`    | Library list           | Añade biblioteca a la lista de búsqueda del usuario  |
| `CHGCURLIB`   | Library list           | Cambia la biblioteca actual de trabajo               |
| `RNMOBJ`      | Objetos                | Renombra un objeto                                   |
| `CRTPGM`      | Programación           | Crea/registra un objeto `*PGM`                       |
| `GO`          | Navegación             | Navega a un menú (ej. `GO MAIN`)                     |
| `SIGNOFF`     | Sesión                 | Cierra la sesión activa                              |

---

## Orden de implementación

### Fase A — Fundaciones del codegen (prerequisito)

Sin esto los demás comandos no pueden generarse correctamente.

#### [MODIFY] `cl_compiler/clc/src/grammar.pest`

La gramática actual no soporta:
- Parámetros con múltiples valores anidados (`ADDLIBLE LIB(*LIBL)`)
- Comandos en dos palabras con espacio (`GO MAIN`)
- Comentarios (`/* ... */`)

Cambios:
- Agregar soporte para `/* comentarios */` ignorados.
- Extender `command` para aceptar `GO MAIN` como caso de dos tokens.

#### [MODIFY] `cl_compiler/clc/src/ast.rs`

Agregar variante `GoCommand { target: String }` o simplemente normalizar `GO MAIN` como un comando de nombre `GO` con parámetro posicional `MAIN`.

#### [MODIFY] `cl_compiler/clc/src/compiler.rs`

Extender `generate_c_backend` para que en lugar del fallback genérico emita llamadas a funciones específicas por comando:

```c
l400_wrkactjob();
l400_wrksyssts();
l400_crtlib("MYLIB");
l400_dltlib("MYLIB");
// etc.
```

---

### Fase B — Runtime `libl400`: funciones C públicas

#### [NEW] `libl400/src/ffi_commands.rs`

Módulo que expone como `extern "C"` las funciones que el código C generado por `clc` puede invocar:

```rust
#[no_mangle]
pub extern "C" fn l400_wrksyssts() { ... }

#[no_mangle]
pub extern "C" fn l400_wrkactjob() { ... }

#[no_mangle]
pub extern "C" fn l400_crtlib(name: *const c_char) { ... }
// etc.
```

Internamente cada función delegará a los módulos ya existentes en `libl400`:
- `l400_wrkactjob` → `cgroup::list_jobs_at`
- `l400_crtlib` → `object::create_library`
- `l400_dltlib` → `object::delete_object`
- `l400_wrkusrprf` → `usrprf::create_user_profile` / lista
- `l400_wrkobj` → `object::list_objects`
- `l400_rnmobj` → `fs::rename` + restablece xattrs
- `l400_crtpgm` → `object::catalog_object`
- `l400_signoff` → `std::process::exit(0)`
- `l400_pwrdwnsys` → `Command::new("shutdown")...`
- `l400_go_main` → emite un mensaje especial / no-op en runtime sin TUI

#### [MODIFY] `libl400/src/lib.rs`
- Exportar `pub mod ffi_commands`.

---

### Fase C — Programas CL de ejemplo (`.clp`)

Crear los programas CL que ejerciten cada comando del cheatsheet. Irán en un directorio dedicado.

#### [NEW] `examples/cl/` (directorio)

| Archivo                    | Contenido CL                    |
|----------------------------|---------------------------------|
| `wrkactjob.clp`            | `PGM` / `WRKACTJOB` / `ENDPGM` |
| `wrksyssts.clp`            | `PGM` / `WRKSYSSTS` / `ENDPGM` |
| `crtlib_demo.clp`          | `PGM` / `CRTLIB LIB(MYLIB)` / `ENDPGM` |
| `dltlib_demo.clp`          | `PGM` / `DLTLIB LIB(MYLIB)` / `ENDPGM` |
| `wrkobj_demo.clp`          | `PGM` / `WRKOBJ OBJ(*ALL)` / `ENDPGM` |
| `wrkusrprf_demo.clp`       | `PGM` / `WRKUSRPRF USRPRF(*ALL)` / `ENDPGM` |
| `signoff_demo.clp`         | `PGM` / `SIGNOFF` / `ENDPGM` |

---

### Fase D — Script de compilación end-to-end

#### [NEW] `scripts/compile_cheatsheet.sh`

Script que itera sobre `examples/cl/*.clp` y:
1. Invoca `clc -i <archivo>.clp -o /tmp/l400/<nombre>`
2. Verifica que el binario fue catalogado como `*PGM`
3. Reporta éxito o error por comando

---

## Criterio de aceptación

- `clc` compila cada `.clp` del directorio `examples/cl/` sin error.
- El binario resultante tiene el xattr `user.l400.objtype=*PGM` estampado.
- Ejecutar el binario produce la salida correcta (ej. listar jobs, crear biblioteca).
- Los tests de `cargo test -p clc` cubren el codegen de los nuevos comandos.

---

## Notas arquitectónicas

- Los comandos interactivos (`WRKSYSSTS`, `WRKACTJOB`, `STRPDM`, etc.) que en OS/400 real abren una pantalla TUI, en esta iteración **imprimirán la información en stdout** (modo batch/non-interactive). La integración TUI viene después.
- `GO MAIN`, `F4`, `F10`, `F11` son comandos de sesión interactiva que **no tienen representación en CL compilado**; no se incluirán en el codegen, sino que se manejarán a nivel de la TUI directamente.
- `STRPDM`, `STRSEU`, `STRSQL`, `WRKMBRPDM` son entornos completos (editores/SQL); en esta fase sólo se registran como comandos reconocidos que emiten un mensaje de "no disponible en modo batch".
