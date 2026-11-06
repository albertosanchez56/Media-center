# Media Center — Decisiones

Estado del documento: registro vivo, con la segunda revisión aplicada.  
Última actualización: 2026-11-04.

Una decisión confirmada se respeta al implementar. Una decisión pendiente no se da por cerrada dentro del código ni de otros documentos. Si dos documentos discrepan, se anota aquí antes de implementar.

El esquema de [DATABASE.md](DATABASE.md) no queda aprobado por las decisiones de modelo de esta revisión. Esas decisiones orientan la propuesta; las tablas se crearán cuando DP-17 se cierre.

## Cómo se registra una decisión

Cada entrada indica fecha, estado y consecuencias. Al resolver una pendiente, se cambia su estado, se escribe la opción elegida y se ajustan los documentos afectados.

Estados: confirmada, pendiente, prevista, descartada.

## Decisiones confirmadas

### DC-01 Aplicación local de escritorio para Windows

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** La primera versión es una aplicación de escritorio para Windows. Funciona en local, sin cuentas y sin servidores externos.
- **Consecuencias:** No hay registro de usuarios, ni API remota, ni sincronización. El streaming móvil queda fuera.

### DC-02 Pila de la primera versión

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** Interfaz con Angular y TypeScript. Escritorio con Tauri 2 y Rust. Biblioteca en SQLite. Descargas autorizadas con yt-dlp y FFmpeg.
- **Consecuencias:** La lógica de archivos, base de datos, procesos y reproducción se concentra en Rust. Angular presenta estado y envía acciones.

### DC-03 Spring Boot queda fuera

- **Fecha:** 2026-11-04
- **Estado:** descartada para este proyecto
- **Decisión:** La primera versión no usa Spring Boot ni otro servidor de aplicación.
- **Consecuencias:** No se añade un backend Java ni un puerto de servicio para la interfaz.

### DC-04 Tres bibliotecas y clasificación elegida por la persona

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** Las bibliotecas son Música, Vídeos y Películas. La persona elige la biblioteca al importar. Un MP4 puede entrar en cualquiera de las tres.
- **Consecuencias:** El contenedor del archivo no asigna la biblioteca. La ficha guarda la biblioteca elegida. La importación confirmada crea una ficha, no varias.

### DC-05 Una sola reproducción, independiente de la pantalla visible

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** Hay una reproducción activa. Cambiar de sección no la detiene. El vídeo asociado a una canción forma parte de esa misma reproducción: aporta imagen, no un segundo audio.
- **Consecuencias:** El estado de reproducción vive fuera de la ruta visible de Angular. Iniciar otro elemento sustituye al actual.

### DC-06 Los metadatos de trabajo viven en SQLite

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** La biblioteca se guarda en SQLite. Las imágenes se guardan en disco y la base conserva la referencia. Los cambios de la persona usuaria prevalecen. Editar la ficha no modifica el archivo original.
- **Consecuencias:** Hace falta recordar qué campos están editados en cada ficha. Releer un archivo no puede restaurar por encima de esos campos. La escritura de etiquetas, si llega a existir, será otra acción.

### DC-07 Los archivos físicos no se borran solos

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** La aplicación no elimina archivos de la persona usuaria sin autorización. Si un archivo desaparece, las fichas se conservan.
- **Consecuencias:** La reconciliación cambia el estado del archivo físico. No usa la ausencia del archivo como orden de borrado. La recuperación de un movimiento tampoco borra el original cuando el resultado es incierto.

### DC-08 Descargas limitadas a un uso autorizado de YouTube

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** El descargador acepta enlaces compatibles de YouTube y existe para usos permitidos por los permisos, los derechos y las condiciones aplicables. Puede descargar un elemento o una playlist, solo audio o audio y vídeo, con vista previa, miniatura, cola, progreso, cancelación, reintentos y detección de duplicados.
- **Consecuencias:** Otros sitios que yt-dlp sea capaz de leer no se ofrecen en la primera versión. La propuesta de lotes y posiciones organiza esa cola. La regla concreta de duplicado sigue pendiente en DP-06.

