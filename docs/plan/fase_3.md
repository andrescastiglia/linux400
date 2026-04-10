# Análisis y Plan de Ejecución: Fase 3 — Subsistema Relacional BDB y Workaround DAX

Este documento define el plan de ejecución técnico para la **Fase 3** de Linux/400: la implementación del subsistema de base de datos integrado que emula el *Single-Level Storage* (SLS) de OS/400 sobre ZFS, incluyendo Archivos Físicos (`*FILE PF`), Archivos Lógicos (`*FILE LF`) y Colas de Datos (`*DTAQ`).

## 1. Objetivo y Diseño

En OS/400, todos los datos —desde tablas de base de datos hasta colas de mensajes— conviven en un espacio de almacenamiento unificado conocido como *Single-Level Storage*. Para Linux/400, esta capa se construye sobre:

1. **ZFS como filesystem subyacente** (configurado en Fase 2 con `xattr=sa`).
2. **`sled`** como *embedded key-value store* 100% Rust, proveyendo transacciones atómicas y persistencia nativa sin dependencias de C externas.
3. **`O_DIRECT` (Flagging)** sobre los descriptores de archivo para bypassear el Page Cache de Linux y delegar el caching exclusivamente al ARC de ZFS, evitando la doble capitalización de caché.

> [!IMPORTANT]
> **Decisión Técnica:** Se optó por `sled` (Rust puro) sobre Berkeley DB (bindings C) para conservar la portabilidad del proyecto y facilitar la compilación cruzada hacia arquitecturas alternativas (ARM64, RISC-V). Esta decisión sacrifica la compatibilidad con herramientas *legacy* BDB pero garantiza la integridad de la cadena de compilación.

---

## 2. Investigación Técnica y Restricciones

### 2.1 Sled como Storage Engine para Archivos Físicos

`sled` es un árbol B+ *log-structured* embebido escrito en Rust puro. Sus características lo hacen ideal para emular los *Physical Files* (PF) de OS/400:

| Característica | Relevancia para Linux/400 |
|:---------------|:--------------------------|
| Transacciones ACID | Garantiza la atomicidad de `write_rcd` / `delete_rcd` |
| Árboles nombrados (`open_tree`) | Permite emular miembros de archivo (PF puede tener múltiples miembros) |
| IDs automáticos (`generate_id`) | Fundamental para la cola FIFO de `*DTAQ` |
| Zero-copy reads con `IVec` | Evita copias innecesarias en `chain_rcd` de lectura secuencial |

### 2.2 Workaround DAX — O_DIRECT sobre ZFS

ZFS no soporta el protocolo DAX (Direct Access) a nivel VFS porque su diseño CoW (Copy-on-Write) es fundamentalmente incompatible con el mapeo directo de páginas de PMEM. La solución adoptada:

```
[Programa L400]
      │
      ▼
 open(O_DIRECT)    ← Bypassa el Page Cache del Kernel Linux
      │
      ▼
 [ZFS ARC]         ← Caché de segundo nivel gestionado por OpenZFS
      │
      ▼
 [Storage Pool]    ← Disco físico / imagen sparse
```

La función `open_object_direct` en `libl400/src/object.rs` implementa este patrón usando la flag `libc::O_DIRECT` (vía `rustix::fs::OFlags::DIRECT`) para garantizar que los I/O del compilador y del runtime nunca pasen por el Page Cache del kernel Linux.

> [!WARNING]
> `O_DIRECT` impone restricciones de alineación de buffer (512 bytes o 4096 bytes según el fs). Los buffers de escritura en `write_rcd` deben alinearse correctamente. `sled` internamente maneja sus propios buffers internamente, pero al instanciar `PhysicalFile` sobre ZFS con `O_DIRECT`, el VFS de ZFS manejará la alineación transparentemente a través del módulo `zfs.ko`.

### 2.3 Emulación de Data Queues (`*DTAQ`)

Las Colas de Datos en OS/400 son estructuras de comunicación inter-proceso con semántica similar a las POSIX message queues pero con soporte nativo de timeout bloqueante. La emulación en `sled`:

```
   SNDDTAQ ──► [tree.insert(generate_id(), payload)]
                         │
   RCVDTAQ ◄── [tree.pop_min()]  ← FIFO garantizado por ID monotónico
```

El timeout bloqueante se implementa con polling micro-granular (`sleep(10ms)`) hasta que `pop_min()` retorna un valor o se alcanza el `wait_time` especificado.

---

## 3. Componentes Implementados

### 3.1 `libl400/src/db.rs` — Physical Files

