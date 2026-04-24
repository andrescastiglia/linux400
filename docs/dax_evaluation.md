# Evaluacion tecnica de DAX para Linux/400

## Decision actual

DAX no es requisito de Linux/400 en la etapa actual. El proyecto prioriza:

- experiencia OS/400-style: sign-on, menu, comandos, TUI y trabajo sin shell;
- objetos tipados bajo `L400_ROOT`;
- xattrs como contrato de catalogo;
- `sled` como backend operativo por defecto para `*FILE` y `*DTAQ`;
- ZFS con `xattr=sa` como backend objetivo de persistencia;
- BPF LSM para enforcement basico de tipos y ejecucion.

DAX queda como perfil futuro de plataforma, no como dependencia de kernel ni como condicion para cerrar la arquitectura base.

## Por que no es requisito ahora

Linux/400 quiere acercarse al modo mental de Single-Level Storage de OS/400, pero la primera version no implementa un SLS fisico. Hoy el sistema consigue la semantica operativa mediante:

- catalogo de objetos en `/l400`;
- `user.l400.objtype` como frontera de tipo;
- metadatos `user.l400.*`;
- runtime Rust para PF/LF/DTAQ;
- binarios ELF catalogados como `*PGM`;
- punteros etiquetados LAM/TBI o mascara software como helper de runtime, no como requisito de storage.

Forzar DAX ahora introduciria una dependencia de hardware/filesystem que no ayuda al objetivo inmediato: que el usuario pueda arrancar, autenticarse, administrar objetos y desarrollar desde pantalla verde.

## Encaje tecnico de DAX

DAX permite mapear almacenamiento persistente directamente al espacio de direcciones de un proceso, saltando el page cache. Es util cuando hay PMEM/NVDIMM/CXL o dispositivos configurados como `fsdax`, y normalmente se usa con filesystems como XFS o ext4 montados con modo DAX.

Ventajas potenciales:

- menor copia entre storage y memoria;
- latencia mas estable para estructuras persistentes;
- `mmap` con acceso byte-addressable sobre medios adecuados;
- posibilidad de caches persistentes de muy baja latencia.

Limitaciones para Linux/400:

- ZFS no expone DAX como modelo nativo, y ZFS es el backend objetivo para `/l400` por snapshots, datasets, COW y `xattr=sa`.
- `sled` y Berkeley DB no pasan automaticamente a ser motores DAX-safe por estar sobre un filesystem DAX.
- La persistencia de punteros requiere disciplina de layout, versionado, flush, recovery y compatibilidad ABI; DAX por si solo no resuelve eso.
- El soporte empresarial real exigiria matriz de hardware y pruebas de corrupcion/recuperacion.

## Relacion con ZFS

La direccion de plataforma actual es:

```text
/l400 -> backend persistente con xattrs
      -> preferido: ZFS dataset con xattr=sa
      -> fallback: ext4/xfs con user xattrs
```

ZFS aporta mas valor inmediato que DAX:

- metadatos de objeto eficientes con `xattr=sa`;
- snapshots de bibliotecas;
- posible separacion futura de datasets por `*LIB`;
- integridad y recovery de almacenamiento;
- administracion parecida a pools/ASP desde la perspectiva del proyecto.

Por eso DAX no debe aparecer en `KERNEL.md` como obligatorio. Si se incorpora, debe ser un backend especializado, no reemplazo directo del catalogo `/l400`.

## Usos futuros razonables

### Perfil `SLS-DAX`

Un perfil avanzado podria declararse en `l400-support-report` cuando exista:

- hardware PMEM/NVDIMM/CXL o dispositivo `fsdax`;
- filesystem XFS/ext4 con DAX;
- pruebas de flush/recovery;
- configuracion explicita fuera del `/l400` principal;
- runtime que marque objetos compatibles con este backend.

### Cache persistente para PF/LF

PF/LF podrian mantener indices o paginas calientes en un volumen DAX, sincronizando el catalogo canonico en ZFS/sled. Esto requiere:

- invalidacion;
- journaling o redo/undo;
- checksums/versiones;
- reconstruccion ante inconsistencias.

### `*DTAQ` de baja latencia

Una cola de datos podria usar memoria persistente para workloads especificos. No debe sustituir al `DTAQ` actual hasta definir:

- orden FIFO durable;
- atomicidad de send/receive;
- timeout;
- recuperacion tras crash;
- visibilidad desde TUI y comandos.

### Espacios persistentes para objetos especiales

Los helpers LAM/TBI podrian etiquetar punteros a regiones `mmap` persistentes. Esto seria opt-in por tipo de objeto y version de ABI, no un comportamiento global.

## Requisitos si se implementa

Antes de agregar DAX al producto hay que cerrar:

1. Modelo de objeto persistente: layout, version, checksums y migracion.
2. Semantica de flush: `msync`, `pmem_persist`, `clwb`/`clflushopt` segun plataforma.
3. Recovery: deteccion de escrituras parciales y reconstruccion.
4. Integracion con `user.l400.storage_backend`.
5. Tests con crash simulado.
6. Reporte de capacidades en `l400-support-report`.
7. Documentacion operativa y comandos de administracion.

## Estado recomendado en el plan

DAX debe quedar despues de estas prioridades:

1. `/l400` persistente y bootstrap de sistema.
2. Comandos administrativos minimos.
3. `SBMJOB` y work management real.
4. PF/LF/DTAQ con esquema y comandos.
5. Seguridad/autorizacion integrada.

Solo despues tiene sentido evaluar DAX como acelerador especializado.

## Conclusion

Linux/400 no necesita DAX para cumplir su meta actual: OS/400-style sobre Linux con objetos tipados, TUI, comandos y runtime persistente. El camino correcto es terminar primero el backend durable de `/l400`, la administracion de objetos y la politica de seguridad. DAX queda reservado para una fase empresarial o experimental donde se pueda justificar con hardware, tests de recovery y un contrato claro de storage.