### DC-09 Migraciones versionadas

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** Todo cambio del esquema SQLite se aplica con migraciones versionadas.
- **Consecuencias:** No se altera a mano una base ya creada ni se reescribe una migración aplicada. Categorías y colecciones, si se confirman, llegan en una migración posterior.

### DC-10 El material personal no entra en Git

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** Bases de datos locales, bibliotecas, descargas y archivos multimedia de uso personal permanecen fuera del repositorio.
- **Consecuencias:** Las rutas de datos de la aplicación tienen que estar cubiertas por `.gitignore` o quedar fuera del directorio del proyecto.

### DC-11 libmpv es el motor previsto

- **Fecha:** 2026-11-04
- **Estado:** confirmada en la elección del motor; la integración concreta sigue abierta
- **Decisión:** El motor multimedia previsto es libmpv.
- **Consecuencias:** No se parte de otro reproductor como base. La forma de incrustarlo se estudia dentro de Media Center y se documenta en la tarea F1.5 antes de las funciones avanzadas. Hasta ese cierre, DP-01 sigue pendiente. No se mantiene abierta de forma indefinida ni se resuelve en un proyecto de pruebas aparte.

### DC-12 Extracción de portadas incrustadas

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** Si el archivo trae una portada incrustada, la aplicación guarda una copia en la carpeta de imágenes de Media Center y la referencia desde la ficha. El archivo multimedia no se modifica. Una imagen personalizada se guarda como otro archivo de esa ficha y no sustituye ni elimina la copia extraída.
- **Consecuencias:** Al importar hay una imagen de origen `embedded` y, si la persona cambia la portada, otra de origen `user`. La seleccionada puede cambiar. La herramienta que realiza la lectura sigue pendiente en DP-10. Esta decisión sustituye la antigua propuesta CP-01.

### DC-13 La ficha es dueña de sus datos y no del archivo compartido

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** Los metadatos personalizados y las imágenes pertenecen a cada ficha. El modelo separa la identidad del archivo físico de las fichas y de sus vínculos. Eliminar una ficha no borra un archivo físico que otra ficha utiliza.
- **Consecuencias:** La propuesta de `stored_files`, `media_items` e `item_files` refleja esa separación. La función de crear varias fichas para el mismo archivo sigue pendiente en DP-18. Aunque una ficha sea la única que usa un archivo, DC-07 impide borrarlo sin autorización.

### DC-14 Copia y movimiento seguros

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** Toda operación queda registrada antes de alterar el origen. En el mismo volumen se usa un movimiento directo cuando el diario ya está confirmado, el nombre de destino está libre y el renombrado puede recuperarse. Entre volúmenes distintos se copia a un temporal, se verifica la copia, se confirma el alta y solo entonces se puede eliminar el original. Un nombre ocupado no se sobrescribe. Sin espacio suficiente la operación no deja el original a medias. Con incertidumbre, el original no se elimina de forma automática.
- **Consecuencias:** El procedimiento y la recuperación están descritos en [ARCHITECTURE.md](ARCHITECTURE.md). La tabla `file_operations` forma parte de la propuesta de datos y todavía no está aprobada. El algoritmo de huella no es obligatorio para la verificación por tamaño y sigue pendiente en DP-05.

### DC-15 Momento de la integración de libmpv

- **Fecha:** 2026-11-04
- **Estado:** confirmada
- **Decisión:** El trabajo de integración empieza al comienzo del desarrollo, en la ventana de Media Center, sin un proyecto de pruebas independiente. Antes de desarrollar las funciones avanzadas del reproductor se elige y se documenta una integración concreta.
- **Consecuencias:** Las tareas F1.4 y F1.5 de [ROADMAP.md](ROADMAP.md) ocupan ese lugar. F1.4 explora las opciones en la aplicación. F1.5 cierra DP-01. Ampliar el vídeo, el minirreproductor avanzado, los subtítulos, las pistas y el vídeo asociado esperan ese cierre.

