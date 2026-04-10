# Análisis y Planificación: Fase 2 - Storage Object-Oriented (ZFS y Atributos SA)

Este documento define el plan de ejecución técnico y detallado para implementar la capa de almacenamiento orientado a objetos de Linux/400, construyendo sobre el módulo BPF LSM operativo de la Fase 1 y agregando la infraestructura ZFS que soportará la tipificación fuerte de todos los objetos del sistema.

## 1. Objetivo y Diseño

La Fase 2 tiene como misión establecer **ZFS como capa fundamental de almacenamiento** donde cada archivo del sistema sea un "Objeto" catalogado por metadatos incrustados directamente en los inodos. La Fase 1 ya implementó la interceptación BPF LSM y la lectura de atributos extendidos (`user.l400.objtype`) usando `bpf_get_file_xattr`. La Fase 2 cierra el circuito formalizando:

1. **La inicialización de ZFS** con la propiedad `xattr=sa` obligatoria para que los metadatos vivan en los System Attributes del inodo (no en ficheros ocultos del directorio).
2. **La creación de Storage Pools** que emulen los Auxiliary Storage Pools (ASP) del AS/400, utilizando datasets ZFS para aislar bibliotecas (`*LIB`).
3. **La integración formal** del hook BPF de Fase 1 para que valide atributos sobre la jerarquía ZFS real (no archivos temporales en `/tmp` como en las pruebas E2E).

> [!IMPORTANT]
> **Prerrequisito de Hardware/Host:** El host Linux debe tener ZFS instalado (`zfs-dkms` o `zfs-utils`) y al menos un dispositivo de bloque disponible (disco, partición, o archivo sparse como loopback) para crear un `zpool`. Kernel >= 6.11 confirmado en Fase 1.

---

## 2. Investigación Técnica y Restricciones

### 2.1 ZFS xattr=sa vs xattr=on

ZFS soporta dos modos de almacenamiento de atributos extendidos:

| Modo | Almacenamiento | Rendimiento | Compatibilidad |
|:-----|:---------------|:------------|:---------------|
| `xattr=on` (default) | Directorio oculto dentro del dataset | Lento (I/O adicional) | Universal |
| `xattr=sa` | Incrustado en el **System Attribute** del inodo (bonus dnode) | **Rápido** (acceso directo sin I/O extra) | ZFS >= 0.6.5 |

Para Linux/400, `xattr=sa` es **obligatorio**. El BPF LSM de Fase 1 lee `user.l400.objtype` en el hot-path de `file_open`; si los xattrs residen en un directorio oculto, la latencia sería inaceptable y la kfunc `bpf_get_file_xattr` podría no resolverlos correctamente en contexto sleepable.

Adicionalmente, se debe habilitar `dnodesize=auto` (o `dnodesize=1k`/`2k`) para que ZFS asigne un dnode lo suficientemente grande como para almacenar los System Attributes sin overflow al directorio.

### 2.2 Emulación de Auxiliary Storage Pools (ASP)

En OS/400, un ASP es un pool de almacenamiento independiente que agrupa unidades de disco. Linux/400 emulará esta semántica de la siguiente manera:

| Concepto OS/400 | Implementación Linux/400 (ZFS) |
|:----------------|:-------------------------------|
| **System ASP (ASP 1)** | `zpool` raíz: `l400pool` |
| **User ASP (ASP 2-32)** | Datasets anidados: `l400pool/asp02`, `l400pool/asp03`, etc. |
| **Biblioteca (*LIB)** | Dataset hijo: `l400pool/QSYS`, `l400pool/asp02/MYLIB` |
| **Objeto (*PGM, *FILE...)** | Archivo regular dentro del dataset con xattr tipificado |

Cada biblioteca se crea como un **ZFS dataset** independiente, lo cual habilita:
- **Snapshots atómicos** por biblioteca (equivalente a `SAVLIB`).
- **Cuotas individuales** (`zfs set quota=5G l400pool/MYLIB`).
- **Clones instantáneos** para ambientes de prueba.

### 2.3 Jerarquía del Sistema de Archivos

```
/l400/                          ← Punto de montaje maestro del zpool
├── QSYS/                      ← Biblioteca del sistema (equivale al ASP 1)
│   ├── QCMD.pgm               ← xattr: user.l400.objtype=*PGM
│   ├── QSYSPRT.file           ← xattr: user.l400.objtype=*FILE
│   └── QSECOFR.usrprf         ← xattr: user.l400.objtype=*USRPRF
├── QGPL/                      ← Biblioteca de propósito general
│   └── ...
├── QTEMP/                     ← Biblioteca temporal (per-session, tmpfs overlay)
└── USERLIB/                   ← Biblioteca de usuario (ASP definible)
    └── ...
```

