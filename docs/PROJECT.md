# Linux/400: proyecto y arquitectura actual

Linux/400 busca recrear la forma de trabajo de OS/400/IBM i sobre Linux: sign-on, menu principal, comandos operativos, objetos tipados, bibliotecas, trabajos interactivos/batch y herramientas de desarrollo en pantalla verde. No persigue compatibilidad binaria ni reemplazar IBM i programa por programa. La meta es que el operador no tenga que vivir en `bash`: entra a una sesion Linux/400 y opera el sistema desde menus y comandos estilo OS/400.

## Principios de diseño

- **Modo de trabajo primero**: sign-on, menu, linea de comandos, teclas F, opciones y pantallas de trabajo son parte del producto, no una demo.
- **Linux como microplataforma operativa**: se usa Linux para procesos, memoria, seguridad, cgroups, arranque y drivers.
- **Objetos sobre filesystem**: los objetos viven bajo `L400_ROOT` (`/l400` por defecto) y se tipan con xattrs.
- **No compatibilidad historica estricta**: UTF-8, ELF nativo, SSH/TTY y Rust/C reemplazan EBCDIC, TIMI y 5250.
- **Degradacion explicita**: si eBPF, ZFS, BTF o cgroups no estan disponibles, el sistema debe informar el modo degradado y seguir sirviendo para desarrollo/operacion basica.

## Estado del repo

| Area | Estado actual |
| --- | --- |
| Runtime de objetos | Implementado en `libl400`: crear/listar/copiar/borrar/catalogar objetos, bibliotecas, xattrs y source members. |
| Tipos compartidos | Implementado en `l400-ebpf-common`, `#![no_std]`, usado por runtime y eBPF. |
| PF/LF/DTAQ | Implementado con `sled` por defecto; Berkeley DB queda como backend opcional. |
| Politica kernel | Implementada con Aya BPF LSM y loader con modos `full`, `degraded`, `dev`. |
| TUI | Implementada con `ratatui`: sign-on, menu principal, object browser, jobs, DTAQ, command line, STRPDM, WRKMBRPDM, STRSEU, STRSQL. |
| Comandos batch | Dispatcher `l400cmd` y funciones FFI para comandos operativos/desarrollo. |
| Compiladores | `clc` genera C intermedio por defecto y enlaza contra `libl400`; `c400c` compila C a ELF y cataloga `*PGM`. |
| Workloads | cgroups v2 best-effort y job registry en `L400_RUN_DIR`. |
| Distribucion | Scripts de userspace, initramfs, ISO live/install, instalador textual y soporte. |

## Mapa de componentes

### `libl400/`

El core del sistema. Expone APIs Rust y funciones C para programas CL/C compilados.

- `object.rs`: catalogo de objetos, bibliotecas, source members y metadatos.
- `zfs.rs`: lectura/escritura de `user.l400.objtype`, validacion contra `VALID_OBJ_TYPES` y helpers ZFS.
- `storage.rs`: seleccion de backend (`sled` por defecto, `berkeleydb` opcional) y xattrs de storage.
- `db.rs`: PF, LF, indices secundarios y `SELECT` minimo.
- `dtaq.rs`: colas de datos persistentes.
- `cgroup.rs`: `QINTER`, `QBATCH` y job registry.
- `auth.rs`: autorizaciones de objeto (`*USE`, `*CHANGE`, `*ALL`, `*EXCLUDE`) sobre xattrs.
- `lam.rs`: helpers de punteros etiquetados LAM/TBI/software.
- `ffi_commands.rs`: comandos OS/400-style invocables desde CL compilado y `l400cmd`.
- `bin/l400cmd.rs`: dispatcher de comandos por symlink o `l400cmd CMD`.
- `bin/sbmjob.rs`: envio batch experimental.

### `os400-tui/`

La experiencia interactiva principal.

Flujo actual:

```text
SignOn -> MainMenu
        -> ObjectBrowser
        -> WorkManagement
        -> DataQueueViewer
        -> CommandLine
        -> STRPDM -> WRKMBRPDM -> STRSEU
                              -> STRSQL
```

La TUI autentica contra PAM o `/etc/shadow`, bloquea el perfil `ROOT`, arranca sugerida como `QSECOFR`, registra su job interactivo y muestra estado del loader eBPF.

### `l400-ebpf-common/`, `l400-ebpf/`, `l400-loader/`

La capa kernel no intenta reimplementar OS/400 dentro del kernel. Se limita a reforzar la frontera de objetos:

- objetos con `user.l400.objtype` valido pueden abrirse;
- etiquetas desconocidas se deniegan;
- solo `*PGM` con atributo de toolchain valido se puede ejecutar;
- binarios nativos sin etiqueta siguen funcionando para compatibilidad del sistema base.

El loader persiste estado para que la TUI y los reportes sepan si la proteccion esta activa.

### `cl_compiler/clc/`

Compilador CL nativo. En la ruta default:

1. parsea con Pest;
2. genera C intermedio;
3. compila objeto `.o` con `clang` o `cc`;
4. enlaza contra `libl400`;
5. cataloga el resultado como `*PGM`.

El backend LLVM existe tras el feature `llvm-backend`, pero no es el camino principal actual.

### `c400_compiler/`

