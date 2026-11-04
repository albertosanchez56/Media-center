# Media Center — Arquitectura

Estado del documento: propuesta de estructura, con la segunda revisión aplicada.  
Última actualización: 2026-11-04.

Este documento reparte responsabilidades entre la interfaz, la lógica y los datos. El procedimiento de copia y movimiento y la extracción de portadas están confirmados en [DECISIONS.md](DECISIONS.md). La integración concreta de libmpv, la herramienta de etiquetas y el esquema de SQLite siguen abiertos. El esquema propuesto está en [DATABASE.md](DATABASE.md) y no está aprobado.

## Vista general

Media Center es un proceso de escritorio. Angular dibuja la interfaz. Rust, a través de Tauri 2, administra la ventana, el sistema de archivos, SQLite, los procesos externos y el motor multimedia.

```
Interfaz Angular
    comandos y eventos de Tauri
Servicios de aplicación en Rust
    repositorio SQLite
    archivos de la carpeta administrada
    imágenes en disco
    diario de operaciones de archivo
    adaptador del motor multimedia
    adaptador de descarga
    adaptador de etiquetas
```

La interfaz pide acciones y muestra estado. No abre la base de datos, no mueve archivos y no lanza yt-dlp, FFmpeg ni el motor de vídeo por su cuenta.

## Capas

| Capa | Responsabilidad | Tecnología prevista |
| --- | --- | --- |
| Interfaz | Navegación, listas, formularios, reproductor visible y progreso | Angular y TypeScript |
| Aplicación | Casos de uso: importar, editar, reproducir, descargar, reconciliar, recuperar operaciones | Rust |
| Datos | Fichas, archivos físicos, vínculos, playlists, lotes de descarga, diario y ajustes | SQLite, mediante un módulo de acceso usado solo por la capa de aplicación |
| Archivos | Medios administrados e imágenes | Sistema de archivos |
| Adaptadores | Motor, descarga y etiquetas | Interfaces internas. La implementación del motor se elige al comienzo del desarrollo |

La separación sigue el criterio del proyecto: interfaz, lógica de negocio y acceso a datos en módulos distintos.

## Distribución prevista del repositorio

La tarea F1.1 creó esta distribución en la raíz del repositorio:

```
src/            Interfaz Angular
public/         Archivos estáticos de Angular
src-tauri/      Aplicación Rust y configuración de Tauri
docs/           Documentación de producto
```

Angular compila en `dist/media-center/browser`. Tauri carga esa misma ruta (`frontendDist`) en la versión compilada, y `http://localhost:4200` (`devUrl`) durante el desarrollo. No se crea un segundo proyecto para probar libmpv: la integración se estudia en esta misma aplicación.

Las carpetas `library/`, `downloads/` y `data/` de la raíz están ignoradas por Git para no versionar material local. Su existencia en `.gitignore` no decide todavía dónde guardará los datos la aplicación instalada.

El instalador de Windows es el empaquetado de esta aplicación de escritorio. No añade un servidor. Su formato y la inclusión de libmpv, FFmpeg y yt-dlp están pendientes en [DECISIONS.md](DECISIONS.md).

## Comunicación entre interfaz y Rust

La interfaz invoca comandos de Tauri para acciones con resultado: listar una biblioteca, guardar una ficha, iniciar una importación o pedir la vista previa de un enlace.

Rust emite eventos para lo que cambia durante un rato: posición de reproducción, progreso de un elemento de descarga, progreso global de un lote y avisos de una operación de archivo incompleta.

El estado de reproducción vive en Rust. Así, cambiar de ruta en Angular no crea otro reproductor ni detiene el que ya suena. La interfaz se suscribe a ese estado y pinta la barra inferior, el panel derecho o el minirreproductor según la sección visible.

## Módulos de aplicación previstos

| Módulo | Encargo |
| --- | --- |
| Biblioteca | Altas, consultas, búsqueda, filtros, favoritos y clasificación en Música, Vídeos o Películas |
| Archivos físicos | Identidad del fichero en disco, compartida por las fichas que lo usen |
| Importación | Selección, copia o movimiento seguro, lectura inicial de metadatos y alta de la ficha |
| Recuperación | Reanudar o revisar operaciones de archivo incompletas al arrancar |
| Metadatos | Ediciones de cada ficha, prioridad de esos cambios e imágenes propias de la ficha |
| Playlists | Creación, orden y varias apariciones de una misma ficha |
| Reproducción | Un solo controlador: cola, aleatorio, repetición, pistas, subtítulos y posición de películas |
| Presentación visual | Elegir portada, vídeo original o vídeo asociado para el panel |
| Reconciliación | Comparar el disco con los archivos físicos y reflejar el estado en todas sus fichas |
| Descargas | Lotes, vista previa, posición de origen, cancelación, reintentos, duplicados y alta del resultado |
| Ajustes | Preferencias locales de la aplicación |