### DC-16 Categorías y colecciones se añaden sin reescribir tablas

- **Fecha:** 2026-11-04
- **Estado:** confirmada como forma de evolución; las funciones siguen previstas
- **Decisión:** Cuando se confirmen las reglas, la migración correspondiente solo creará tablas nuevas y sus vínculos con las fichas. No añadirá columnas de categoría o de colección a las tablas iniciales.
- **Consecuencias:** DP-02 y DP-03 siguen pendientes. El boceto de tablas futuras de [DATABASE.md](DATABASE.md) no forma parte del esquema inicial ni está aprobado.

## Decisiones pendientes

### DP-01 Integración concreta de libmpv

- **Fecha:** 2026-11-04
- **Estado:** pendiente. Debe cerrarse en F1.5, antes de las funciones avanzadas del reproductor
- **Bloquea:** F7.4, F7.6, F8.2 y F8.3. No bloquea el esqueleto ni el estudio temprano F1.4
- **Qué hay que elegir:** Cómo se une libmpv a la ventana de Tauri 2.
- **Opciones a examinar, sin preferencia cerrada:**
  - Renderizar libmpv en una superficie nativa coordinada con la ventana.
  - Controlar un proceso de mpv mediante su IPC.
  - Otra integración que cumpla los mismos requisitos de producto.
- **La opción elegida tiene que permitir:**
  - Una sola reproducción visible para la persona usuaria.
  - Vídeo en el panel derecho, ampliado y minimizado, sin reiniciar.
  - Audio de la canción junto con otro archivo de vídeo en silencio.
  - Pausa, reanudación y salto de posición aplicados a las dos fuentes.
  - Subtítulos, pistas de audio y pantalla completa.
- **Consecuencias de dejarla abierta durante la planificación:** Se puede documentar el producto y, al empezar el desarrollo, construir la ventana y probar las opciones dentro de ella. No se implementan las funciones avanzadas sobre una integración supuesta. La decisión no permanece abierta después de F1.5.
- **Observaciones de F1.4 (2026-11-06), sin elegir opción:** Se probaron dos integraciones, con el detalle y la evidencia en [PROGRESS.md](PROGRESS.md):
  - A: `libmpv-2.dll` LGPL cargada dentro del proceso de Media Center.
  - B: `mpv.exe` en otro proceso, controlado por JSON IPC sobre una tubería con nombre.
  - Las dos dibujan en la misma ventana hija Win32, colocada sobre un recuadro reservado del HTML. En las pruebas se comportaron igual: reproducción, vídeo asociado en silencio con desfase de 0,000 s tras pausa y saltos, tamaños sin reiniciar, recorte al desplazar, rueda y controles HTML con huecos.
  - Diferencias observadas: B depende de un ejecutable aparte, de una tubería por sesión y de un proceso que hay que terminar al detener. La compilación oficial de mpv.exe es GPL; la DLL de A es LGPL.
  - Limitaciones comunes de la superficie nativa: va por encima de WebView2 y hay que recortarla con una región; sigue al HTML con un pequeño retraso; los controles HTML encima del vídeo necesitan huecos; la rueda se reenvía con un paso fijo.
  - Los saltos tienen que ser `relative+exact`. Con `relative`, el audio de una canción con vídeo asociado se queda retenido hasta el siguiente fotograma clave del vídeo.
  - Con la presentación «flip» de D3D11 el recorte se respetó en el equipo de prueba. No se ha comprobado en otros equipos y se trata como opción experimental.
  - Al cerrar Media Center con una reproducción activa, ninguna de las dos deja procesos propios ni tuberías.
  - **Candidata provisional: A**, desde el 2026-11-06. **B queda documentada como alternativa.** No es una elección: DP-01 sigue pendiente hasta F1.5. F1.4 está hecha y el código de las dos integraciones se conserva en el diagnóstico de desarrollo para F1.5.
  - Pendiente de comprobar para cerrar DP-01: subtítulos, pistas de audio, pantalla completa, varios monitores y otras escalas. Las pruebas y sus dependencias están en [PROGRESS.md](PROGRESS.md), en «Pruebas pendientes para las funciones avanzadas».

