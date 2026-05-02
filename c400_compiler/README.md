# c400_compiler

## Objetivo

`c400_compiler` contiene `c400c`, el frontend C de Linux/400. Su objetivo es permitir compilar programas C nativos, enlazarlos con `libl400` cuando corresponde y catalogar el resultado como objeto `*PGM` dentro del modelo de bibliotecas Linux/400.

Este componente es el puente para que codigo C sea parte del entorno operativo, invocable por `CALL`, visible como objeto y sujeto a autorizaciones, auditoria y metadata de toolchain.

## Nivel de avance

Estado: **medio**.

Ya existe el flujo base de compilacion y catalogacion de `*PGM`, con integracion al workspace Cargo y a los smoke tests de toolchain. Es suficiente para demos V1 y programas simples.

Para plena funcionalidad faltan:

- contrato mas completo de parametros y entorno de ejecucion estilo IBM i;
- diagnosticos formales y mensajes CPF consistentes para errores de compilacion/catalogacion;
- firma o attestacion fuerte del toolchain, mas alla de marcas simples;
- integracion completa con source members, PDM/SEU y comandos de compilacion desde la TUI;
- pruebas de compatibilidad para programas C con acceso a PF, DTAQ, spool y servicios runtime.