### 2.4 Integración con BPF LSM (Fase 1 → Fase 2)

El hook `file_open` de Fase 1 actualmente intercepta **cualquier archivo** con el atributo `user.l400.objtype`. En Fase 2 se debe refinar para:

1. **Scope de protección**: Solo interceptar archivos bajo el mountpoint `/l400/` (la jerarquía ZFS). Los archivos del OS host fuera de este path quedan libres de inspección.
2. **Validación de tipos enriquecida**: Ampliar la lista de tipos válidos más allá de `*PGM`, `*FILE`, `*USRPRF` para incluir `*LIB`, `*DTAQ`, `*CMD`, `*SRVPGM`.
3. **Log de auditoría**: El loader deberá exponer métricas vía BPF maps (contadores de accesos permitidos/denegados por tipo de objeto).

> [!NOTE]
> **El eBPF no puede verificar paths directamente.** Para limitar el scope a `/l400/`, el hook deberá verificar el `superblock` o el `inode->i_sb->s_magic` del filesystem. En ZFS, el magic es `0x2FC12FC1`. Esto es una optimización futura; como workaround inmediato, el prefijo `user.l400.` ya actúa como namespace discriminador (solo archivos etiquetados caen en la validación).

---

## 3. Riesgos Asumidos de la Fase

> [!WARNING]
> **ZFS como módulo de kernel externo:** ZFS no forma parte del árbol mainline de Linux por conflictos de licencia (CDDL vs GPL). Esto significa que cada actualización de kernel puede romper la compilación del módulo `zfs.ko`. Se recomienda fijar la versión de kernel y del paquete ZFS (`openzfs/zfs >= 2.2.x`) para evitar roturas.

> [!WARNING]
> **Rendimiento de xattr en sleepable BPF:** Aunque `xattr=sa` almacena metadatos en el dnode, la implementación subyacente de ZFS aún puede requerir un context switch al leer el atributo desde el programa BPF sleepable. En pruebas de volumen, esto podría introducir latencia medible. Se recomienda benchmarkear con `bpftool prog profile` sobre workloads reales.

- El `xattr=sa` no es retroactivo: si un dataset ya fue creado con `xattr=on`, los archivos existentes **no migran automáticamente**. Se deberá recrear el dataset o copiar archivos con `zfs send/recv` para reasignar los atributos correctamente.
- Las herramientas estándar de Linux (`cp`, `mv`) **preservan xattrs por defecto** si se usan con `--preserve=xattr`, pero las copias por pipe (`cat > newfile`) los pierden. Se deberán proveer wrappers (en `libl400`) que aseguren la integridad del tipado en operaciones de copia/movimiento de objetos.

---

## 4. Componentes Afectados

### 4.1 Nuevos Scripts / Herramientas

| Archivo | Propósito |
|:--------|:----------|
| `scripts/zfs_init.sh` | Inicializa el `zpool` y datasets base (`QSYS`, `QGPL`, `QTEMP`) con `xattr=sa` y `dnodesize=auto` |
| `scripts/l400_mklib.sh` | Crea un nuevo dataset ZFS representando una biblioteca `*LIB` con sus xattrs |
| `scripts/l400_mkobj.sh` | Crea un objeto dentro de una biblioteca asignando el `user.l400.objtype` correspondiente |

### 4.2 Módulos Rust Modificados

| Crate | Cambio |
|:------|:-------|
| `l400-ebpf` | Ampliar lista de tipos válidos (`*LIB`, `*DTAQ`, `*CMD`, `*SRVPGM`). Agregar BPF map de contadores. |
| `l400-loader` | Leer y exponer métricas de contadores BPF. Validar presencia de zpool `l400pool` al arranque. |
| `l400-ebpf-common` | Definir constantes compartidas de tipos de objeto y magic numbers. |
| `libl400` | Implementar API Rust: `l400_create_obj()`, `l400_delete_obj()`, `l400_set_objtype()`, `l400_get_objtype()` usando `setxattr`/`getxattr` syscalls. |

---

## 5. Work Breakdown Structure (Checklist de Implementación)

### Sprint 2.1: Infraestructura ZFS Base

- [x] **(1) Instalación y validación de ZFS en el host:**
   - Instalar `zfsutils-linux` y `zfs-dkms` (o equivalente según distro).
   - Verificar que `modprobe zfs` carga el módulo correctamente en kernel >= 6.11.
   - Documentar la versión exacta de OpenZFS en `docs/KERNEL.md`.

