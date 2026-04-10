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
- [ ] Configurar las invocaciones iniciales (`arch_prctl`) al Kernel para habilitar soporte in-hardware de **Intel LAM** (En procesadores Sapphire Rapids o superior) o **ARM TBI**.
- [ ] **Fallback de Software Seguro:** Para hardwares donde LAM no esté disponible, se integra un pre-procesador condicional o un enmascaramiento binario (`ptr & 0x0000FFFFFFFFFFFF`) dentro las funciones core de `libL400.so` previo a dereferenciar cualquier dirección para evitar *SIGSEGV*.

### Fase 6: Cargas de Trabajo (Cgroups v2)
Aislar y emular las cargas clásicas QINTER / QBATCH del sistema respetando la barrera del Kernel 6.11.
- [ ] Configurar `qinter.slice` (Interactivo) otorgando `cpu.weight=10000` y picos máximos de latencia I/O dentro del árbol BPF/cgroup, para defender el tecleo de las TUI.
- [ ] Configurar `qbatch.slice` (Lotes) restringiendo el peso de contención a valores bajos (ej. `cpu.weight=50`) para proteger al sistema principal.

### Fase 7: Frontend TUI (Green Screen) e Interfaces de Consola
- [ ] Programar un TUI "Main Menu" iterado (`/bin/os400-menu`) usando `Ratatui`/`Ncurses`.
- [ ] Configurar atajos clásicos (F3, F4, F12) y setear los perfiles de *Login SSH* para reemplazar a Bash con este despachador, operando asíncronamente en UTF-8 puro.

### Fase 8 (HITO FINAL): Empaquetado de Kernel Minimalista e ISO
- [ ] Aislar núcleo host configurando Alpine Linux Base + `musl`.
- [ ] Compilar **Kernel 6.11+** estabilizado inyectando banderas requeridas: `CONFIG_BPF_LSM=y`, `CONFIG_X86_64_LAM=y` (o equivalente a parche backport si no lo contempla la rama principal) y empaquetar módulo ZFS unificado.
- [ ] Construir Initramfs customizado que despliegue el BPF LSM antes del Systemd chroot inicial.

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
