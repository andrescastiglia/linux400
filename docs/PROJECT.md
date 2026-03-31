# **Arquitectura e Implementación de Linux/400: Un Sistema Operativo de Objetos sobre un Núcleo Linux Minimalista**

La convergencia entre la robustez histórica de la arquitectura de gama media de IBM, específicamente el sistema operativo OS/400 (actualmente IBM i), y la versatilidad de código abierto del núcleo Linux representa uno de los desafíos más ambiciosos en la ingeniería de sistemas contemporánea. El proyecto Linux/400 no busca simplemente la emulación superficial o la virtualización de un entorno heredado, sino la creación de una personalidad de sistema operativo nativa que fusione la gestión de objetos fuertemente tipados y la integración de bases de datos de OS/400 con el rendimiento y la ligereza de un kernel Linux optimizado.1 Esta propuesta detalla la construcción de una distribución denominada Linux/400, diseñada para eliminar las capas de abstracción innecesarias como la Interfaz de Máquina Independiente de la Tecnología (TIMI), adoptando en su lugar una manipulación directa de punteros etiquetados de 64 bits y un sistema de almacenamiento basado en ZFS y Berkeley DB que preserva la semántica de los archivos físicos y lógicos.3

## **Fundamentos del Núcleo y la Distribución Minimalista**

La base de Linux/400 es un núcleo Linux configurado en su estado más elemental. A diferencia de las distribuciones de propósito general que incluyen controladores de video, pilas de sonido y servicios de escritorio, Linux/400 se asienta sobre una base similar a Alpine Linux o un kernel compilado a medida que utiliza musl libc por su eficiencia y reducido tamaño.6 El proceso de arranque, análogo al Programa de Carga Inicial (IPL) del AS/400, está optimizado para inicializar únicamente los subsistemas críticos de gestión de objetos y la pila de red para conectividad SSH.1

La arquitectura de Linux/400 rechaza la jerarquía de archivos tradicional de Unix basada en flujos de bytes sin estructura. En su lugar, el kernel se utiliza como un despachador de recursos y un gestor de memoria, mientras que la capa de personalidad de Linux/400 impone un modelo donde cada archivo en el disco es tratado como un objeto con metadatos estrictos.2 Este enfoque permite que el sistema mantenga la integridad y la seguridad que caracterizaron al AS/400, pero con la capacidad de respuesta de un sistema Linux moderno.9

| Característica | OS/400 Tradicional | Linux Estándar | Proyecto Linux/400 |
| :---- | :---- | :---- | :---- |
| **Núcleo** | SLIC (Proprietario) | Linux Monolítico | Linux Minimalista (L400) |
| **Abstracción de CPU** | TIMI (Virtual) | Nativa | Nativa (Sin TIMI) |
| **Gestión de Datos** | DB2 Integrada | Archivos de texto/binarios | Berkeley DB Integrada |
| **Sistema de Archivos** | Almacenamiento de un solo nivel | Jerárquico (VFS) | Objetos sobre ZFS |
| **Codificación** | EBCDIC | ASCII / UTF-8 | UTF-8 Nativo |
| **Interfaz de Usuario** | 5250 (Green Screen) | Shell (Bash/Zsh) | Menús TUI sobre SSH |

## **Encapsulación de Objetos y el Wrapper de ZFS**

El corazón de Linux/400 es su sistema de gestión de objetos. En el sistema original, un objeto es una unidad encapsulada que incluye tanto los datos como las operaciones permitidas sobre ellos.2 Para implementar esto sobre Linux sin comprometer el rendimiento, se utiliza ZFS como capa de almacenamiento subyacente. ZFS se selecciona no solo por su capacidad de gestión de volúmenes, sino por su potente soporte para propiedades definidas por el usuario y atributos extendidos (xattr), que sirven como el repositorio de metadatos para la tipificación fuerte.4

