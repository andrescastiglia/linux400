# cl_compiler

## Objetivo

`cl_compiler` agrupa el compilador de Control Language. El crate activo es `cl_compiler/clc`, que parsea CL, genera codigo nativo y cataloga programas como `*PGM`.

Su objetivo es que el operador pueda escribir miembros CL, compilarlos y ejecutar programas con semantica operacional Linux/400: comandos, variables, parametros, control de flujo, `MONMSG`, llamadas a otros programas y estado CPF del runtime.

## Nivel de avance

Estado: **medio-alto**.

`clc` ya tiene parser Pest, AST, backend C por defecto, backend LLVM opcional, soporte de variables, parametros, `IF/ELSE`, `DO/ENDDO`, `MONMSG`, `CALL` y comandos runtime relevantes. El flujo fuente -> compilacion -> catalogacion `*PGM` esta cubierto por pruebas.

Para plena funcionalidad faltan:

- ampliar cobertura del lenguaje CL y comandos administrativos;
- mejorar compatibilidad de parametros, mensajes CPF y `MONMSG` en escenarios complejos;
- integracion de compilacion desde pantallas PDM/SEU como flujo primario;
- catalogo de errores y ayudas de prompt mas completo;
- pruebas de regresion para programas CL reales de administracion V1.
