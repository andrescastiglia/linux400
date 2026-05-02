# Linux/400

Linux/400 es un experimento de sistema operativo estilo OS/400/IBM i sobre Linux. El proyecto modela bibliotecas, objetos tipados, perfiles, autorizaciones, jobs, spool, PF/LF, DTAQ, comandos y una TUI de pantalla verde encima de un runtime Rust, xattrs de filesystem, un loader eBPF LSM y herramientas de build/install.

La meta de Version 1 es una operacion basica AS/400-like: instalar, arrancar, administrar usuarios y objetos, ejecutar jobs, revisar spool, compilar CL/C, respaldar/restaurar y aplicar mantenimiento sin depender de una shell para el flujo normal.

## Documentacion

Documentos principales:

| Documento | Contenido |
| --- | --- |
| [docs/KERNEL.md](docs/KERNEL.md) | Vision general del proyecto, objetivo V1/V2, perfiles de plataforma y no-objetivos. |
| [docs/PROJECT.md](docs/PROJECT.md) | Estado actual: features implementadas, faltantes y limitaciones por area. |
| [docs/cheetsheet.md](docs/cheetsheet.md) | Cheatsheet de comandos AS/400-style por rol: operador, administrador y programador. |
| [docs/plan/implementation_plan.md](docs/plan/implementation_plan.md) | Plan por fases para llegar a Version 1. |

README por componente:

| Ruta | Contenido |
| --- | --- |
| [libl400/README.md](libl400/README.md) | Runtime central: objetos, xattrs, PF/LF/DTAQ, auth, jobs, spool, comandos y `*SRVPGM`. |
| [l400-ebpf-common/README.md](l400-ebpf-common/README.md) | Contrato `no_std` compartido entre userspace y eBPF; tipos de objeto validos. |
| [l400-ebpf/README.md](l400-ebpf/README.md) | Programa eBPF LSM, politica kernel, hooks y reglas de ejecucion. |
| [l400-loader/README.md](l400-loader/README.md) | Loader privilegiado, modos `full/degraded/dev` y diagnostico de enforcement. |
| [cl_compiler/README.md](cl_compiler/README.md) | Compilador CL (`clc`) y alcance actual del lenguaje. |
| [c400_compiler/README.md](c400_compiler/README.md) | Frontend C/400 (`c400c`) para catalogar programas nativos como `*PGM`. |
| [os400-tui/README.md](os400-tui/README.md) | TUI de pantalla verde: sign-on, menus, comandos, PDM/SEU/SQL, jobs y spool. |
| [scripts/README.md](scripts/README.md) | Build, instalacion, release gates, backup/restore, migracion y rescue. |
| [examples/README.md](examples/README.md) | Ejemplos CL y flujos demo. |
| [scratch/README.md](scratch/README.md) | Area auxiliar para experimentos y artefactos temporales. |

## Ambiente de desarrollo

Requisitos base para desarrollo userspace:

- Linux con soporte de xattrs de usuario;
- Rust/Cargo estable;
- toolchain C (`cc` o `clang`);
- `pkg-config` y librerias necesarias por el host;
- `rsync`, `tar` y utilidades core para scripts;
- opcional: QEMU/OVMF para test de instalacion;
- opcional: BPF toolchain para compilar `l400-ebpf`;
- opcional: ZFS con `xattr=sa` para perfil cercano a `full`.

Preparacion rapida:

```bash
git clone <repo> linux400
cd linux400

# Si el target Rust no esta instalado en tu host:
rustup target add x86_64-unknown-linux-gnu

# Directorios locales para desarrollo sin tocar /l400 real.
export L400_ROOT=/tmp/l400-dev
export L400_RUN_DIR=/tmp/l400-run
mkdir -p "$L400_ROOT" "$L400_RUN_DIR"
```

Inicializar objetos base:

```bash
cargo run -p l400 --bin l400-bootstrap -- --quiet
```

Nota: `cargo build` desde la raiz intenta incluir `l400-ebpf` y puede fallar si no esta instalado el toolchain BPF. Para desarrollo normal usa comandos por paquete.

## Compilar

Runtime y herramientas userspace:

```bash
cargo build -p l400
cargo build -p clc
cargo build -p c400c
cargo build -p os400-tui
cargo build -p l400-loader
```

Compilador CL:

```bash
cargo run -p clc -- --help
```

Compilador C/400:

```bash
cargo run -p c400c -- --help
```

eBPF, solo si el host tiene toolchain BPF:

```bash
cd l400-ebpf
cargo build --target bpfel-unknown-none --release
cd ..
```

Userspace empaquetable:

```bash
./scripts/build/build_userspace.sh
```

Distribucion/ISO:

```bash
./scripts/build/build_distribution.sh
```

Release candidate formal:

```bash
RUN_E2E_INSTALL=1 ./scripts/build/build_release_rc.sh
```

Para builds internos sin gate QEMU:

```bash
RUN_RC_GATE=0 ./scripts/build/build_release_rc.sh
```

## Correr

TUI interactiva:

```bash
export L400_ROOT=/tmp/l400-dev
export L400_RUN_DIR=/tmp/l400-run
cargo run -p l400 --bin l400-bootstrap -- --quiet
cargo run -p os400-tui
```

Comandos estilo AS/400 desde shell:

```bash
L400_ROOT=/tmp/l400-dev cargo run -p l400 --bin l400cmd -- WRKSYSSTS
L400_ROOT=/tmp/l400-dev cargo run -p l400 --bin l400cmd -- CRTLIB 'LIB(QGPL)'
L400_ROOT=/tmp/l400-dev cargo run -p l400 --bin l400cmd -- WRKOBJ 'LIB(QGPL)'
```

Demo de objetos V1:

```bash
cargo run -p l400 --example objects_v1_demo -- /tmp/l400-demo
```

Demo de workload interactivo/batch:

```bash
./scripts/test/test_workload_demo.sh
```

Loader eBPF por modo:

```bash
cargo run -p l400-loader -- --mode dev --once
cargo run -p l400-loader -- --mode degraded --once
cargo run -p l400-loader -- --mode full --once
```

`full` requiere BPF LSM/BTF/permisos y artefacto eBPF disponible. `dev` y `degraded` son los modos esperados para desarrollo comun.

## Probar

Gate rapido local:

```bash
cargo fmt --all --check
cargo test -p l400
cargo test -p clc
cargo test -p os400-tui
```

Smoke tests principales:

```bash
./scripts/test/test_objects_v1_demo.sh
./scripts/test/test_toolchain_v1_demo.sh
./scripts/test/test_workload_demo.sh
./scripts/test/test_loader_modes.sh
./scripts/test/test_l400_backup_restore.sh
./scripts/test/test_l400_upgrade_metadata.sh
```

Gate de release:

```bash
./scripts/test/test_release_rc.sh
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```

El test QEMU de instalacion requiere host preparado con QEMU/OVMF y puede tardar mas que los gates locales.

## Storage

El backend operativo por defecto para `*FILE` y `*DTAQ` es `sled`. `BerkeleyDb` queda como camino opt-in para builds con feature `berkeleydb` y `L400_STORAGE_BACKEND=berkeleydb`.

Para un sistema instalado, `/l400` debe estar en storage persistente con xattrs. ZFS con `xattr=sa` es el perfil recomendado; ext4/xfs con xattrs de usuario son fallback para desarrollo o modo degradado.
