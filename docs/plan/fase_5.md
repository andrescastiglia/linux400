# Análisis y Plan de Ejecución: Fase 5 — Memory Tagging de 64-bits (TBI / Intel LAM)

## 1. Contexto y Objetivos

OS/400 emplea direcciones de 48 bits con punteros "espaciales" (space pointers) donde los 16 bits superiores codifican información de control. Linux/400 replica esta semántica usando **Top-Byte Ignore (TBI)** en ARM64 o **Linear Address Masking (LAM)** en x86-64, permitiendo que los 16 bits superiores de un puntero de 64 bits contengan metadatos de objeto (`*PGM`, `*FILE`, etc.).

El objetivo de Fase 5 es:
1. Detectar en runtime si el hardware soporta LAM (Intel Sapphire Rapids+) o TBI (ARM64)
2. Configurar `arch_prctl` para habilitar el modo de direcciones virtualizadas
3. Proveer un **fallback de software seguro** usando enmascaramiento bitwise para CPUs sin soporte hardware

## 2. Arquitectura de Memory Tagging en Linux/400

### 2.1 Concepto de Space Pointers

En OS/400, un puntero de dirección de almacenamiento único (SSA) contiene:
- **Bits 0-63**: Dirección lineal
- **Bits 64-79**: Espacio/selección de segmento (16 bits superiores)

Linux/400 mapea esto a:
```
┌────────────────────────────────────┬──────────────┐
│  Upper 16 bits (metadata/space)   │ Lower 48 bits│
│  bits [63:48]                      │ bits [47:0]  │
└────────────────────────────────────┴──────────────┘
```

### 2.2 Modos de Addressing

| Modo | Plataforma | Descripción |
|------|------------|-------------|
| **LAM (Linear Address Masking)** | Intel x86-64 (Sapphire Rapids+) | Máscara bits superiores configurable via `ARCH_[SET\|GET]_LAM` |
| **TBI (Top Byte Ignore)** | ARM64 | Bits [63:56] ignorados en address generation |
| **SW Mask** | Genérico | `ptr & 0x0000_FFFF_FFFF_FFFF` antes de dereferenciar |

### 2.3 Estados de LAM

```
000000: LAM disabled (uso normal, bits [63:62] = 00)
01xxxx: LAM48 con U57=1 (48-bit addressing + upper bits as metadata)
10xxxx: LAM57 (57-bit addressing + upper bits as metadata)  
11xxxx: Reserved
```

Para Linux/400, usamos **LAM48** (模式 `01xxxx`) para mantener compatibilidad con punteros de 48 bits del modelo OS/400.

## 3. Implementación

### 3.1 Detección de Capacidad Hardware

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryTaggingMode {
    /// Intel LAM48 (Sapphire Rapids+)
    IntelLam48,
    /// ARM64 TBI
    ArmTbi,
    /// Fallback software (enmascaramiento bitwise)
    SoftwareMask,
    /// No disponible
    Unsupported,
}
```

### 3.2 API Pública (`libl400/src/lam.rs`)

```rust
pub fn detect_hardware_mode() -> MemoryTaggingMode;
pub fn enable_lam48() -> Result<(), LamError>;
pub fn enable_tbi() -> Result<(), LamError>;
pub fn tag_pointer<T>(ptr: *const T, space: u16) -> *const T;
pub fn untag_pointer<T>(ptr: *const T) -> *const T;
pub fn is_tagged_pointer<T>(ptr: *const T) -> bool;
```

### 3.3 Criterios de Selección

```
1. Si CPUID.(EAX=7,ECX=2).EDX[28] == 1 → Intel LAM disponible
   → Verificar modelo >= Sapphire Rapids (Family 6, Model > 0xAF)
   → Si cumple, usar IntelLam48

2. Si target_arch == "aarch64" → ARM TBI siempre disponible (desde ARMv8.0-A)

3. Caso contrario → SoftwareMask (bitwise AND 0x0000_FFFF_FFFF_FFFF)
```

### 3.4 Integración con libl400

Los módulos que usan punteros espaciales deben invocar `untag_pointer()` **antes** de dereferenciar:

```rust
// En object.rs, db.rs, dtaq.rs
let tagged_ptr = ...;
let addr = untag_pointer(tagged_ptr);
unsafe { std::ptr::read(addr) }
```

### 3.5 Inicialización

La habilitación de LAM/TBI ocurre en:
1. `libl400::init()` — llamado automáticamente via `#[no_mangle] extern "C" fn init()`
2. O explícitamente via `lam::enable_for_platform()`

## 4. API Linux (arch_prctl)

```c
#include <asm/prctl.h>

// Habilitar LAM48
arch_prctl(ARCH_SET_LAM, ARCH_LAM_U57_NOT_TRACKED);

// Consultar capability
arch_prctl(ARCH_GET_LAM, &lam_bits);
```

En Rust, usar `rustix::process::arch_prctl` (requiere feature `process`).

## 5. Tests

```bash
# Test de detección de hardware
cargo test -p l400 lam::tests

# Test de round-trip tag/untag
cargo test -p l400 test_tag_pointer

# Test de integración con objetos
cargo test -p l400 test_object_tagging
```

## 6. Riesgos Asumidos

| Riesgo | Mitigación |
|--------|------------|
| CPU sin LAM (pre-Sapphire Rapids) | Fallback a SoftwareMask con zero overhead en dereference |
| Compatibilidad con kernel antiguo | SoftwareMask funciona en cualquier kernel >= 4.x |
| Conteo de referencias en punteros tagged | Usar `Arc<...>` + `untag_pointer()` antes de clone |

## 7. Dependencias

- `rustix` (features: `process`, `fs`) para `arch_prctl` y `O_DIRECT`
- No se requieren crates adicionales

## 8. Métricas de Éxito

- [x] `detect_hardware_mode()` retorna modo correcto según CPU
- [x] `enable_lam48()` configura `arch_prctl` sin errores en HW soportado
- [x] `tag_pointer()` / `untag_pointer()` son inversas (round-trip)
- [x] `SoftwareMask` produce resultados idénticos a LAM hardware
- [x] Tests pasan en entorno sin LAM (fallback correcto)