### DP-02 Reglas de las categorías personalizadas

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F6.4
- **Qué hay que elegir:**
  - Si una categoría pertenece a una biblioteca o sirve para todas.
  - Si una ficha puede tener varias categorías.
  - Si las categorías forman una lista plana o una jerarquía.
  - Quién las crea y si se pueden renombrar o eliminar sin borrar fichas.
- **Consecuencias de dejarla abierta:** El esquema inicial no incluye tablas de categorías. DC-16 reserva una migración posterior de tablas nuevas.

### DP-03 Significado de una colección

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F6.3
- **Qué hay que elegir:** En qué se diferencia una colección de una playlist y de una categoría. Por ejemplo, si es un grupo sin orden de reproducción, una saga de películas u otra agrupación.
- **Consecuencias de dejarla abierta:** El esquema inicial no incluye tablas de colecciones. DC-16 reserva una migración posterior de tablas nuevas.

### DP-04 Archivos fuera de la carpeta administrada

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F9.3
- **Qué hay que elegir:** Si, además de copiar y de mover, la importación puede limitarse a recordar la ruta original.
- **Opciones:**
  - Solo copiar o mover a la carpeta administrada.
  - Permitir también una referencia externa.
- **Consecuencias de dejarla abierta:** La propuesta de `stored_files` guarda una ruta relativa a la carpeta administrada. Copiar y mover sí pueden implementarse con DC-14. Esta decisión es distinta de DP-18: DP-18 trata varias fichas de un archivo ya administrado; DP-04 trata un archivo que permanece fuera.

### DP-05 Identidad de un archivo movido o renombrado

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F9.2
- **Qué hay que elegir:** Qué señales permiten afirmar que un archivo en una ruta nueva es el mismo que ya tenía registro.
- **Señales posibles, combinables:** tamaño, fecha, hash completo, hash parcial, ruta anterior.
- **Consecuencias de dejarla abierta:** Se puede marcar un archivo como no disponible cuando falta en su ruta. No se reasigna la ruta de forma automática. La verificación de una copia o un movimiento usa el tamaño anotado y no espera a esta decisión.

### DP-06 Regla de duplicados en descargas

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F10.5
- **Qué hay que elegir:** Cuándo una descarga se considera repetida.
- **Señales posibles:** URL de origen, identificador del vídeo, hash del archivo resultante, misma biblioteca o cualquier biblioteca.
- **Consecuencias de dejarla abierta:** La columna `source_key` de la propuesta queda preparada y sin semántica definitiva. El lote y la posición no deciden el duplicado. El resto de la cola puede diseñarse sin esta regla.

### DP-07 Vídeo asociado más corto que la canción

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F8.3
- **Qué hay que elegir:** Si el vídeo se reproduce en bucle, si se queda en el último fotograma o si cada ficha elige el comportamiento.
- **Consecuencias de dejarla abierta:** El requisito está contemplado y no se implementa un bucle por suposición. El audio de la canción sigue hasta el final en cualquiera de las opciones.

### DP-08 Preferencia entre portada, vídeo original y vídeo asociado

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** el detalle de F8.2
- **Qué hay que elegir:** Si la presencia de un vídeo asociado cambia sola la imagen mostrada, o si la ficha guarda una preferencia de la persona usuaria.
- **Consecuencias de dejarla abierta:** La columna `visual_preference` es una propuesta, no un campo aprobado. DC-12 ya fija que la portada extraída y la imagen personalizada conviven.

