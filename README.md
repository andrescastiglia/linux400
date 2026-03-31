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
