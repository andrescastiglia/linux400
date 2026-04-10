# Análisis y Plan de Ejecución: Fase 4 — Compiladores Híbridos (Control Language y C/400)

Este documento describe el plan técnico y el estado de implementación de la **Fase 4** de Linux/400: la expansión de los compiladores nativos (`c400c` y `clc`) para que produzcan ejecutables (`*PGM`) registrados automáticamente dentro del sistema de almacenamiento orientado a objetos de Linux/400.

## 1. Objetivo y Diseño

En OS/400, la compilación de un programa no produce únicamente un ejecutable ELF: produce un **Objeto del Sistema** (`*PGM`) que vive en una Biblioteca (`*LIB`) bajo el Single-Level Storage. El sistema registra automáticamente sus metadatos, tipo, propietario y fecha de creación.

Linux/400 replica esta semántica en dos fases durante la compilación:

1. **Fase de compilación/enlazado** — Usar el compilador nativo del host (`clang` o `cc`) para generar el binario ELF vinculado con `libl400.so`.
2. **Fase de catalogación SLS** — Inmediatamente después de una compilación exitosa, llamar a `libl400::zfs::set_objtype(output_path, "*PGM")` para estampar el metadato ZFS que el BPF LSM reconocerá en tiempo de ejecución.

> [!IMPORTANT]
> **Estrategia de Compilación:** Ambos compiladores delegan la generación de código nativo al compilador C del host (`clang` o `cc`) mediante `std::process::Command`. Esta arquitectura de "shell-out" permite avanzar rápidamente sin reimplementar un backend LLVM completo en Rust, mientras que el valor diferencial de Linux/400 reside en la orquestación del tipado y la integración con ZFS.

---

## 2. Investigación Técnica y Restricciones

### 2.1 Compilador C/400 (`c400c`) — Shell-out a Clang

El compilador C/400 es un *front-end orquestador* que:
1. Valida que la ruta destino esté bajo `/l400/` (jerarquía ZFS protegida).
2. Invoca `clang <input.c> -o <output> -L<libl400_path> -ll400` para generar el ELF.
3. Llama a `set_objtype(output, "*PGM")` para catalogar el resultado.

```
   c400c --input programa.c --output /l400/QGPL/MIPGM
         │
         ├─► (1) Validar path bajo /l400/
         ├─► (2) clang programa.c -o /l400/QGPL/MIPGM -ll400
         └─► (3) set_objtype("/l400/QGPL/MIPGM", "*PGM")
```

### 2.2 Compilador Control Language (`clc`) — LLVM vía Inkwell + Shell-out

`clc` tiene una arquitectura más compleja:
1. **Parser Pest** — Parsea el fuente `.clp` con gramática CL definida en `cl.pest`.
2. **AST → IR** — El visitante de AST emite IR LLVM mediante `inkwell`.
3. **Emisión de objeto** — LLVM emite el `.o` intermedio.
4. **Linking** — `cc` enlaza el `.o` con `libl400`.
5. **Catalogación** — `set_objtype(output, "*PGM")` estampa el metadato ZFS.

> [!WARNING]
> **Dependencia de LLVM en el host:** `inkwell` (a través de `llvm-sys`) requiere una versión exacta de LLVM instalada estáticamente o con `libPolly.a` disponible. En el host de desarrollo (LLVM 20.1.8), la librería `libPolly` solo existe como `.so` (dinámica), lo que causa un error de enlazado estático en `llvm-sys`. La solución es usar el feature `no-llvm-linking` de `llvm-sys` o proporcionar `LLVM_SYS_201_PREFIX` apuntando a una instalación con librerías estáticas. En Docker (Ubuntu 26.04 con LLVM 21 completo), este problema no existe.

### 2.3 Integración de `libl400` en los Compiladores

Ambos compiladores referencian `libl400` como dependencia de workspace:

```toml
# c400_compiler/Cargo.toml
libl400 = { path = "../libl400" }

# cl_compiler/clc/Cargo.toml
libl400 = { path = "../../libl400" }
```

La función `set_objtype` de `libl400::zfs` es el único punto de contacto: los compiladores no necesitan conocer los módulos `db`, `dtaq` ni `object` en esta fase.

---

## 3. Componentes Afectados