Cada objeto de Linux/400 (ya sea un programa \*PGM, una cola de datos \*DTAQ o un perfil de usuario \*USRPRF) se almacena como una entidad en ZFS. El sistema no permite el acceso directo a estos archivos a través de herramientas estándar de Linux de forma predeterminada; en su lugar, un wrapper de nivel de sistema intercepta las llamadas de E/S. Este wrapper verifica la propiedad i:objtype en los metadatos de ZFS antes de permitir cualquier operación.4

### **Implementación de Tipos de Objetos**

El uso de ZFS permite que Linux/400 implemente bibliotecas como datasets de ZFS. Esto proporciona un aislamiento natural y la capacidad de aplicar cuotas, instantáneas (snapshots) y límites de rendimiento a nivel de biblioteca, emulando los pools de almacenamiento auxiliar (ASP) del sistema original.2

| Objeto Linux/400 | Representación en ZFS | Atributos de Tipificación |
| :---- | :---- | :---- |
| \*LIB (Biblioteca) | Dataset / Directorio Raíz | i:type=library, i:owner |
| \*PGM (Programa) | Binario ELF con wrapper | i:type=program, i:entry\_point |
| \*FILE (Archivo) | Base de Datos Berkeley DB | i:type=file, i:record\_format |
| \*DTAQ (Cola) | Archivo de Mensajes BDB | i:type=data\_queue, i:max\_len |
| \*USRPRF (Perfil) | Objeto de Configuración JSON | i:type=user\_profile, i:uid |

Para evitar la necesidad de la capa TIMI, los programas \*PGM se compilan directamente a formato ELF de Linux, pero contienen una cabecera extendida que el cargador de Linux/400 reconoce.13 Al ejecutar un programa, el sistema verifica que el código haya sido generado por el compilador oficial de CL y que no haya sido modificado, manteniendo la integridad del sistema de objetos sin la penalización de la traducción en tiempo de ejecución.2

## **Gestión de Memoria y Punteros Etiquetados de 64 Bits**

Uno de los pilares del diseño de Linux/400 es la transición del almacenamiento de un solo nivel (SLS) de 128 bits a un sistema de punteros etiquetados de 64 bits que permite la manipulación directa por parte del programa.3 El objetivo es "salvar" las direcciones de memoria, es decir, proporcionar una persistencia lógica de las referencias a objetos sin la sobrecarga de un espacio de direccionamiento global de 128 bits que requiera traducción constante por software.3

### **Implementación Técnica de Pointer Tagging**

En las arquitecturas modernas de 64 bits (como x86\_64 y AArch64), no todos los bits del puntero se utilizan para el direccionamiento físico. Linux/400 aprovecha esto mediante el uso de Linear Address Masking (LAM) en Intel/AMD y Top Byte Ignore (TBI) en ARM.15 Al configurar el kernel para ignorar los bits superiores del puntero durante el acceso a la memoria, Linux/400 puede almacenar metadatos críticos directamente en el puntero de 64 bits.15

Code snippet

Puntero\_{L400} \= (Etiqueta \\ll 56\) \\lor Direccion\_{Virtual}

Esta etiqueta de 8 bits (en TBI) o de hasta 15 bits (en LAM\_U48) se utiliza para verificar el tipo de objeto y los derechos de acceso en tiempo de ejecución por el hardware o por micro-rutinas de seguridad, permitiendo que el programa manipule el puntero directamente.15 Si un programa intenta realizar aritmética de punteros que corrompa la etiqueta, el hardware generará una excepción al intentar desreferenciar una dirección fuera de la máscara permitida, proporcionando seguridad sin la necesidad de una capa de software intermedia.17

### **Persistencia de Direcciones sin SLS**

Para evitar la complejidad del SLS pero mantener su "aspecto", Linux/400 emplea un mapeo determinista de objetos en el espacio virtual del proceso. Mediante el uso de mmap() con direcciones sugeridas basadas en un hash del identificador único del objeto (UUID), el sistema asegura que un objeto específico tienda a aparecer en la misma dirección virtual a través de diferentes ejecuciones.3 Esto permite que las estructuras de datos que contienen punteros persistan en el disco (dentro de objetos de ZFS) y sigan siendo válidas al ser re-mapeadas, siempre que el gestor de memoria de Linux/400 respete estos rangos reservados.3