Compilador C/400 simple: delega en `clang`/`cc`, enlaza con `libl400` y cataloga el binario ELF como `*PGM` con atributo `C`.

## Modelo de objetos

La unidad basica es un archivo o directorio con xattrs Linux/400.

| Concepto | Representacion actual |
| --- | --- |
| Biblioteca `*LIB` | Directorio bajo `L400_ROOT`, opcionalmente dataset ZFS futuro. |
| Programa `*PGM` | ELF Linux catalogado con `user.l400.objtype=*PGM` y `user.l400.objattr=C` o `CL`. |
| Archivo fisico PF | Directorio/base `sled` catalogado como `*FILE`, atributo `PF`, tree `PF_MEMBER`. |
| Archivo logico LF | Objeto `*FILE`, atributo `LF`, indice secundario `LF_IDX_<name>` y xattr `base_pf`. |
| Data queue `*DTAQ` | Objeto catalogado, tree `DTAQ`, lectura FIFO por clave creciente. |
| Source file | Directorio `*FILE` con atributo `SRC`; miembros como archivos planos. |
| Perfil `*USRPRF` | Soporte inicial de objeto y comandos; integracion completa con usuarios Linux pendiente. |

El tipo autorizado se valida en `l400-ebpf-common/src/lib.rs`. Agregar un tipo nuevo requiere actualizar ese crate y revisar runtime/eBPF.

## Experiencia operativa

En la ISO live/install, el usuario deberia ver una consola Linux/400:

1. `l400-console-autologin.sh` entra con `qsecofr` o abre instalador/rescue segun boot mode.
2. `l400-session.sh` prepara entorno (`PATH`, `L400_ROOT`, `L400_LIB_PATH`, `LD_LIBRARY_PATH`) y lanza `os400-tui`.
3. `GO MAIN` abre la TUI cuando hay terminal interactiva.
4. Los comandos OS/400-style se exponen como symlinks a `l400cmd` en `/opt/l400/bin` o `/usr/local/bin`.

Comandos disponibles hoy en el dispatcher:

```text
WRKSYSSTS WRKACTJOB WRKSYSVAL DSPLOG WRKUSRPRF PWRDWNSYS
WRKOBJ CRTLIB DLTLIB ADDLIBLE CHGCURLIB RNMOBJ CRTPGM
GO SIGNOFF STRPDM STRSEU STRSQL WRKMBRPDM
```

`SBMJOB` existe como bin Rust, pero todavia debe integrarse al empaquetado y al dispatcher operativo.

## Seguridad y autorizaciones

Hay tres capas:

- **Catalogo**: `user.l400.objtype` y metadatos de objeto.
- **Runtime**: `user.l400.auth` con autorizaciones estilo OS/400 (`*PUBLIC`, usuario especifico, `*EXCLUDE`).
- **eBPF LSM**: enforcement basico de tipos y ejecucion de `*PGM`.

Pendiente importante: hacer que las autorizaciones runtime y el enforcement kernel converjan en una politica unica visible desde comandos administrativos.

## Almacenamiento

La implementacion ejecutable usa `sled` por defecto porque compila y prueba sin dependencias externas. Berkeley DB permanece como backend opt-in. ZFS sigue siendo la direccion de plataforma para `/l400`, especialmente por `xattr=sa`, snapshots y datasets por biblioteca, pero no debe asumirse como presente en todos los entornos de desarrollo.

Variables relevantes:

```bash
L400_ROOT=/l400
L400_RUN_DIR=/run/l400
L400_STORAGE_BACKEND=sled        # default
L400_STORAGE_BACKEND=berkeleydb  # requiere feature berkeleydb
L400_ZFS_CREATE_DATASETS=0       # desactiva creacion automatica de datasets
L400_ZFS_DATASET_PREFIX=pool/linux400
```

## Build y pruebas

Construcciones seguras desde raiz:

```bash
cargo build -p c400c
cargo build -p clc
cargo build -p l400-loader
cargo test -p l400
```

Flujos smoke:

```bash
./scripts/test/test_objects_v1_demo.sh
./scripts/test/test_toolchain_v1_demo.sh
./scripts/test/test_workload_demo.sh
./scripts/test/test_loader_modes.sh
./scripts/test/test_release_rc.sh
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

El gate de RC, la matriz de plataformas y el procedimiento de migracion de `/l400` estan definidos en `docs/release_platforms.md`.

eBPF requiere toolchain BPF:

```bash
cd l400-ebpf
cargo build --target bpfel-unknown-none --release
```

## Brechas principales

1. Validar en QEMU/instalacion real que `/l400` sobrevive reboot con datos de usuario, no solo objetos base.
2. Agregar confirmaciones visuales dedicadas en TUI para acciones destructivas.
3. Enriquecer sesion multi-job y validacion interactiva completa desde ISO/TUI.
4. Completar prompt F4 avanzado con tabulacion campo a campo y validacion por tipo.
5. Mostrar PF/LF/DTAQ desde TUI con flujos accionables.
6. Integrar scroll/prompt SQL interactivo dentro de TUI.
7. Capturar codigos CPF reales para `MONMSG`.
8. Converger identidad/autorizacion completa con eBPF para usuarios, grupos y owner.