Colecciones y categorías no tienen módulo cerrado. Cuando se definan, entrarán con tablas nuevas, descritas en [DATABASE.md](DATABASE.md), sin reescribir las tablas ya existentes.

## Identidad del archivo y fichas

El archivo físico y la ficha no son el mismo registro.

| Registro | Guarda |
| --- | --- |
| Archivo físico | Ruta dentro de la carpeta administrada, tamaño, estado y origen remoto si llegó por descarga |
| Ficha | Biblioteca, textos visibles, favorito, posición de reanudación e imágenes |
| Vínculo | Qué ficha usa qué archivo y con qué papel: contenido principal o vídeo asociado |

Una importación confirmada crea una ficha en la biblioteca elegida y un vínculo a su archivo principal. El modelo puede asociar más fichas al mismo archivo físico. Ofrecer esa función a la persona usuaria es la decisión pendiente DP-18.

Cada ficha conserva sus metadatos personalizados y sus imágenes. Cambiar la portada de una ficha no cambia la de otra ni el archivo multimedia.

Quitar una ficha borra sus vínculos y puede retirar las imágenes que solo le pertenecían. No borra el archivo físico si otra ficha lo utiliza. Aunque sea la última ficha, el archivo físico tampoco se borra sin autorización.

## Flujos principales

### Importar

1. La persona elige archivos o una carpeta, la biblioteca de destino y si el archivo se copia o se mueve.
2. Rust comprueba el espacio disponible cuando la operación necesita una copia, y elige un nombre libre en la carpeta administrada. Un nombre ocupado no se sobrescribe: se reserva otro nombre y queda anotado en el diario.
3. Rust graba la operación en SQLite antes de modificar el fichero de origen.
4. Ejecuta el procedimiento de copia o movimiento de la sección siguiente.
5. Con el archivo ya verificado en el destino, Rust crea en una transacción la ficha, el archivo físico y el vínculo.
6. El adaptador de etiquetas lee los metadatos y, si existe, la portada incrustada.
7. La aplicación guarda una copia de esa portada en la carpeta de imágenes, ligada a la ficha, con origen `embedded`.
8. La interfaz muestra la ficha para corregirla.

Si el alta de la ficha falla después de haber dejado el archivo en el destino, el original de un movimiento no se elimina. La operación queda para la recuperación.

La extracción de la portada está confirmada. El archivo multimedia no cambia de contenido ni se reescribe. La herramienta que lee la imagen sigue pendiente.

### Copiar

1. Copiar al archivo temporal anotado en el diario.
2. Verificar que el temporal existe y que su tamaño coincide con el del origen.
3. Renombrar el temporal al nombre definitivo dentro de la carpeta administrada. Ese renombrado ocurre en el volumen de destino.
4. Confirmar en SQLite la ficha y el archivo físico.
5. El origen permanece en su sitio.

Una copia no elimina el original en ningún paso.

### Mover en el mismo volumen

Se usa un movimiento directo cuando se cumplen todas estas condiciones:

- El origen y el destino están en el mismo volumen.
- El diario de la operación ya está confirmado en SQLite y contiene el nombre definitivo.
- Ese nombre está libre.
- El sistema puede renombrar el archivo sin copiarlo.

El renombrado es el propio movimiento. Después se verifica que el destino existe y que el tamaño coincide con el anotado. A continuación se confirma la ficha. Si el renombrado falla, el origen sigue donde estaba.

Si alguna condición no se cumple, el mismo volumen usa el procedimiento de volúmenes distintos. Así un movimiento directo solo se hace cuando hay una forma segura de recuperarlo.

### Mover entre volúmenes distintos

1. Comprobar que el destino tiene espacio para el archivo completo.
2. Copiar al temporal del destino.
3. Verificar tamaño y existencia.
4. Renombrar el temporal al nombre definitivo.
5. Confirmar en SQLite la ficha, el archivo físico y el estado `db_committed` de la operación.
6. Eliminar el original solo en ese momento.

Hasta el paso 6 el original sigue en su ruta. Si la verificación o SQLite fallan, el original no se elimina.

### Conflictos, espacio, interrupciones y SQLite

| Situación | Resultado |
| --- | --- |
| El nombre de destino ya existe | Se elige otro nombre, se anota en el diario y no se sobrescribe el archivo existente |
| No hay espacio suficiente antes de copiar | La operación no empieza. El original permanece |
| Se agota el espacio durante la copia | Se retira solo el temporal incompleto creado por esta operación. El original permanece y la operación queda fallida |
| Se interrumpe el renombrado directo | La recuperación usa el diario. Si el archivo está en el destino y el tamaño coincide, se completa el alta. Si no se puede demostrar eso, la operación queda en revisión y no se borra nada |
| Falla SQLite antes de tocar el origen | No hay cambio en disco |
| Falla SQLite después de un renombrado directo ya verificado | El diario indica el destino. La recuperación completa el alta si el archivo sigue ahí y el tamaño coincide |
| Falla SQLite después de una copia verificada | Se conservan el original y la copia. No se elimina el original. La recuperación puede reintentar el alta |
| Falla o se interrumpe después del alta y antes de borrar el original de un movimiento | La recuperación borra el original solo si el destino verifica y el alta está confirmada |