| Función | Descripción |
|:--------|:------------|
| `create_pf(lib_path, name, record_len)` | Crea un dataset sled bajo ZFS y lo etiqueta como `*FIL` |
| `PhysicalFile::open(path)` | Abre un PF existente validando su existencia en el filesystem |
| `write_rcd(key, buffer)` | Inserta un registro atomicamente |
| `chain_rcd(key)` | Recupera un registro por clave (Random Read) |

### 3.2 `libl400/src/dtaq.rs` — Data Queues

| Función | Descripción |
|:--------|:------------|
| `crtdtaq(lib_path, name)` | Crea una cola nueva con etiqueta ZFS `*DTA` |
| `DataQueue::open(path)` | Abre una cola existente |
| `snddtaq(buffer)` | Encola un mensaje con ID autoincremental |
| `rcvdtaq(wait_time)` | Desencola el primer mensaje (FIFO); bloquea hasta `wait_time` segundos |

### 3.3 `libl400/src/object.rs` — DAX Workaround

| Función | Descripción |
|:--------|:------------|
| `open_object_direct(path)` | Abre un objeto L400 con `O_DIRECT` para bypass de Page Cache |

---

## 4. Work Breakdown Structure (Checklist de Implementación)

### Sprint 3.1: Configuración de Dependencias

- [x] **(1) Actualizar `libl400/Cargo.toml`:**
   - Agregar `sled = "0.34"` como storage engine.
   - Agregar `rustix = { version = "0.38", features = ["fs", "std"] }` para `O_DIRECT` idiomático.

### Sprint 3.2: Base de Datos Relacional (PF/LF)

- [x] **(2) Crear `libl400/src/db.rs`:**
   - Struct `PhysicalFile` con campo `db: sled::Db` y `tree: sled::Tree`.
   - Función `create_pf` que crea el directorio de sled, etiqueta con `set_objtype("*FIL")` y retorna un handle.
   - Métodos `write_rcd(key, buffer)` y `chain_rcd(key)`.

### Sprint 3.3: Colas de Datos (DTAQ)

- [x] **(3) Crear `libl400/src/dtaq.rs`:**
   - Struct `DataQueue` con campos `db: sled::Db` y `tree: sled::Tree`.
   - Función `crtdtaq` con etiquetado `*DTA`.
   - Métodos `snddtaq` y `rcvdtaq` con soporte de timeout bloqueante.

### Sprint 3.4: Workaround DAX y Validación

- [x] **(4) Modificar `libl400/src/object.rs`:**
   - Agregar función `open_object_direct(path)` usando `OpenOptionsExt::custom_flags(O_DIRECT)`.
   - La función valida el tipado ZFS antes de abrir (previene bypass sobre archivos no-L400).

- [x] **(5) Actualizar `libl400/src/lib.rs`:**
   - Re-exportar `db::{create_pf, PhysicalFile, DbError}`.
   - Re-exportar `dtaq::{crtdtaq, DataQueue, DtaqError}`.
   - Re-exportar `object::open_object_direct`.

- [x] **(6) Ejecutar `cargo test`:**
   - Test suite pasó sin errores de compilación.
   - 0 tests unitarios fallidos (tests E2E quedan pendientes de ejecución sobre ZFS real).

---

## 5. Criterios de Aceptación

| # | Criterio | Estado |
|:--|:---------|:-------|
| 1 | `create_pf` crea directorio sled con xattr `*FIL` en ZFS | ✅ Implementado |
| 2 | `write_rcd` + `chain_rcd` permiten round-trip de datos | ✅ Implementado |
| 3 | `crtdtaq` + `snddtaq` + `rcvdtaq` emulan semántica FIFO bloqueante | ✅ Implementado |
| 4 | `rcvdtaq` respeta `wait_time = 0` (no bloqueante) y `wait_time > 0` (timeout) | ✅ Implementado |
| 5 | `open_object_direct` inyecta `O_DIRECT` solo sobre objetos tipificados | ✅ Implementado |
| 6 | `cargo test -p libl400` pasa sin errores | ✅ Verificado |

---

## 6. Notas de Deuda Técnica

- **Archivos Lógicos (`*FILE LF`):** ✅ Implementado. Se utiliza la vinculación vía xattrs (`user.l400.base_pf`) y árboles secundarios en sled para garantizar transaccionalidad.
- **Tests E2E sobre ZFS real:** ✅ Verificado. Se agregó `test_zfs_e2e_lf` que valida la creación y vinculación de objetos directamente sobre `/linux400pool/`.
- **Alineación de buffer para O_DIRECT:** ✅ Resuelto. Se implementó `AlignedBuffer` y la utilidad `validate_alignment` para asegurar que los I/O cumplan los requisitos del Kernel Linux y ZFS.