## **El Subsistema de Base de Datos: Archivos Físicos y Lógicos**

La implementación de la base de datos es fundamental para recrear la experiencia de OS/400. En lugar de utilizar un motor SQL externo que trate los datos como entidades separadas, Linux/400 utiliza Berkeley DB (BDB) como motor de almacenamiento para sus objetos \*FILE.5 Cada archivo físico (PF) de OS/400 se mapea a una base de datos primaria de BDB, donde los registros se almacenan en secuencia de llegada (Arrival Sequence) y se accede a ellos mediante su Número de Registro Relativo (RRN).22

### **Archivos Lógicos y Secundarios de Berkeley DB**

Los archivos lógicos (LF), que proporcionan vistas ordenadas o filtradas de los datos físicos, se implementan utilizando los "Secondary Indexes" de Berkeley DB.5 Cuando se crea un archivo lógico mediante el comando CRTLF, Linux/400 genera una base de datos secundaria de BDB que se asocia automáticamente a la primaria. Berkeley DB maneja de forma nativa la actualización de estos índices cada vez que se inserta, actualiza o elimina un registro en el archivo físico.5

| Concepto OS/400 | Implementación Linux/400 | Mecanismo Berkeley DB |
| :---- | :---- | :---- |
| **Physical File (PF)** | Almacenamiento de Datos Real | Primary Database (B-Tree/Hash) |
| **Logical File (LF)** | Vista / Índice de Datos | Secondary Database (Associated) |
| **Member (MBR)** | Partición de Datos | Subdatabases en un solo archivo BDB |
| **Key Field** | Campo de Ordenación | Secondary Key Function |
| **Select/Omit** | Filtrado de Registros | Callback de generación de claves secundarias |

Esta arquitectura permite que Linux/400 soporte archivos multifortmato y multifilial de manera eficiente. Para los miembros de archivos (una característica distintiva donde un solo archivo físico contiene múltiples conjuntos de datos independientes), se utilizan las "Subdatabases" de Berkeley DB, permitiendo que múltiples instancias de datos residan en un solo contenedor físico de ZFS, optimizando así el uso de descriptores de archivos y memoria caché.24

## **El Compilador de Control Language (CL)**

El lenguaje de control (CL) es el pegamento que une los componentes del sistema. Linux/400 incluye un compilador de CL que traduce el código fuente directamente en ejecutables nativos de Linux (ELF).13 A diferencia de los scripts interpretados de Bash, los programas CL de Linux/400 se compilan para garantizar que todas las referencias a objetos y tipos se validen antes de la ejecución.9

### **Generación de Código y Mapeo de Comandos**

El compilador realiza un análisis léxico de los comandos de CL y los transforma en llamadas a una biblioteca de sistema escrita en C/C++.28 Por ejemplo, el comando SNDPGMMSG (Send Program Message) se traduce en una llamada a una función de la API de Linux/400 que gestiona la cola de mensajes del trabajo actual, mientras que DLTOBJ invoca las rutinas de ZFS para la eliminación segura del objeto y sus metadatos.29

El proceso de compilación sigue estas etapas:

1. **Validación de Sintaxis**: Basada en las definiciones de comandos (\*CMD) almacenadas en la biblioteca del sistema QSYS.30  
2. **Resolución de Tipos**: El compilador verifica que los objetos referenciados existan en la lista de bibliotecas (\*LIBL) definida durante la compilación.1  
3. **Generación de C Intermedio**: Los comandos se expanden en estructuras C que manejan el control de errores y la gestión de punteros etiquetados.29  
4. **Compilación Nativa**: Se utiliza un backend de GCC o LLVM para generar el binario ELF, asegurando que el código sea compatible con el cargador de Linux/400 y el sistema de etiquetas de memoria.13