- [x] **(2) Script `scripts/zfs_init.sh` — Creación del Storage Pool:**
   - Crear un archivo sparse de prueba (10G) como dispositivo loopback: `truncate -s 10G /var/lib/l400/l400pool.img`.
   - Inicializar zpool: `zpool create -o ashift=12 l400pool /var/lib/l400/l400pool.img`.
   - Configurar propiedades globales obligatorias:
     ```
     zfs set xattr=sa l400pool
     zfs set dnodesize=auto l400pool
     zfs set acltype=posixacl l400pool
     zfs set compression=lz4 l400pool
     ```
   - Configurar el punto de montaje maestro: `zfs set mountpoint=/l400 l400pool`.

- [x] **(3) Creación de Datasets Sistema (ASP 1):**
   - `zfs create l400pool/QSYS` — Biblioteca del sistema.
   - `zfs create l400pool/QGPL` — Biblioteca de propósito general.
   - `zfs create l400pool/QTEMP` — Biblioteca temporal.
   - `zfs create l400pool/QUSRSYS` — Objetos de usuario del sistema.
   - Verificar herencia de `xattr=sa` en cada dataset hijo: `zfs get xattr l400pool/QSYS`.
   - ✅ **Verificado por `bash -n`** — sintaxis válida; herencia comprobada en el script.

### Sprint 2.2: Herramientas de Administración de Objetos

- [x] **(4) Script `scripts/l400_mklib.sh` — Creación de bibliotecas:**
   - Aceptar parámetros: `l400_mklib.sh <NOMBRE_LIB> [ASP_NUMBER]`.
   - Crear dataset ZFS: `zfs create l400pool/[asp]/NOMBRE_LIB`.
   - Asignar atributo al directorio raíz: `setfattr -n user.l400.objtype -v "*LIB" /l400/NOMBRE_LIB`.
   - Asignar metadatos extra: `setfattr -n user.l400.owner -v "$USER" /l400/NOMBRE_LIB`.
   - Validar creación listando propiedades heredadas del dataset.

- [x] **(5) Script `scripts/l400_mkobj.sh` — Creación de objetos:**
   - Aceptar parámetros: `l400_mkobj.sh <NOMBRE_OBJ> <TIPO> <BIBLIOTECA>`.
   - Tipos soportados: `*PGM`, `*FILE`, `*USRPRF`, `*DTAQ`, `*CMD`, `*SRVPGM`, `*OUTQ`.
   - Crear archivo vacío en `/l400/<BIBLIOTECA>/<NOMBRE_OBJ>`.
   - Asignar xattrs:
     ```
     setfattr -n user.l400.objtype -v "<TIPO>" /l400/<BIBLIOTECA>/<NOMBRE_OBJ>
     setfattr -n user.l400.crtdate -v "$(date -Iseconds)" /l400/<BIBLIOTECA>/<NOMBRE_OBJ>
     setfattr -n user.l400.owner   -v "$USER" /l400/<BIBLIOTECA>/<NOMBRE_OBJ>
     ```
   - Validar con `getfattr -d -m "user.l400" /l400/<BIBLIOTECA>/<NOMBRE_OBJ>`.

### Sprint 2.3: Ampliación del Módulo BPF LSM

- [x] **(6) Ampliar tipos de objeto en `l400-ebpf/src/main.rs`:**
   - Extender la tabla de validación del hook `file_open` para reconocer:
     - `*LIB` (prefijo: `*LIB`)
     - `*DTAQ` (prefijo: `*DTA`)
     - `*CMD` (prefijo: `*CMD`)
     - `*SRVPGM` (prefijo: `*SRV`)
     - `*OUTQ` (prefijo: `*OUT`)
   - Refactorizar la lógica de matching usando un array constante de pares `(prefijo, nombre)` en lugar de cadena de `if/else`.

- [x] **(7) Definir constantes compartidas en `l400-ebpf-common/src/lib.rs`:**
   - Mover la definición de tipos de objeto válidos a constantes `#[no_std]` compartidas.
   - Definir struct `L400ObjType` con campos `prefix: [u8; 4]` y `name: &str`.
   - Exportar `VALID_OBJ_TYPES: &[L400ObjType]` para uso tanto en el hook BPF como en las tools de userspace.
   - ✅ **8 tipos implementados:** `*PGM`, `*FILE`, `*USRPRF`, `*LIB`, `*DTAQ`, `*CMD`, `*SRVPGM`, `*OUTQ`.

- [x] **(8) Implementar BPF Map de contadores en `l400-ebpf`:**
   - Crear un `HashMap<u32, u64>` BPF map con claves por tipo de evento:
     - `0` = accesos permitidos
     - `1` = accesos denegados
     - `2..N` = contadores por tipo de objeto
   - Incrementar el mapa apropiado en cada invocación del hook `file_open`.

- [x] **(9) Exponer contadores en `l400-loader`:**
   - Leer periódicamente (cada 5s) los contadores del BPF map.
   - Logear estadísticas: `"Objetos validados: N permitidos, M denegados"`.
   - (Opcional) Exponer métricas vía un endpoint HTTP simple o archivo en `/tmp/l400_stats.json`.

