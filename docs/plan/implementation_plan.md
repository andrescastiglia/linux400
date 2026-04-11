# Roadmap Maestro de Implementación: Proyecto Linux/400

Este documento es el **plan estratégico de alto nivel** estructurado a partir de las arquitecturas definidas en `PROJECT.md` y las restricciones técnicas debatidas sobre el `KERNEL.md`. Su objetivo es organizar el desarrollo sistemático del ecosistema "Linux/400".

## User Review Required

> [!IMPORTANT]
> **Estrategia Confirmada:** Se realizará un acoplamiento progresivo de la "personalidad Linux/400" sobre un sistema host Linux base (**Kernel mínimo >= 6.11**). Una vez estabilizados los subsistemas de Compilación (CL y C/400), TUI, la gestión ZFS/DAX y los punteros LAM, se empaquetará todo en una imagen ISO instalable de Kernel Minimalista.

## Fases del Proyecto (Roadmap de Tareas)

### Fase 1: Acoplamiento BPF LSM (Kernel Hooks >= 6.11)
El requerimiento base del kernel 6.11 permite inyectar políticas rigurosas de seguridad en tiempo de ejecución.
- [x] Incorporar la toolchain Rust (`clc`, `c400c`, `libl400.so`).
- [x] Desarrollar y cargar módulos **BPF LSM** en Rust (vía *Aya*) para interceptar llamadas como `file_open` y `bprm_check_security`. Esto leerá las meta-etiquetas de ZFS e impondrá el tipado fuerte de los "Objetos" sin impacto en el userspace.

### Fase 2: Storage Object-Oriented (ZFS y Atributos SA)
Conservar la majestuosidad de ZFS para la gobernanza de datos y copias atomicas, implementando su bypass.
- [x] Inicializar ZFS en el host configurando **`xattr=sa`** (System Attributes) para que los metadatos `i:objtype` vivan incrustados en los inodos.
- [x] Crear *Storage Pools* locales simulando los ASP (Pool `/linux400pool`).
- [x] Enlazar el BPF LSM de la Fase 1 para que valide el atributo extendido de seguridad antes de cualquier apertura.

### Fase 3: Subsistema Relacional BDB y Workaround DAX
Implementar la semántica de *Single-Level Storage (SLS)* garantizando baja latencia a pesar del caché ZFS.
- [x] **Workaround DAX sobre ZFS:** Implementado en `libl400.so` mediante `AlignedBuffer` (alineación 4096 bytes) y el flag **`O_DIRECT`**, mitigando la doble capa de caché y permitiendo acceso directo a objetos ZFS.
- [x] Integrar motor de datos relacional (sustituido BDB por **Sled**) mapeando llamadas nativas a PF y LF (Archivo Físico y Lógico).
- [x] Configurar las **Colas de Datos (`*DTAQ`)** como colas transaccionales sobre `sled`.

### Fase 4: Compiladores Híbridos (Control Language y C/400)
- [x] **Compilador CL (`clc`)**: Parser Pest y codegen LLVM operativos. Soporta enlazado dinámico con LLVM 20 para evitar dependencias estáticas (libPolly).
- [x] **Compilador C/400 (`c400c`)**: Front-end envolvente de C operativo, inyectando la runtime `l400` y realizando catalogación automática en ZFS.
- [x] Los compiladores emiten binarios que inyectan los "tags" espaciales y catalogan el objeto como `*PGM` en ZFS.

### Fase 5: Memory Tagging de 64-bits (TBI / Intel LAM)
- [x] Módulo `lam.rs` con detección automática de hardware (Intel LAM48 / ARM TBI / Software Mask)
- [x] `arch_prctl` para LAM48 vía syscall inline en CPUs Intel Sapphire Rapids+
- [x] **Fallback de Software Seguro:** `untag_pointer()` con enmascaramiento bitwise (`ptr & 0x0000_FFFF_FFFF_FFFF`) para CPUs sin LAM
- [x] API pública: `tag_pointer()`, `untag_pointer()`, `get_space_bits()`, `is_tagged_pointer()`, `enable_for_platform()`
- [x] Inicialización automática via `init()` - `enable_for_platform()` llamado en carga de libl400