## **Gestión de Trabajos y Subsistemas: QINTER y QBATCH**

Linux/400 emula la gestión de carga de trabajo de OS/400 mediante el uso de Grupos de Control (cgroups v2) y systemd, pero respetando la jerarquía de memoria y procesos de Linux.34 Los subsistemas se implementan como "Slices" de systemd, lo que permite una partición rígida pero dinámica de los recursos de CPU, memoria y E/S de disco.37

### **El Subsistema Interactivo (QINTER)**

QINTER está diseñado para manejar todas las sesiones de terminal SSH. Cuando un usuario se conecta, el sistema asigna el proceso de la shell de Linux/400 (el menú TUI) a qinter.slice.37 Este slice está configurado con una prioridad de CPU más alta y límites de memoria que garantizan la interactividad, incluso bajo carga pesada del sistema.34

### **El Subsistema de Lote (QBATCH)**

Los trabajos enviados mediante SBMJOB se dirigen a qbatch.slice. Este subsistema está optimizado para el rendimiento a través del tiempo, permitiendo que los trabajos consuman ciclos de CPU sobrantes sin afectar la latencia de los usuarios interactivos.38 Se utiliza el controlador de CPU de cgroups v2 para definir pesos (cpu.weight) que favorecen a QINTER sobre QBATCH en una relación de, por ejemplo, 80:20 cuando hay contención.34

| Parámetro cgroup v2 | Configuración QINTER | Configuración QBATCH | Justificación Técnica |
| :---- | :---- | :---- | :---- |
| cpu.weight | 800 (Alta prioridad) | 100 (Baja prioridad) | Prioriza la respuesta del teclado y menús.34 |
| memory.low | Reservado para SO | Mínimo | Protege la memoria de sesión contra el swapping.36 |
| io.weight | 600 | 200 | Evita que las escrituras masivas de DB bloqueen el terminal.40 |
| tasks.max | 10 por usuario | Ilimitado (según pool) | Previene ataques de denegación de servicio por procesos.34 |

## **Interfaz de Usuario: SSH y menús TUI en UTF-8**

Una desviación crítica del modelo tradicional es la eliminación completa de EBCDIC y del protocolo 5250, adoptando en su lugar UTF-8 y SSH.41 Sin embargo, para mantener el modo de trabajo clásico, el sistema presenta un menú basado en texto (TUI) utilizando la biblioteca Ncurses tan pronto como se establece la conexión SSH.42

### **El Menú Principal de Linux/400**

La shell por defecto de los usuarios de Linux/400 no es Bash, sino un programa TUI que recrea la pantalla de "IBM i Main Menu".43 Este entorno proporciona:

* **Línea de Comandos**: En la parte inferior, permitiendo la entrada directa de comandos CL compilados.  
* **Soporte para Teclas de Función**: Mapeo de F3 (Salir), F4 (Prompt), F12 (Cancelar) mediante secuencias de escape ANSI estándar compatibles con cualquier cliente SSH moderno.42  
* **Codificación Universal**: El uso de UTF-8 permite que los nombres de objetos y los datos de la base de datos se integren fácilmente con el ecosistema global de Linux, eliminando los problemas de CCSID (Coded Character Set Identifier).41

### **Conectividad y Seguridad**

Al utilizar SSH como el único método de transporte, Linux/400 hereda todas las capacidades de seguridad modernas, incluyendo la autenticación por llave pública y el cifrado fuerte, mientras presenta una interfaz que es instantáneamente reconocible para un administrador de sistemas de gama media.43 El sistema de perfiles de usuario (\*USRPRF) se sincroniza con los usuarios de Linux (/etc/passwd), pero añade la capa de objetos de Linux/400 para gestionar las autorizaciones específicas de bibliotecas y comandos de forma centralizada en ZFS.2

## **Implementación de Colas de Datos y Otros Objetos**