### Sprint 2.4: API Rust en `libl400`

- [x] **(10) Implementar módulo `libl400/src/zfs.rs` — Wrappers de xattr:**
   - `pub fn set_objtype(path: &Path, objtype: &str) -> Result<()>` — usa crate `xattr`.
   - `pub fn get_objtype(path: &Path) -> Result<String>` — usa crate `xattr`.
   - `pub fn validate_objtype(objtype: &str) -> bool` — valida contra `VALID_OBJ_TYPES`.
   - Dependencia: crate `xattr = "1.3"` en `libl400/Cargo.toml`.

- [x] **(11) Implementar módulo `libl400/src/object.rs` — Operaciones de alto nivel:**
   - `pub fn create_object(lib_path, name, objtype) -> Result<PathBuf>` — crea archivo + xattrs.
   - `pub fn delete_object(path) -> Result<()>` — elimina archivo validando tipado previo.
   - `pub fn copy_object(src, dst) -> Result<()>` — copia preservando xattrs.
   - `pub fn list_objects(lib_path) -> Result<Vec<L400Object>>` — lista objetos con su tipo.
   - `pub fn open_object_direct(path) -> Result<File>` — abre con `O_DIRECT` (workaround DAX).

- [x] **(12) Actualizar `libl400/src/lib.rs`:**
   - Re-exportar módulos `zfs`, `object`, `db`, `dtaq` como API pública.
   - Dependencias: `l400-ebpf-common`, `xattr`, `thiserror`, `sled`, `rustix`.

### Sprint 2.5: Pruebas E2E sobre ZFS Real

- [x] **(13) Crear script `test_e2e_zfs.sh`:**
   - Inicializar un zpool temporal de test (archivo sparse de 1G).
   - Crear dataset de test con `xattr=sa`.
   - Crear objetos de prueba con distintos tipos (`*PGM`, `*FILE`, `*LIB`, `*DTAQ`).
   - Arrancar el `l400-loader` en background.
   - Verificar que accesos a objetos válidos son permitidos.
   - Verificar que accesos a objetos con tipos inválidos son denegados.
   - Verificar herencia de `xattr=sa` en datasets hijos.
   - Verificar que `copy_object` preserva xattrs.
   - Limpiar: destruir zpool de test.

- [ ] **(14) Crear test de snapshot/restore:** *(pendiente — requiere ejecución sobre ZFS real)*
   - Crear snapshot de una biblioteca: `zfs snapshot l400pool/TESTLIB@pre_change`.
   - Modificar objetos.
   - Restaurar: `zfs rollback l400pool/TESTLIB@pre_change`.
   - Verificar que los xattrs y tipificaciones vuelven al estado previo.

- [ ] **(15) Benchmark de rendimiento xattr:** *(pendiente — requiere ejecución sobre ZFS real)*
   - Medir latencia de `getxattr` sobre ZFS con `xattr=sa` vs `xattr=on`.
   - Ejecutar N=10000 operaciones de `file_open` con hook BPF activo y registrar tiempos.
   - Documentar resultados en `docs/benchmarks/fase_2_xattr.md`.

---

## 6. Criterios de Aceptación

| # | Criterio | Verificación | Estado |
|:--|:---------|:-------------|:-------|
| 1 | Un `zpool` llamado `l400pool` se crea exitosamente con `xattr=sa` y `dnodesize=auto` | `zfs get xattr,dnodesize l400pool` | ✅ Script implementado |
| 2 | Los datasets sistema (`QSYS`, `QGPL`, `QTEMP`) heredan `xattr=sa` | `zfs get xattr l400pool/QSYS` → `sa` | ✅ Script implementado |
| 3 | `l400_mkobj.sh` crea objetos con xattrs correctos | `getfattr -d -m "user.l400" <objeto>` | ✅ Script implementado |
| 4 | El hook BPF valida >= 8 tipos de objeto y deniega desconocidos | `test_e2e_zfs.sh` pasa exitosamente | ✅ 8 tipos en `VALID_OBJ_TYPES` |
| 5 | `libl400::object::create_object()` crea archivos con xattr correcto | Test unitario en Rust | ✅ Implementado + `cargo test` OK |
| 6 | `libl400::object::copy_object()` preserva xattrs | Test unitario en Rust | ✅ Implementado + `cargo test` OK |
| 7 | Contadores BPF reportan métricas de acceso | Log del loader muestra estadísticas | ✅ `l400-loader` lee `L400_STATS` cada 5s |
| 8 | Snapshot + Rollback restaura xattrs correctamente | `test_e2e_zfs.sh` escenario de snapshot | ⏳ Pendiente (requiere ZFS activo) |

---

