# os400-tui

## Objetivo

`os400-tui` es la interfaz primaria de pantalla verde. Su objetivo es que un operador use Linux/400 sin depender de una shell Unix: sign-on, menu principal, linea de comandos, objetos, jobs, usuarios, spool, PDM, SEU, SQL, logs y paneles de sistema.

La TUI debe exponer capacidades reales del runtime, mostrar degradacion cuando falte plataforma y mantener una experiencia de operacion tipo OS/400.

## Nivel de avance

Estado: **medio-alto**.

Ya incluye sign-on, sesion con library list, menu, command line, F4 prompt, navegacion, object browser, work management, usuarios, spool/outq, logs, PDM, SEU, SQL y smoke tests.

Para plena funcionalidad faltan:

- completar pantallas administrativas V1: instalacion, upgrades/PTFs, backup/restore, subsistemas, job queues y spool avanzado;
- reemplazar cualquier dato demo por runtime real o mensajes de degradacion claros;
- mejorar validaciones CPF, ayuda por campo y flujos de confirmacion;
- pruebas interactivas mas amplias para navegacion, permisos y pantallas criticas;
- experiencia de operador instalada desde boot hasta apagado sin shell.