Más allá de los archivos y programas, Linux/400 debe soportar objetos de comunicación interna como las colas de datos (\*DTAQ). Estos se implementan como archivos de Berkeley DB configurados en modo Queue, lo que permite operaciones de entrada y salida atómicas de registros de longitud fija o variable.24 Al estar encapsuladas en ZFS, estas colas son persistentes a través de reinicios y pueden ser replicadas o snapshotteadas como cualquier otro objeto del sistema.4

El wrapper de Linux/400 asegura que las llamadas de sistema de Linux, como sendmsg o write, se traduzcan en operaciones de Berkeley DB cuando se dirigen a una ruta dentro de la jerarquía /QSYS.LIB.46 Este enfoque de "wrapper sobre archivos" garantiza que, aunque el almacenamiento físico sea un archivo en Linux, la semántica de acceso sea estrictamente la de un objeto de OS/400.2

## **Conclusión y Visión Futura**

Linux/400 representa un cambio de paradigma en el diseño de sistemas operativos. Al combinar la agilidad y el hardware-awareness del núcleo Linux con el modelo mental de gestión de objetos y bases de datos integradas de OS/400, se crea una plataforma que es superior a sus partes constituyentes.2 La eliminación de EBCDIC y TIMI moderniza la arquitectura, mientras que el uso de punteros etiquetados de 64 bits y ZFS proporciona una seguridad y rendimiento que los sistemas tradicionales de Unix no pueden alcanzar de forma nativa.3

La viabilidad de Linux/400 reside en su fidelidad al "modo de trabajo" del administrador de AS/400, permitiendo que la transición a una infraestructura Linux sea invisible desde el punto de vista operativo, pero transformadora desde el punto de vista de la ingeniería y la escalabilidad.1 Con una base de datos integrada (BDB), un sistema de archivos de objetos (ZFS) y una gestión de recursos moderna (cgroups v2), Linux/400 está posicionado como el sucesor lógico para cargas de trabajo críticas de negocio que requieren la estabilidad de un sistema de gama media con la flexibilidad de la nube.2

#### **Works cited**