### Fase 6: Cargas de Trabajo (Cgroups v2)
- [x] Módulo `cgroup.rs` con gestión de slices cgroups v2
- [x] `l400.qinter` slice: `cpu.weight=10000`, `io.weight=100` (Interactive TUI/terminal)
- [x] `l400.qbatch` slice: `cpu.weight=100`, `io.weight=50` (Batch DTAQ processors)
- [x] API: `create_l400_slices()`, `assign_to_workload()`, `get_current_workload()`
- [x] Límites de memoria configurables por workload type

### Fase 7: Frontend TUI (Green Screen) e Interfaces de Consola
- [x] Crate `os400-tui/` con Ratatui y estilo Green Screen
- [x] Menú principal con opciones: WRKLIB, WRKPGM, WRKOBJ, WRKACTJOB, DSPDTAQ, CMD
- [x] Paneles: WorkManagement (WRKACTJOB), ObjectBrowser (WRKOBJ), DataQueueViewer (DSPDTAQ), CommandLine
- [x] Atajos de teclado: F3=Exit, F4=Prompt, F5=Refresh, F12=Cancel, Enter=Select
- [x] Navegación entre pantallas y historial de comandos

### Fase 8 (HITO FINAL): Empaquetado de Kernel Minimalista e ISO
- [x] Script `build_alpine_base.sh`: Rootfs Alpine Linux con musl, ZFS, LLVM
- [x] Script `build_kernel.sh`: Kernel 6.11+ con `CONFIG_BPF_LSM=y`, `CONFIG_X86_64_LAM=y`
- [x] Script `build_initramfs.sh`: Initramfs con preload BPF LSM
- [x] Script `build_iso.sh`: ISO booteable con SYSLINUX/EFI

---

## Análisis de Riesgos y Factibilidad Técnica (Actualizado)

En base a las deliberaciones arquitectónicas, los bloqueadores han sido resueltos de la siguiente manera:

> [!NOTE]
> **Superación de Conflicto ZFS vs DAX:** Es sabido que ZFS prohíbe el protocolo DAX puro a nivel VFS. El *Workaround* asumido requiere que `libl400.so` mapee las bases de datos de alto impacto en ZFS (o ZVOLs dedicados) abriendo el file descriptor con el flag **`O_DIRECT`**. Aunque perdemos el mapeo zero-copy puro a memoria persistente de memoria persistente por DAX, sorteamos el doble-caching del kernel Linux y nos recostamos puramente en el ARC de ZFS para el rendimiento sin corromper el diseño OOP de los atributos del Archivo físico y lógico.

> [!TIP]
> **Compatibilidad Extendida (Fallback LAM x86_64):** El requerimiento de usar las etiquetas de memoria a nivel hardware restringía agudamente la distribución. Integrando máscaras en software (`bitwise AND`) a nivel de la capa de API de `libl400.so`, habilitamos virtualmente cualquier CPU x86 de los 2010s a correr código empaquetado de C/400 o CL. Existirá una ínfima penalización de rendimiento en estas extracciones de memoria por software comparado al Hardware nativo de LAM (Sapphire Rapids), pero el sacrificio viabiliza la distribución general.

> [!TIP]
> **Estabilidad del Kernel (Remoción de Sched_ext):** Al fijar un base razonable de **Kernel 6.11**, podemos confiar firmemente en BPF LSM para la inspección de seguridad delegación, pero **descartamos el uso de sched_ext**, ya que requeriría la rama súper experimental 6.11. Cgroups v2 por si solo será estadísticamente suficiente para retener al `QBATCH` sin sofocar a `QINTER`.

---

## Open Questions

- Las dudas conceptuales se han disuelto. Ahora que todos los subsistemas convergen de manera determinista desde un simple Kernel >= 6.11 y validaciones hibrídas de TBI/LAM o mascara de software, el *Project Plan* ha madurado para comenzar un Sprint de código.
