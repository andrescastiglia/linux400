# Análisis y Planificación: Fase 1 - Acoplamiento BPF LSM

Este documento define la base técnica y detallada para implementar el módulo eBPF LSM de Linux/400 encargado de interceptar y forzar la tipificación de objetos subyacentes usando ZFS.

## 1. Objetivo y Diseño
La "personalidad Linux/400" exige un sistema fuertemente tipado sin degradar el rendimiento copiando buffers hacia el espacio de usuario (FUSE clásico).
La solución seleccionada ancla un programa en los módulos de seguridad de Linux (BPF LSM) usando **Rust y el framework Aya**. 
- Hook principal a auditar: `file_open` (LSM Hook de interceptación a nivel de inodo/file_descriptor).
- Condicional técnico: Si se intenta "ejecutar" o "leer" un archivo nativo protegido, el kernel debe corroborar si su atributo extendido (`xattr`) valida la semántica del objeto (ej. `*PGM` o `*FILE`).

## 2. Investigación Tecnológica y Restricciones (eBPF)

### Lectura de Atributos (xattr) dentro del Kernel
Para leer metadatos persistentes desde ZFS mientras una llamada interrumpe el LSM necesitamos usar **kfuncs** del kernel:
- **`bpf_get_file_xattr`**: Es el bridge expuesto recientemente al eBPF. 
- **Restricción de Sleepable Contexts**: Interactuar con el disco duro para leer metadatos de ZFS requiere llamadas bloqueantes. Esto significa que nuestro programa debe configurarse estrictamente como **sleepable** usando la macro `SEC("lsm.s/file_open")` (en lugar de `lsm/file_open`). 
- **Restricción de Espacio de Nombre**: La función requerirá que nuestro nombre de atributo tenga el prefijo `user.` o `security.bpf.`. Estandarizaremos en el proyecto el uso de `user.l400.objtype`.

### El Ecosistema Rust (Aya)
A diferencia de `BCC` o `libbpf` en C, **Aya** permite escribir la lógica íntegramente en Rust con fuerte tipado asíncrono.
El desarrollo introducirá tres nuevos componentes logísticos al workspace:
1. `l400-ebpf`: Código atómico (`no_std`) que irá a parar al anillo 0.
2. `l400-ebpf-common`: Shared types entre kernel y tools.
3. `l400-loader`: El demonio del user-space que el administrador usará para anclar los módulos una vez que el Linux/400 inicie.

## 3. Riesgos Asumidos de la Fase
- ZFS **no extrae automáticamente** los atributos rápidos a menos que se formatee el dataset con `xattr=sa` (System Attributes). Esto se delegará a los requerimientos de administrador.
- Las `kfuncs` de `vmlinux.h` (incluyendo `bpf_get_file_xattr`) en Rust requieren la generación correcta de binds (`bindgen`) que coincidan milimétricamente con el Kernel `>= 6.11` destino del sistema principal. Es probable que se deban usar FFI raw bindings si Aya no las proporciona nativamente en la versión actual.

## 4. Work Breakdown Structure (Checklist de Implementación)

- [x] **(1) Configuración de Toolchains Core:** Instalar `bpf-linker` vía cargo y garantizar canal Nightly de Rust (indispensable para eBPF Rust module).
- [x] **(2) Estrechar la fundación de Aya:** Crear el crate `l400-ebpf` y `l400-loader`.
- [x] **(3) Programación del LSM Hook (`file_open`):** 
   - Generar la intercepción sleepable (`lsm.s`).
   - Mapear o llamar a la kfunc `bpf_get_file_xattr` usando una signature segura.
   - Definir la lógica simple: Extraer la etiqueta `user.l400.objtype` (ej. valdrá `*PGM`, `*USRPRF`, `*FILE`) y evaluar denegación (`EACCES - Permission denied`) si el flujo es errante.
- [x] **(4) Desarrollo del Loader Space:** Escribir el gestor de espacio de usuario con `aya::Ebpf::load()` que enganche el hook al namespace BPF del OS anfitrión al inicializar el `setup_env.sh`.
- [x] **(5) Pruebas E2E:** Testear manipulando archivos ficticios con el comando estándar de linux (`setfattr -n user.l400.objtype -v "*PGM" myfile.obj`) y probar acceder sin el hook (permitido) y con el hook habilitado.
