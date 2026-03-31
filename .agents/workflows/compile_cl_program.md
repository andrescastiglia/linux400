---
description: Compilar un script de Control Language (.clp) y enlazarlo con la runtime optimizada (LAM/TBI y DAX habilitables)
---
Este flujo asume que ya posees un archivo .clp válido. Reemplaza `<CLP_FILE>` con el archivo fuente y `<OUT>` con el nombre final deseado. Asegúrate de estar ejecutando el host con DAX activado o bajo un kernel con LAM/TBI en modo pasivo.

Paso 1: Usar Cargo directamente para invocar `clc`, el cual enlazará estáticamente los módulos de `libl400.so` embebidos con llamadas locales de memoria.
// turbo
```bash
cd /home/user/Source/os400/cl_compiler
cargo run --bin clc -- -i <CLP_FILE> -o <OUT>
```

Paso 2: Iniciar el ejecutable permitiendo experimentalmente los punteros etiquetados a nivel proceso (ejemplo de utilidad conceptual, ya que libl400 lo controlará automáticamente en su contexto interno a través de `prctl` en C).
// turbo
```bash
# ./<OUT>
```
