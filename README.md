# Linux/400

Linux/400 es un ecosistema operativo que busca portar de forma nativa la personalidad fuertemente tipada de objetos del histórico entorno OS/400 (IBM i) hacia una distribución de núcleo Linux minimalista. 

El proyecto descarta la jerarquía tradicional de flujos de bytes de Unix y las capas ineficientes de software (como TIMI), y emplea instead características de vanguardia del hardware y del Ring 0:

- **BPF LSM:** Interceptores del kernel eBPF programados en Rust (Aya) que exigen un control de atributos asíncronos en los *file_open*, leyendo fuertemente etiquetas como `*PGM` o `*USRPRF` a velocidad de memoria. **(Estrictamente anclado al Kernel >= 6.11).**
- **Top-Byte Ignore (TBI/LAM):** Los inodos residen dentro del pool ZFS usando `xattr=sa` y se cargan usando etiquetas de memoria físicas en arquitecturas AMD64/ARM64.
- **Persistencia Base de Datos:** Emulación directa del Storage Único (SLS) mapeando Berkeley DB mediante `O_DIRECT` a nivel de descriptores.
- **Compiladores Nativos:** Compilador Híbrido CL (`clc`) de lenguaje de mandos y `c400c`, apoyándose íntegramente sobre Inkwell (LLVM).

---

## Compilación y Entorno (Docker Multi-Arquitectura)

Para resolver las discrepancias de *Memory Tagging* en diferentes hardwares, empleamos un entorno de compilación multi-arquitectura para Intel LAM (`amd64`) y ARM TBI (`arm64`) usando QEMU y Docker Buildx.

### 1. Preparar la Imagen Base
Este script descargará las dependencias (LLVM 15, Rust Nightly, Aya, `bpf-linker`, y librerías ZFS/BDB) y empaquetará la imagen localmente.
```bash
./build_docker_env.sh
```

### 2. Despliegue Interactivo ZFS y Docker
Dado la naturaleza intrusiva del software, hemos preparado un ejecutable unificado `run_dev_env.sh` que:
1. Validará forzosamente que tu sistema corre un **Kernel 6.11 o mayor**.
2. Creará un bloque virtual de 2GB y formateará un Pool ZFS purista (`linux400pool`) con `xattr=sa`.
3. Anclará ese dataset físico a un contenedor `--privileged` que comparte acceso al anillo `bpf/` de tu máquina host.

Para arrancar el desarrollo invoca:
```bash
./run_dev_env.sh
```
*(Nota: El comando solicitará sudo localmente unicamente para instanciar el disco ZFS).*

### 3. Usar el Toolchain (`cl_compiler`)
Una vez dentro del prompt del entorno aislado, navega al proyecto Rust para probar los binarios que compilarán el Control Language hacia objetos reales:
```bash
cd cl_compiler
cargo build --release

# O prueba un parsing directo:
./target/release/clc --help
```

### 4. Demo de objetos V1
La v1 actual toma `sled` como backend operativo de `*FILE` y `*DTAQ`, manteniendo `user.l400.objtype` como frontera autoritativa de tipado para el runtime y el LSM.

Puedes generar una demo local de bibliotecas, `*PGM`, PF/LF y `*DTAQ` con:
```bash
cargo run -p l400 --example objects_v1_demo -- /tmp/l400-demo
```

Y validar la salida esperada de esa demo con:
```bash
./scripts/test/test_objects_v1_demo.sh
```

### 5. Toolchain V1
El subset CL soportado hoy en v1 es explícito y pequeño: `PGM`, `SNDPGMMSG` y `ENDPGM`. En modo stub, `clc` genera un ejecutable observable que reproduce los `SNDPGMMSG` soportados y marca el resto de comandos como fuera del subset v1.

Los ejemplos canónicos actuales son:
```bash
tests/hola_mundo.c
tests/prueba.clp
```

Puedes validar el flujo completo `fuente -> compilación -> catalogación *PGM -> ejecución` con:
```bash
./scripts/test/test_toolchain_v1_demo.sh
```

### 6. Workloads V1
Linux/400 intenta separar carga interactiva (`QINTER`) y batch (`QBATCH`) con cgroups v2. Cuando el host no permite esa separación completa, el runtime mantiene un registro de jobs en `L400_RUN_DIR` para que la TUI y las demos sigan mostrando workloads reales en modo degradado.

Puedes generar una demo simple de job interactivo + batch con:
```bash
./scripts/test/test_workload_demo.sh
```

### 7. Loader/eBPF por modo
`l400-loader` soporta tres modos operativos:

- `full`: requiere hook eBPF activo; si no puede cargar/adjuntar el LSM, falla.
- `degraded`: intenta activar enforcement; si falla, sigue arriba sin protección activa.
- `dev`: como `degraded`, pero optimizado para desarrollo local y tolerante a assets/BTF/hooks ausentes.

Ejemplos:
```bash
cargo run -p l400-loader -- --mode full --once
cargo run -p l400-loader -- --mode degraded --once
cargo run -p l400-loader -- --mode dev --once
```

### 8. Release candidate v1
La RC v1 queda descrita en:

- `docs/RELEASE_V1_RC.md`
- `docs/SUPPORT_MATRIX.md`
- `docs/RELEASE_CHECKLIST.md`

Build reproducible de RC:
```bash
./scripts/build/build_release_rc.sh
```

Smoke tests de RC:
```bash
./scripts/test/test_release_rc.sh
RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh
```