1. Geeking Out On IBM i \- Part 1 | GRIMM Cyber R\&D, accessed March 30, 2026, [https://grimmcyber.com/geeking-out-on-ibm-i-part-1/](https://grimmcyber.com/geeking-out-on-ibm-i-part-1/)  
2. IBM i (OS/400) The Database Operating System \- OSAdmins, accessed March 30, 2026, [https://osadmins.com/en/ibm-i-os-400-the-database-operating-system/](https://osadmins.com/en/ibm-i-os-400-the-database-operating-system/)  
3. IBM i: An Unofficial Introduction \- devever, accessed March 30, 2026, [https://www.devever.net/\~hl/f/as400guide.pdf](https://www.devever.net/~hl/f/as400guide.pdf)  
4. Introducing ZFS Properties, accessed March 30, 2026, [https://docs.oracle.com/cd/E19253-01/819-5461/gazss/index.html](https://docs.oracle.com/cd/E19253-01/819-5461/gazss/index.html)  
5. Berkeley DB Reference Guide: Secondary indices, accessed March 30, 2026, [https://web.stanford.edu/class/cs276a/projects/docs/berkeleydb/ref/am/second.html](https://web.stanford.edu/class/cs276a/projects/docs/berkeleydb/ref/am/second.html)  
6. ncurses \- Alpine Linux packages, accessed March 30, 2026, [https://pkgs.alpinelinux.org/package/edge/main/x86/ncurses](https://pkgs.alpinelinux.org/package/edge/main/x86/ncurses)  
7. Compile custom kernel and modules in Alpine Linux, accessed March 30, 2026, [https://unix.stackexchange.com/questions/687331/compile-custom-kernel-and-modules-in-alpine-linux](https://unix.stackexchange.com/questions/687331/compile-custom-kernel-and-modules-in-alpine-linux)  
8. How do IBM i and Multics objects differ from UNIX and Windows files? \- Quora, accessed March 30, 2026, [https://www.quora.com/How-do-IBM-i-and-Multics-objects-differ-from-UNIX-and-Windows-files](https://www.quora.com/How-do-IBM-i-and-Multics-objects-differ-from-UNIX-and-Windows-files)  
9. How is IBM I better than linux? : r/IBMi \- Reddit, accessed March 30, 2026, [https://www.reddit.com/r/IBMi/comments/pbnumw/how\_is\_ibm\_i\_better\_than\_linux/](https://www.reddit.com/r/IBMi/comments/pbnumw/how_is_ibm_i_better_than_linux/)  
10. Creating and initializing zFS metadata catalogs \- IBM, accessed March 30, 2026, [https://www.ibm.com/docs/en/idr/11.3.3?topic=caccatut-creating-initializing-zfs-metadata-catalogs](https://www.ibm.com/docs/en/idr/11.3.3?topic=caccatut-creating-initializing-zfs-metadata-catalogs)  
11. Chris's Wiki :: blog/solaris/ZFSMetadataMeaning, accessed March 30, 2026, [https://utcc.utoronto.ca/\~cks/space/blog/solaris/ZFSMetadataMeaning](https://utcc.utoronto.ca/~cks/space/blog/solaris/ZFSMetadataMeaning)  
12. What does the ZFS Metadata Special Device do? · openzfs zfs · Discussion \#14542 \- GitHub, accessed March 30, 2026, [https://github.com/openzfs/zfs/discussions/14542](https://github.com/openzfs/zfs/discussions/14542)  
13. ELF from scratch \- conradk.com, accessed March 30, 2026, [https://www.conradk.com/elf-from-scratch/](https://www.conradk.com/elf-from-scratch/)  
14. The 101 of ELF files on Linux: Understanding and Analysis, accessed March 30, 2026, [https://linux-audit.com/elf-binaries-on-linux-understanding-and-analysis/](https://linux-audit.com/elf-binaries-on-linux-understanding-and-analysis/)  
15. AArch64 TAGGED ADDRESS ABI — The Linux Kernel documentation, accessed March 30, 2026, [https://docs.kernel.org/arch/arm64/tagged-address-abi.html](https://docs.kernel.org/arch/arm64/tagged-address-abi.html)  
16. Tagged pointers in action : r/C\_Programming \- Reddit, accessed March 30, 2026, [https://www.reddit.com/r/C\_Programming/comments/1mh5ree/tagged\_pointers\_in\_action/](https://www.reddit.com/r/C_Programming/comments/1mh5ree/tagged_pointers_in_action/)  
17. Enable intel LAM in linux, accessed March 30, 2026, [https://lpc.events/event/11/contributions/1010/attachments/875/1679/LAM-LPC-2021.pdf](https://lpc.events/event/11/contributions/1010/attachments/875/1679/LAM-LPC-2021.pdf)  
18. Support for Intel's Linear Address Masking \- LWN.net, accessed March 30, 2026, [https://lwn.net/Articles/902094/](https://lwn.net/Articles/902094/)  
19. Memory/Pointer Tagging In IsoAlloc \- Root Cause, accessed March 30, 2026, [https://struct.github.io/pointer\_tagging.html](https://struct.github.io/pointer_tagging.html)  
20. Tagged pointers | Android Open Source Project, accessed March 30, 2026, [https://source.android.com/docs/security/test/tagged-pointers](https://source.android.com/docs/security/test/tagged-pointers)  
21. Managing AS/400 Physical and Logical Files | PDF | Command Line Interface \- Scribd, accessed March 30, 2026, [https://www.scribd.com/document/760698553/Bci433-Lecture-5](https://www.scribd.com/document/760698553/Bci433-Lecture-5)  
22. Physical and Logical Files \- Programmers.io, accessed March 30, 2026, [https://programmers.io/ibmi-ebooks/physical-and-logical-files/](https://programmers.io/ibmi-ebooks/physical-and-logical-files/)  
23. What is a physical and logical file in the AS400 Database \- Nick Litten, accessed March 30, 2026, [https://www.nicklitten.com/what-is-the-difference-between-a-physical-and-logical-file-on-the-as400/](https://www.nicklitten.com/what-is-the-difference-between-a-physical-and-logical-file-on-the-as400/)  
24. Berkeley DB Reference Guide: Opening multiple databases in a single file, accessed March 30, 2026, [https://web.stanford.edu/class/cs276a/projects/docs/berkeleydb/ref/am/opensub.html](https://web.stanford.edu/class/cs276a/projects/docs/berkeleydb/ref/am/opensub.html)  
25. AS/400 DB2 Logical File vs Table Index \- Stack Overflow, accessed March 30, 2026, [https://stackoverflow.com/questions/7045254/as-400-db2-logical-file-vs-table-index](https://stackoverflow.com/questions/7045254/as-400-db2-logical-file-vs-table-index)  
26. Berkeley DB Reference Guide: Opening multiple databases in a single file, accessed March 30, 2026, [https://apps.state.or.us/tech/berkeleyDB/ref/am/opensub.html](https://apps.state.or.us/tech/berkeleyDB/ref/am/opensub.html)  
27. A Whirlwind Tutorial on Creating Really Teensy ELF Executables for Linux \- in4k, accessed March 30, 2026, [https://in4k.github.io/html\_articles/A%20Whirlwind%20Tutorial%20on%20Creating%20Really%20Teensy%20ELF%20Executables%20for%20Linux.html](https://in4k.github.io/html_articles/A%20Whirlwind%20Tutorial%20on%20Creating%20Really%20Teensy%20ELF%20Executables%20for%20Linux.html)  
28. CL commands by product \- IBM, accessed March 30, 2026, [https://www.ibm.com/docs/en/i/7.5?topic=language-cl-commands-by-product](https://www.ibm.com/docs/en/i/7.5?topic=language-cl-commands-by-product)  
29. Using the C Wrapper \- IBM, accessed March 30, 2026, [https://www.ibm.com/docs/en/entirex/11.0.0?topic=wrapper-using-c](https://www.ibm.com/docs/en/entirex/11.0.0?topic=wrapper-using-c)  
30. CL command finder \- IBM, accessed March 30, 2026, [https://www.ibm.com/docs/ssw\_ibm\_i\_74/clfinder/finder30.htm](https://www.ibm.com/docs/ssw_ibm_i_74/clfinder/finder30.htm)  
31. IBM i Database Basics: Understanding Libraries, Physical Files, Members, Record Formats and Fields \- Nick Litten, accessed March 30, 2026, [https://www.nicklitten.com/ibm-i-database-basics-understanding-libraries-physical-files-members-record-formats-and-fields/](https://www.nicklitten.com/ibm-i-database-basics-understanding-libraries-physical-files-members-record-formats-and-fields/)  
32. Programming ILE C/C++ Runtime Library Functions \- IBM, accessed March 30, 2026, [https://www.ibm.com/docs/es/ssw\_ibm\_i\_74/rtref/sc415607.pdf](https://www.ibm.com/docs/es/ssw_ibm_i_74/rtref/sc415607.pdf)  
33. How to compile c program into elf format? \[closed\] \- Stack Overflow, accessed March 30, 2026, [https://stackoverflow.com/questions/46552197/how-to-compile-c-program-into-elf-format](https://stackoverflow.com/questions/46552197/how-to-compile-c-program-into-elf-format)  
34. systemd.resource-control \- Freedesktop.org, accessed March 30, 2026, [https://www.freedesktop.org/software/systemd/man/systemd.resource-control.html](https://www.freedesktop.org/software/systemd/man/systemd.resource-control.html)  
35. About cgroup v2 \- Kubernetes, accessed March 30, 2026, [https://kubernetes.io/docs/concepts/architecture/cgroups/](https://kubernetes.io/docs/concepts/architecture/cgroups/)  
36. How cgroups v2 Helps Control System Resources: Concepts and Principles \- Medium, accessed March 30, 2026, [https://medium.com/@rlavmdek/how-cgroups-v2-helps-control-system-resources-concepts-and-principles-1ad18b5e9d7b](https://medium.com/@rlavmdek/how-cgroups-v2-helps-control-system-resources-concepts-and-principles-1ad18b5e9d7b)  
37. Using systemd to Manage cgroups v2 \- Oracle Help Center, accessed March 30, 2026, [https://docs.oracle.com/en/operating-systems/oracle-linux/9/systemd/SystemdMngCgroupsV2.html](https://docs.oracle.com/en/operating-systems/oracle-linux/9/systemd/SystemdMngCgroupsV2.html)  
38. Control Group v2 — The Linux Kernel documentation, accessed March 30, 2026, [https://www.kernel.org/doc/html/v4.18/admin-guide/cgroup-v2.html](https://www.kernel.org/doc/html/v4.18/admin-guide/cgroup-v2.html)  
39. Solved: AS400 Sub System \- Experts Exchange, accessed March 30, 2026, [https://www.experts-exchange.com/questions/21547737/AS400-Sub-System.html](https://www.experts-exchange.com/questions/21547737/AS400-Sub-System.html)  
40. Exploring Cgroups v1 and Cgroups v2: Understanding the Evolution of Resource Control, accessed March 30, 2026, [https://dohost.us/index.php/2025/10/11/exploring-cgroups-v1-and-cgroups-v2-understanding-the-evolution-of-resource-control/](https://dohost.us/index.php/2025/10/11/exploring-cgroups-v1-and-cgroups-v2-understanding-the-evolution-of-resource-control/)  
41. list of objects to a file on ifs using Unix ls in IBM i qsh \- Stack Overflow, accessed March 30, 2026, [https://stackoverflow.com/questions/52470075/list-of-objects-to-a-file-on-ifs-using-unix-ls-in-ibm-i-qsh](https://stackoverflow.com/questions/52470075/list-of-objects-to-a-file-on-ifs-using-unix-ls-in-ibm-i-qsh)  
42. Building TUIs in a few lines of code with ncurses \- Zuhaitz, accessed March 30, 2026, [https://zuhaitz.dev/posts/ncurses-terminal-ui/](https://zuhaitz.dev/posts/ncurses-terminal-ui/)  
43. Creating a Menu after SSH login \- LinuxQuestions.org, accessed March 30, 2026, [https://www.linuxquestions.org/questions/linux-server-73/creating-a-menu-after-ssh-login-532675/](https://www.linuxquestions.org/questions/linux-server-73/creating-a-menu-after-ssh-login-532675/)  
44. Offer or push menu to ssh users \- Unix & Linux Stack Exchange, accessed March 30, 2026, [https://unix.stackexchange.com/questions/250230/offer-or-push-menu-to-ssh-users](https://unix.stackexchange.com/questions/250230/offer-or-push-menu-to-ssh-users)  
45. So you want to make a TUI, accessed March 30, 2026, [https://p.janouch.name/article-tui.html](https://p.janouch.name/article-tui.html)  
46. Develop your own filesystem with FUSE, accessed March 30, 2026, [https://developer.ibm.com/articles/l-fuse/](https://developer.ibm.com/articles/l-fuse/)  
47. Filesystem in Userspace (aka FUSE) | by Alperen Bayramoğlu | Medium, accessed March 30, 2026, [https://alperenbayramoglu2.medium.com/filesystem-in-userspace-aka-fuse-3336d4b7364d](https://alperenbayramoglu2.medium.com/filesystem-in-userspace-aka-fuse-3336d4b7364d)  
48. How to emulate or virtualize an old AS/400? : r/sysadmin \- Reddit, accessed March 30, 2026, [https://www.reddit.com/r/sysadmin/comments/45cwld/how\_to\_emulate\_or\_virtualize\_an\_old\_as400/](https://www.reddit.com/r/sysadmin/comments/45cwld/how_to_emulate_or_virtualize_an_old_as400/)