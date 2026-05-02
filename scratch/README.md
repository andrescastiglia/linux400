# scratch

## Objetivo

`scratch` es un area auxiliar para pruebas locales, artefactos temporales, paquetes descargados y experimentos que no forman parte del runtime principal.

Su objetivo es facilitar investigacion y validaciones puntuales sin mezclar esos archivos con crates o scripts productivos.

## Nivel de avance

Estado: **auxiliar / sin objetivo de funcionalidad plena**.

No es un componente de producto. Actualmente contiene insumos como paquetes Berkeley DB y utilidades de verificacion.

Para mantenerlo sano faltan:

- documentar cada experimento que deba conservarse;
- mover a `scripts`, `docs` o crates reales cualquier pieza que pase a ser parte del producto;
- evitar depender de archivos de `scratch` en gates de release;
- limpiar artefactos obsoletos cuando ya no sirvan para reproducibilidad.
