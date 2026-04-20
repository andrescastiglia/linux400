# OS/400 (IBM i) Cheetsheet

1. Gestión de Trabajos y Sistema

* **WRKSYSSTS**: Muestra el estado general del sistema (CPU, ASP, jobs).
* **WRKACTJOB**: Gestiona los trabajos activos en tiempo real.
* **WRKSYSVAL**: Permite ver o cambiar valores de configuración del sistema.
* **DSPLOG**: Consulta el historial de mensajes del sistema (QHST).
* **WRKUSRPRF**: Gestión de perfiles de usuario.
* **PWRDWNSYS**: Apaga o reinicia el sistema (¡Cuidado con el parámetro *IMMED!).

1. Manejo de Objetos y Bibliotecas (Libraries)

* **WRKOBJ**: Busca y gestiona cualquier objeto por nombre o tipo.
* **CRTLIB / DLTLIB**: Crea o elimina una biblioteca.
* **ADDLIBLE**: Añade una biblioteca a la lista de búsqueda del usuario (Library List).
* **CHGCURLIB**: Cambia la biblioteca actual de trabajo.
* **RNMOBJ**: Renombra un objeto existente.

3. Archivos y Programación

* **STRPDM**: Inicia el Programming Development Manager (La "navaja suiza" del desarrollador).
* **STRSEU**: Inicia el editor de fuentes (Source Entry Utility).
* **CRTPGM**: Crea un objeto de programa ejecutable.
* **STRSQL**: Entorno interactivo para consultas SQL.
* **WRKMBRPDM**: Lista y gestiona miembros dentro de un archivo de fuentes.

4. Soporte y Navegación

* **GO MAIN**: Te lleva al menú principal de comandos.
* **SIGNOFF**: Cierra la sesión activa.
* **F4**: Promptear un comando (Muestra los parámetros disponibles).
* **F10**: Muestra parámetros adicionales en un prompt.
* **F11**: Cambia la vista de información en las listas (ej. de nombre a tipo de objeto).