### Recuperación de operaciones incompletas

Al arrancar, Rust lee el diario y trata cada operación que no esté completada:

| Etapa encontrada | Acción |
| --- | --- |
| Diario creado y el origen intacto, sin temporal o con temporal a medias | Se elimina solo ese temporal, si el diario lo identifica y está incompleto. El original se conserva. La operación pasa a fallida |
| Copia verificada en el destino y alta sin confirmar | Se reintenta el alta. El original se conserva |
| Renombrado directo verificado y alta sin confirmar | Se completa el alta contra el destino. No hace falta borrar el origen porque el renombrado ya lo trasladó |
| Alta confirmada, movimiento y origen todavía presente, destino verificado | Se completa el borrado del original. El resultado ya es conocido |
| Destino ausente, tamaño distinto o diario insuficiente para explicar el estado | La operación pasa a `needs_review`. No se elimina el original de forma automática. La interfaz lo muestra para que la persona decida |

La regla de cierre es única: sin una comprobación positiva del destino y del estado en SQLite, el original no se borra solo.

Las descargas de un archivo nuevo usan el mismo esquema de temporal, verificación y publicación. En ese caso no hay original de la persona usuaria que proteger. Un archivo a medias de una descarga cancelada se retira solo si es el temporal de ese trabajo.

### Editar una ficha

1. La interfaz envía los campos modificados.
2. Rust actualiza la ficha y marca esos campos como editados por la persona usuaria.
3. Si cambia la imagen, Rust guarda un archivo nuevo en la carpeta de imágenes, ligado a esa ficha, y lo marca como seleccionado.
4. La portada extraída permanece guardada y deja de ser la imagen seleccionada. No se borra por el hecho de personalizar la ficha.
5. El archivo multimedia no se reescribe.

Una lectura posterior del archivo puede refrescar los campos de esa ficha que la persona no ha tocado. Los campos editados se mantienen. Otra ficha vinculada al mismo archivo conserva sus propios valores.

### Reproducir

1. La interfaz pide reproducir una ficha o una aparición de una playlist.
2. El controlador único prepara la cola y pide al adaptador del motor que abra el archivo físico vinculado como contenido principal.
3. Rust publica estado: elemento actual, posición, pausa, pista y subtítulo.
4. Si la persona navega a otra sección, el controlador sigue en marcha y la interfaz cambia solo la presentación.

Para una película, el controlador recuerda la posición en la ficha y la ofrece al volver a abrirla.

La integración de libmpv se incorpora al principio del desarrollo, en la ventana real de Media Center. Con lo aprendido ahí se elige y se documenta una opción antes de construir las funciones avanzadas: ampliar y minimizar el vídeo, el minirreproductor, los subtítulos, las pistas y el vídeo asociado sincronizado. Esa elección está pendiente y tiene una tarea propia, F1.5, para no permanecer abierta sin límite.

### Canción con imagen en movimiento

El adaptador del motor tiene que poder separar la fuente de audio y la fuente de imagen:

| Situación | Audio | Imagen |
| --- | --- | --- |
| Canción con portada | Archivo principal de la ficha | Imagen seleccionada de esa ficha |
| Canción MP4 sin vídeo asociado | Archivo principal de la ficha | Vídeo de ese mismo archivo |
| Canción con vídeo asociado | Archivo principal de la ficha | Archivo de vídeo asociado, en silencio |

En los tres casos la persona controla una sola reproducción. El vídeo asociado sigue la pausa, la reanudación y la posición de la canción. La forma de conseguirlo forma parte de la integración que se documente en F1.5.

### Descargar

1. La interfaz envía un enlace compatible de YouTube.
2. Rust pide a yt-dlp la vista previa, sin iniciar la descarga.
3. La persona elige audio o audio y vídeo, formato, calidad y biblioteca.
4. Rust crea un lote y un elemento por cada pieza de la vista previa. Cada elemento guarda la posición original. Un vídeo suelto es un lote con un elemento en la posición 0.
5. La cola informa del progreso de cada elemento y del progreso global del lote, calculado a partir de sus elementos.
6. FFmpeg participa en las conversiones que la opción elegida requiera.
7. Al completar un elemento, su archivo queda publicado en la carpeta administrada y se crean su archivo físico y su ficha.
8. La miniatura descargada se guarda como imagen de esa ficha, distinta de una portada extraída más adelante.
9. Si el origen era una playlist, las fichas completadas pueden formar una playlist local ordenada por la posición original. Los elementos fallidos conservan su posición y su error.