| Archivo | Estado | Cambio |
|:--------|:-------|:-------|
| `c400_compiler/Cargo.toml` | ✅ Modificado | Agregado `clap` y `libl400` |
| `c400_compiler/src/main.rs` | ✅ Modificado | CLI completa + shell-out a `clang` + `set_objtype` |
| `cl_compiler/clc/Cargo.toml` | ✅ Modificado | Agregado `libl400`, ajuste de feature LLVM a `llvm20-1` |
| `cl_compiler/clc/src/main.rs` | ✅ Modificado | Paso 3 de catalogación ZFS tras linking |

---

## 4. Work Breakdown Structure (Checklist de Implementación)

### Sprint 4.1: Compilador C/400 (`c400_compiler`)

- [x] **(1) Actualizar `c400_compiler/Cargo.toml`:**
   - `clap = { version = "4.4", features = ["derive"] }` para CLI.
   - `libl400 = { path = "../libl400" }` para catalogación ZFS.

- [x] **(2) Reemplazar stub en `c400_compiler/src/main.rs`:**
   - Struct `Args` con campos `--input` y `--output`.
   - Validación de path (`/l400/` warning si fuera del pool).
   - Shell-out a `clang` con flags `-L` y `-ll400`.
   - Llamada a `set_objtype(output_path, "*PGM")` tras éxito del linker.

- [x] **(3) `cargo build -p c400c`:** Compiló exitosamente.

### Sprint 4.2: Compilador Control Language (`cl_compiler/clc`)

- [x] **(4) Actualizar `cl_compiler/clc/Cargo.toml`:**
   - Corrección de feature LLVM: `llvm21-1` → `llvm20-1` (host usa LLVM 20.1.8).
   - Agregar `libl400 = { path = "../../libl400" }`.

- [x] **(5) Modificar `cl_compiler/clc/src/main.rs`:**
   - Agregar imports: `use libl400::zfs::set_objtype` y `use std::path::Path`.
   - Paso 3 post-linking: validación de path + `set_objtype("*PGM")`.
   - Mensaje de error explícito y `exit(1)` si `set_objtype` falla.

- [x] **(6) Resolver bloqueo de `libPolly` en host:**
   - **Diagnóstico:** `llvm-sys` busca `libPolly.a` pero el host solo tiene `LLVMPolly.so`.
   - **Solución implementada:** Uso de la feature `no-llvm-linking` en `llvm-sys` + `build.rs` personalizado para enlazado dinámico con `libLLVM-20.so`.

### Sprint 4.3: Pruebas y Validación

- [x] **(7) Test de integración `c400c`:**
   - Crear `tests/hola_mundo.c` con un programa trivial que llame a `libl400::init()`.
   - Ejecutar `c400c --input tests/hola_mundo.c --output /l400/QGPL/HOLAMUNDO`.
   - Verificar: `getfattr -n user.l400.objtype /l400/QGPL/HOLAMUNDO` → `*PGM`.

- [x] **(8) Test de integración `clc`:** (requiere resolución de Polly)
   - Crear un script CL de prueba mínimal.
   - Compilar con `clc --input prueba.clp --output /l400/QGPL/PRUEBA`.
   - Verificar catalogación ZFS del resultado.

---

## 5. Criterios de Aceptación

| # | Criterio | Estado |
|:--|:---------|:-------|
| 1 | `c400c --input x.c --output /l400/LIB/PGM` produce un ELF con xattr `*PGM` | ✅ Completado |
| 2 | Si la compilación C falla, no se crea el xattr | ✅ Garantizado por diseño |
| 3 | `c400c` emite `[WARN]` si el destino no está bajo `/l400/` | ✅ Implementado |
| 4 | `clc` etiqueta el ejecutable generado como `*PGM` | ✅ Implementado |
| 5 | `clc` aborta con `exit(1)` si `set_objtype` falla tras linking exitoso | ✅ Implementado |
| 6 | Ambos compiladores forman parte del workspace Rust | ✅ `c400c` + `cl_compiler/clc` en `Cargo.toml` |

---

## 6. Resolución del Bloqueo libPolly
 
 El bloqueo de `libPolly` estático en el host se resolvió mediante **enlazado dinámico manual**. Se configuró `clc` para usar la feature `no-llvm-linking` de `llvm-sys` y un script `build.rs` que detecta la librería compartida `libLLVM-20.so` mediante `llvm-config`.
 
 Esta solución permite compilar con el backend LLVM habilitado (`--features llvm-backend`) sin requerir cambios en el sistema del host.