### DP-09 Dónde se guardan la base, los medios y las imágenes

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F2.1 y la documentación de instalación F12.4
- **Qué hay que elegir:** Directorio de datos de Windows, carpeta elegida por la persona usuaria, o una combinación.
- **Nota:** `.gitignore` excluye `library/`, `downloads/` y `data/` en la raíz del repositorio para no versionar material local. Eso no fija las rutas de la aplicación.
- **Consecuencias de dejarla abierta:** No se crea el archivo SQLite definitivo.

### DP-10 Herramienta de etiquetas

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F3.3 y condiciona F11.1
- **Qué hay que elegir:** Qué herramienta lee etiquetas y extrae portadas incrustadas sin modificar el archivo, y si esa misma herramienta sirve para la escritura opcional.
- **La elección tiene que cubrir:** lectura de los formatos musicales previstos, extracción de la imagen incrustada según DC-12 y un fallo controlado cuando el archivo no tenga etiquetas.
- **Consecuencias de dejarla abierta:** La importación puede copiar y registrar el archivo. El relleno automático de la ficha y la extracción esperan a esta decisión. El comportamiento de extracción ya está confirmado.

### DP-11 Escritura opcional de etiquetas

- **Fecha:** 2026-11-04
- **Estado:** prevista en el producto; pendiente en el detalle
- **Bloquea:** F11.1
- **Qué hay que elegir:** En qué formatos se ofrece, qué campos se escriben y cómo se confirma la modificación del archivo.
- **Consecuencias de dejarla abierta:** El comportamiento por defecto sigue siendo no modificar el archivo. La función no forma parte de la edición normal de la ficha ni de la extracción de portadas.

### DP-12 Campos de búsqueda, filtros, repetición y cola

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** el detalle de F4.2, F4.3 y F7.5
- **Qué hay que elegir:**
  - Campos que cubre el buscador.
  - Filtros además de la biblioteca y el favorito.
  - Modos de repetición: elemento, cola, ninguno u otros.
  - Si la cola y el modo aleatorio se guardan al cerrar la aplicación.
- **Consecuencias de dejarla abierta:** Se pueden listar bibliotecas y favoritos. El buscador y los modos de repetición esperan el detalle.

### DP-13 Reanudación fuera de las películas

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Qué hay que elegir:** Si los vídeos y las canciones también guardan la posición.
- **Consecuencias de dejarla abierta:** La reanudación automática se implementa para películas, que es el requisito confirmado.

### DP-14 Subtítulos

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Qué hay que elegir:** Si los subtítulos salen solo de pistas incrustadas, también de archivos externos, o de ambas fuentes.
- **Consecuencias de dejarla abierta:** La interfaz de reproducción reserva la selección de subtítulos. El origen concreto llega con la integración elegida en F1.5.

### DP-15 Convivencia de la barra musical y el minirreproductor

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** el detalle visual de F7.6
- **Qué hay que elegir:** Dónde se ve una película dentro de su sección, dónde queda el minirreproductor al salir, y qué ocurre con la barra inferior mientras suena algo que no es una canción.
- **Consecuencias de dejarla abierta:** El armazón de cuatro zonas puede construirse. El comportamiento del visionado de películas espera el diseño y la integración ya elegida.

### DP-16 Formatos y calidades ofrecidos en la descarga

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** el detalle de F10.3
- **Qué hay que elegir:** Qué contenedores y calidades muestra la interfaz para audio y para audio con vídeo.
- **Consecuencias de dejarla abierta:** yt-dlp y FFmpeg siguen siendo las herramientas. La lista visible no se inventa en la implementación.

