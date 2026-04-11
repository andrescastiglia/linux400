# Análisis y Plan de Ejecución: Fase 6 — Cargas de Trabajo (Cgroups v2)

## 1. Contexto y Objetivos

OS/400 distingue entre dos tipos fundamentales de cargas de trabajo:
- **QINTER (Interactive)**: Trabajos interactivos (sesiones de terminal, menús TUI, transacciones en línea)
- **QBATCH (Batch)**: Trabajos por lotes (procesamiento de colas, jobs programados, reportes nocturnos)

Linux/400 emula esta separación usando **cgroups v2** (Control Groups) para aislar recursos y garantizar que los trabajos interactivos no sean sofocados por procesos en lote.

## 2. Arquitectura de Cgroups en Linux/400

### 2.1 Jerarquía de Slices

```
/sys/fs/cgroup/
├── sys.slice/           # Sistema base
├── user.slice/          # Sesiones de usuario
├── l400.slice/          # Raíz Linux/400
│   ├── l400.qinter/    # Interactive workload (TUI, terminal, etc.)
│   └── l400.qbatch/    # Batch workload (DTAQ processors, etc.)
└── machine.slice/       # Containers/VMs
```

### 2.2 Parámetros de Cgroup por Workload

| Parámetro | QINTER (Interactive) | QBATCH (Batch) | Descripción |
|-----------|---------------------|----------------|-------------|
| `cpu.weight` | 10000 | 100 | Peso relativo de CPU |
| `cpu.max` | 100000 100000 | 10000 100000 | Burst y quota |
| `io.weight` | 100 | 50 | Peso de I/O |
| `memory.high` | 512M | 1G | Límite blando de memoria |
| `memory.max` | 1G | 4G | Límite duro de memoria |
| `pids.max` | 512 | 2048 | Máximo de procesos |

### 2.3 Integración con BPF LSM

El hook BPF de Fase 1 puede consultar el cgroup del proceso que intenta abrir un archivo para determinar si es un trabajo interactivo o batch, y aplicar políticas de acceso diferenciales si es necesario.

## 3. API Pública

### 3.1 Tipos de Workload

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkloadType {
    Interactive,  // QINTER - Terminal, TUI, menús
    Batch,        // QBATCH - DTAQ processors, jobs
}
```

### 3.2 Funciones Principales

```rust
pub fn create_l400_slices() -> Result<(), CgroupError>;
pub fn assign_to_workload(pid: u64, workload: WorkloadType) -> Result<(), CgroupError>;
pub fn get_current_workload() -> Result<WorkloadType, CgroupError>;
pub fn set_cpu_priority(workload: WorkloadType, weight: u64) -> Result<(), CgroupError>;
pub fn set_memory_limit(workload: WorkloadType, high: u64, max: u64) -> Result<(), CgroupError>;
```

### 3.3 Uso en Compiladores y Runtime

```rust
// En c400c o clc al compilar un programa interactivo
assign_to_workload(getpid(), WorkloadType::Interactive)?;

// Al iniciar un DTAQ processor
assign_to_workload(getpid(), WorkloadType::Batch)?;
```

## 4. Implementación

### 4.1 Estructura del Módulo (`libl400/src/cgroup.rs`)

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CgroupError {
    #[error("Cgroups v2 not available")]
    NotAvailable,
    #[error("Failed to create slice: {0}")]
    SliceCreation(String),
    #[error("Failed to assign process: {0}")]
    AssignmentFailed(String),
    #[error("Permission denied (requires root)")]
    PermissionDenied,
}
```

### 4.2 Rutas de Cgroup

```rust
const L400_CGROUP_PATH: &str = "/sys/fs/cgroup/l400.slice";
const QINTER_SLICE: &str = "l400.qinter";
const QBATCH_SLICE: &str = "l400.qbatch";
```

### 4.3 Creación de Slices

```rust
fn create_slice(name: &str, params: &CgroupParams) -> Result<PathBuf, CgroupError> {
    let slice_path = PathBuf::from(L400_CGROUP_PATH).join(name);
    std::fs::create_dir_all(&slice_path)?;
    
    // Escribir parámetros de cgroup v2
    write_cgroup_param(&slice_path, "cpu.weight", params.cpu_weight)?;
    write_cgroup_param(&slice_path, "cpu.max", params.cpu_max)?;
    write_cgroup_param(&slice_path, "io.weight", params.io_weight)?;
    write_cgroup_param(&slice_path, "memory.high", params.memory_high)?;
    write_cgroup_param(&slice_path, "memory.max", params.memory_max)?;
    
    Ok(slice_path)
}
```

### 4.4 Asignación de Procesos

```rust
fn assign_process_to_cgroup(pid: u64, slice_path: &Path) -> Result<(), CgroupError> {
    let tasks_file = slice_path.join("cgroup.threads");
    std::fs::write(tasks_file, pid.to_string())?;
    Ok(())
}
```

## 5. Verificación de Prerrequisitos

```rust
pub fn is_cgroup_v2_available() -> bool {
    PathBuf::from("/sys/fs/cgroup/cgroup.controllers").exists()
}
```

## 6. Tests

```bash
# Test de detección de cgroups v2
cargo test -p l400 cgroup::tests::test_cgroup_v2_available

# Test de creación de slices
cargo test -p l400 cgroup::tests::test_create_slices

# Test de round-trip de asignación
cargo test -p l400 cgroup::tests::test_assign_workload
```

## 7. Riesgos Asumidos

| Riesgo | Mitigación |
|--------|------------|
| cgroups v2 no disponible | Verificar `/sys/fs/cgroup/cgroup.controllers` antes de operar |
| Procesos que escapan al cgroup | Usar `cgroup.threads` (no `cgroup.procs`) para threads |
| Permisos insuficientes | Requerir root o membership en grupo `l400` |
| Interferencia con systemd | Usar `l400.slice` como raíz (no `/sys.slice`) |

## 8. Dependencias

- `rustix` (features: `fs`) para APIs de archivos
- Acceso a `/sys/fs/cgroup/` (normalmente root o grupo `systemd`)

## 9. Métricas de Éxito

- [x] `is_cgroup_v2_available()` retorna `true` en sistemas con cgroups v2
- [x] `create_l400_slices()` crea `l400.qinter` y `l400.qbatch` con parámetros correctos
- [x] `assign_to_workload()` mueve el proceso al cgroup correcto
- [x] `get_current_workload()` retorna el tipo de workload del proceso actual
- [x] Tests pasan en entorno sin cgroups v2 (fallback gracefully)
