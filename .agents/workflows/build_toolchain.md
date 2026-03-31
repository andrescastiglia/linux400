---
description: Construir nativamente el Toolchain del SO (Compilador CL, la Runtime L400 y utilerías eBPF)
---
Paso 1: Dirigirse al directorio raíz del compilador local.
// turbo
```bash
cd /home/user/Source/os400/cl_compiler
```

Paso 2: Generar y enlazar los programas C/Rust de espacio de usuario, además de instanciar los wrappers para los ganchos BPF LSM (Aya) si los hubiera.
*(Requiere toolchain `bpf-linker` para Rust si hay módulos definidos en libl400/ebpf)*.
// turbo
```bash
cargo build --release
```

Paso 3: Validar que el toolchain primario subió localmente sin fallas antes de empaquetar ganchos eBPF en el kernel genérico.
// turbo
```bash
./target/release/clc --help
```