### DP-17 Aprobación del esquema inicial

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F0.2 y F2.2
- **Qué hay que elegir:** Las respuestas a las preguntas finales de [DATABASE.md](DATABASE.md), incluida la aceptación explícita de archivos físicos separados, lotes, diario de operaciones y apariciones repetidas.
- **Consecuencias de dejarla abierta:** No hay tablas definitivas y no se genera la migración inicial. La segunda revisión de la documentación no cierra esta decisión.

### DP-18 Varias fichas para un mismo archivo físico

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** F3.5
- **Qué hay que elegir:** Si la persona usuaria puede crear otra ficha, en otra biblioteca, sobre un archivo que ya está en la carpeta administrada, sin duplicar el fichero.
- **Consecuencias de dejarla abierta:** La importación confirmada sigue creando una sola ficha. El modelo propuesto ya separa archivo, ficha y vínculo para no impedir la función. Los metadatos y las imágenes de cada ficha son independientes por DC-13.

### DP-19 Formato del instalador de Windows

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** el detalle de F12.1 y F12.4
- **Qué hay que elegir:** Qué paquete de Windows generará Tauri, entre los que el proyecto admita cuando exista.
- **Consecuencias de dejarla abierta:** La fase 12 sigue en la hoja de ruta. No se da por elegido un formato concreto.

### DP-20 Empaquetado de componentes externos

- **Fecha:** 2026-11-04
- **Estado:** pendiente
- **Bloquea:** el detalle de F12.3 y F12.4
- **Qué hay que elegir:** Si libmpv, FFmpeg y yt-dlp se incluyen en el instalador o se documentan como requisitos externos. La herramienta de etiquetas entra en la misma decisión cuando DP-10 la elija.
- **Consecuencias de dejarla abierta:** Las licencias se inventariarán cuando existan las dependencias. La guía de instalación no puede afirmar todavía si esas herramientas viajan dentro del paquete.

## Relación entre requisitos que pueden parecer incompatibles

Estas lecturas mantienen los requisitos confirmados sin cerrar las decisiones pendientes.

| Apariencia de conflicto | Lectura actual |
| --- | --- |
| Una sola reproducción y, a la vez, un vídeo asociado | El vídeo asociado no es otra reproducción. Es la imagen de la canción y entra en silencio |
| No modificar el archivo y, a la vez, poder escribir etiquetas | La edición de la ficha y la extracción de la portada no tocan el archivo. La escritura de etiquetas sería una acción distinta, todavía por detallar |
| Carpeta administrada y detección de cambios externos | Copiar o mover deja el archivo bajo la carpeta administrada. La persona u otro programa pueden renombrarlo, moverlo o borrarlo después. Referenciar una ruta exterior es DP-04 |
| Varias bibliotecas para un mismo archivo y una sola ficha al importar | DC-13 separa el archivo de las fichas. DC-04 crea una ficha en la importación. DP-18 decide si más adelante puede existir otra ficha sin copiar el archivo |
| Portada incrustada e imagen personalizada | DC-12 guarda las dos. La personalizada es la que se muestra cuando está seleccionada. La extraída permanece |
| Movimiento directo y no borrar el original con incertidumbre | El movimiento directo en el mismo volumen es un renombrado recuperable porque el diario existe antes. Entre volúmenes, el original solo se elimina tras verificar la copia y confirmar el alta. En `needs_review` no hay borrado automático |
| Sin servidores en la primera versión y streaming móvil en el futuro | El streaming pertenece a una ampliación posterior. El instalador de Windows no añade un servidor |
| libmpv elegido e integración sin definir | El motor está elegido. La técnica se elige en F1.5, al comienzo del desarrollo y antes de las funciones avanzadas. Hoy DP-01 sigue pendiente porque el desarrollo no ha empezado |

## Propuesta aceptada

### CP-01 Extraer la portada incrustada a la carpeta de imágenes

- **Fecha:** 2026-11-04
- **Estado:** aceptada y recogida en DC-12
- **Efecto:** Deja de ser una propuesta abierta. El archivo original no se modifica y la imagen personalizada permanece independiente.
