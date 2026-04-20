# Evaluación Técnica de DAX para Linux/400

La meta final de la arquitectura de memoria etiquetada (LAM/TBI) en Linux/400 es converger en un modelo de Single-Level Store (SLS) verdadero. En este modelo, no hay diferencia semántica entre "disco" y "RAM"; todo es un puntero en un espacio de direcciones masivo de 64-bits (o 128-bits si usamos registros vectoriales, aunque aquí usamos el espacio de 48-bits virtuales + 16 bits de metadata con LAM).

Para alcanzar este ideal SLS en Linux, no basta con hacer un `mmap()` estándar a memoria virtual compartida porque:
1. `mmap` tradicional depende del `page cache` del kernel (DRAM). Cuando se produce una falta de página (page fault), el kernel debe traer los bloques de disco a RAM.
2. Cada operación de guardado (vía `msync`) requiere transferir bloques desde RAM hacia la capa de bloques subyacente.

## La Promesa de DAX (Direct Access)

DAX es una funcionalidad del kernel Linux diseñada nativamente para NVDIMMs y PMEM (Memoria Persistente), que permite saltarse completamente el page cache de Linux.

Cuando mapeamos (`mmap`) un archivo bajo DAX:
- **Zero-Copy**: El puntero virtual obtenido apunta físicamente a las direcciones reales del medio de almacenamiento.
- **Acceso a nivel de byte**: Un load/store de CPU escribe y lee directamente al medio físico. Un simple `ptr[0] = 1;` es una operación de I/O final (asumiendo instrucciones de flush del caché del CPU como `CLWB` o `CLFLUSHOPT`).
- **Eliminación de Page Faults de I/O**: Al no depender de la DRAM como intermediario, el jitter en accesos aleatorios decrece masivamente, y el rendimiento se equipara al bus de memoria (PCIe/DDR) en lugar de atravesar el stack VFS del kernel.

### Requisitos Empresariales

Para que DAX funcione en Linux/400 se requiere una alineación estricta de infraestructura:
1. **Hardware**: Almacenamiento NVMe de alto rendimiento, CXL.mem o módulos NVDIMM.
2. **Filesystem Soportado**: Actualmente Linux soporta DAX principalmente sobre `ext4`, `xfs` y sistemas de memoria como `tmpfs`/`hugetlbfs`. ZFS **no** soporta DAX actualmente, dado su arquitectura de ARC y su filosofía COW (Copy-On-Write).
3. **Mapeo**: Montar el filesystem con la bandera `-o dax` (o `dax=always`).

## Casos de Uso y Obstáculos para Linux/400

Linux/400 basa su resiliencia e inventario en ZFS. Como ZFS no tiene soporte DAX nativo, nuestra visión a futuro incluye dos alternativas:

### Alternativa 1: DAX como Backend de `*DTAQ` y Memoria Compartida (`tmpfs`)
En lugar de forzar ZFS, Linux/400 podría aprovechar DAX a través de un pool temporal o de estado en `tmpfs` o un volumen `ext4` reservado sobre NVDIMM exclusivamente para cargas críticas, como las Data Queues (`*DTAQ`) o cachés de PF/LF de latencia ultra-baja. Los objetos en la biblioteca base (ZFS) sincronizarían contra el backend DAX a requerimiento.

### Alternativa 2: Mapeos Virtualizados con DAX en CXL
Al emerger CXL (Compute Express Link), las máquinas modernas pueden montar terabytes de memoria expandida. Si asignamos un espacio de dispositivo CXL configurado como partición fsdax, Linux/400 podría mover dinámicamente objetos ZFS hacia este volumen DAX al invocarse el job interactivo, comportándose de facto como memoria persistente determinista.

## Conclusión y Siguiente Iteración

Para la Fase 7 y el cierre del diseño base, **no se requiere forzar infraestructura física DAX**. El `mmap` estándar (apoyado por el Page Cache) brinda la misma semántica SLS a nivel de ABI de punteros:
- Los binarios `*PGM` leen la estructura a través del tag de 16-bits.
- La modificación directa a punteros persiste los datos.

En un entorno empresarial o en implementaciones futuras de `Linux/400 Enterprise`, el soporte DAX pasará de ser un ejercicio documental a un perfil de plataforma avanzado (`support-profile: SLS-DAX`), demandando volúmenes XFS y hardware NVMe CXL específico.
