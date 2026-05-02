# examples

## Objetivo

`examples` contiene programas y scripts de ejemplo que muestran flujos Linux/400 canónicos: comandos CL, administracion de bibliotecas, perfiles, objetos, PF/LF, SQL, signoff y pantallas operativas.

Su objetivo es funcionar como material de aprendizaje, datos de humo para demos y referencia corta para validar que el sistema se comporta como una experiencia AS/400 basica.

## Nivel de avance

Estado: **medio**.

Ya hay ejemplos CL para tareas de bibliotecas, objetos, usuarios, sistema, SQL y toolchain. Sirven para demos y pruebas manuales.

Para plena funcionalidad faltan:

- ejemplos end-to-end de instalacion, upgrade/PTF, backup/restore y operacion diaria;
- programas CL mas largos que combinen jobs, spool, perfiles y datos;
- ejemplos RPG y SQL cuando entren en alcance de Version 2;
- documentar para cada ejemplo el objetivo, precondiciones y salida esperada;
- reutilizar ejemplos como fixtures estables en los gates de release.