Cancelar el lote marca como cancelados los elementos que aún no han terminado y conserva los ya completados. Reintentar reutiliza el mismo elemento y la misma posición, e incrementa el contador de intentos. No se crea un segundo elemento para el mismo puesto del lote.

La regla con la que se considera que una descarga está duplicada está pendiente. El identificador de lote no sirve como esa regla. El módulo debe dejar un punto claro para esa comprobación antes de publicar el archivo.

### Reconciliar archivos

Un recorrido compara las rutas de los archivos físicos con el disco. Si un archivo falta, su registro pasa a no disponible y todas las fichas vinculadas lo muestran así. Las fichas, sus textos editados y sus imágenes se conservan.

Si el método de identidad elegido reconoce el mismo contenido en otra ruta, se actualiza la ruta de ese archivo físico. No se crea una ficha nueva por el renombre. Las fichas que ya lo usaban siguen unidas a él.

Ese método de identidad está pendiente. La reconciliación también cubre cambios hechos a mano dentro de la carpeta administrada. No depende de que exista un modo de referencia externa ni de que se confirme la función de varias fichas por archivo.

## Adaptadores

Cada integración externa se esconde detrás de un adaptador. El resto de la aplicación habla con la interfaz del adaptador.

| Adaptador | Encargo confirmado | Elección abierta |
| --- | --- | --- |
| Motor multimedia | Abrir medios, controlar una sola reproducción, informar de posición, pistas y subtítulos, y separar audio de vídeo cuando haga falta | Forma concreta de integrar libmpv. Se elige en F1.5, antes de las funciones avanzadas |
| Descarga | Vista previa, descarga por lotes, progreso, cancelación y miniaturas de enlaces compatibles de YouTube | Versiones mínimas de yt-dlp y FFmpeg, la lista de formatos y si se empaquetan con el instalador |
| Etiquetas | Leer metadatos y extraer la portada incrustada sin modificar el archivo | Herramienta concreta. La escritura opcional queda prevista y desactivada por defecto |

## Datos en disco

Hay tres almacenes. El diario de copia y movimiento vive dentro de SQLite, no en una carpeta aparte.

| Almacén | Contenido |
| --- | --- |
| SQLite | Fichas, archivos físicos, vínculos, imágenes referenciadas, playlists, lotes, diario de operaciones y ajustes |
| Carpeta administrada | Archivos de música, vídeo y películas copiados o movidos. Un fichero ocupa un solo sitio aunque tenga varias fichas |
| Carpeta de imágenes | Portadas extraídas, imágenes personalizadas, pósters y miniaturas. Pertenecen a la ficha que las referencia |

La ubicación de las carpetas y de la base está pendiente. [DATABASE.md](DATABASE.md) propone la forma de los registros y no la ruta final. Esas tablas no están creadas.

## Límites de la primera versión

- No hay servidor HTTP de aplicación ni API remota.
- No hay cuentas ni sincronización.
- El streaming hacia una aplicación móvil no tiene componente en esta arquitectura.
- Spring Boot no participa.
- El instalador, cuando se prepare, distribuye la aplicación de escritorio. No abre un servicio.

## Riesgos técnicos

| Riesgo | Por qué importa | Cuándo se cierra |
| --- | --- | --- |
| Integrar libmpv con la ventana de Tauri | El vídeo tiene que verse dentro del diseño propio y, en las funciones avanzadas, ampliarse sin reiniciar y compartir controles con el audio | Al comienzo del desarrollo, en la tarea F1.5, antes de esas funciones. La elección sigue pendiente hasta entonces |
| Audio y vídeo de archivos distintos | El vídeo asociado no puede sonar y debe seguir la posición de la canción | La misma elección de F1.5, antes de la personalización visual avanzada |
| Identidad de un archivo movido | Sin un criterio estable, un renombre puede desenganchar las fichas o crear otra ficha sin querer | Decisión de identidad, antes de la reconciliación |
| Operación de archivo interrumpida | Un movimiento a medias podría perder el original | El diario y la recuperación de este documento. No se borra el original con incertidumbre |
| Procesos externos de yt-dlp y FFmpeg | Hace falta localizarlos, limitar el tiempo de espera y explicar un fallo sin dañar la biblioteca | Primera tarea del descargador. Empaquetarlos o exigirlos en la instalación sigue pendiente |
| Esquema de datos aprobado tarde | Cambiar tablas ya usadas obliga a migrar | Aprobación de [DATABASE.md](DATABASE.md) antes de crear la base. Esta revisión no lo aprueba |
