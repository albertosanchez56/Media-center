# Media Center — Diseño inicial de datos

Estado del documento: propuesta revisada, pendiente de aprobación.  
Última actualización: 2026-11-04.

Las tablas de este documento son un punto de partida para revisar. No son el esquema definitivo y no deben crearse hasta que se aprueben. Los nombres, las columnas y las relaciones pueden cambiar. La segunda revisión incorpora la separación entre archivo físico y ficha, las apariciones repetidas en una playlist, los lotes de descarga y el diario de operaciones. Eso no aprueba el esquema.

Los requisitos que esta propuesta intenta cubrir están en [REQUIREMENTS.md](REQUIREMENTS.md). Las decisiones que todavía cambian el modelo están en [DECISIONS.md](DECISIONS.md). El procedimiento de archivos que el diario debe poder recuperar está en [ARCHITECTURE.md](ARCHITECTURE.md).

## Criterios que sí orientan el diseño

Estos criterios ya están acordados y la propuesta los respeta:

- SQLite guarda la biblioteca. Las imágenes viven en disco y la base guarda la referencia.
- El archivo físico y la ficha son registros distintos. Un archivo desaparecido no borra las fichas.
- Varias fichas pueden vincularse al mismo archivo físico. Ofrecer esa función sigue pendiente; el modelo propuesto ya la admite.
- Los metadatos personalizados y las imágenes pertenecen a cada ficha.
- Los campos editados por la persona usuaria prevalecen sobre una lectura posterior del archivo.
- Editar la ficha no reescribe el archivo multimedia.
- La portada incrustada se extrae a la carpeta de imágenes y el archivo original se conserva igual. La imagen personalizada se guarda aparte.
- Eliminar una ficha no borra un archivo físico usado por otra ficha, ni ningún archivo físico sin autorización.
- El esquema evolucionará con migraciones versionadas. Categorías y colecciones entrarían en migraciones posteriores que añaden tablas nuevas.
- Hay tres bibliotecas fijas: Música, Vídeos y Películas.
- Una canción puede señalar un archivo principal y, aparte, un vídeo asociado.
- Las playlists son listas ordenadas. La misma ficha puede ocupar varias posiciones.
- Las descargas se agrupan en lotes. Cada elemento conserva la posición original hasta completarse, fallar, cancelarse o reintentarse.
- Toda copia o movimiento queda en un diario antes de alterar el origen, para poder recuperarla.

## Qué deja fuera a propósito

Hasta que haya una decisión, la propuesta no incluye en el esquema inicial tablas de:

- Categorías personalizadas.
- Colecciones.
- Referencias a archivos que permanecen fuera de la carpeta administrada.
- Letras, estadísticas, identificadores de MusicBrainz o TMDB, ni datos de una aplicación móvil.

Tampoco fija el algoritmo de hash, la ruta de la base de datos ni si la cola de reproducción se persiste.

La sección «Migraciones posteriores» indica cómo podrían añadirse categorías y colecciones sin reescribir las tablas iniciales. Ese boceto tampoco está aprobado y no forma parte de la primera migración.

## Enfoque general

Una **ficha** (`media_items`) es lo que la persona ve en una biblioteca. Un **archivo físico** (`stored_files`) es un fichero de la carpeta administrada. Un **vínculo** (`item_files`) une una ficha con un archivo y le da un papel: contenido principal o vídeo asociado.

Los campos visibles salen de la ficha. Una instantánea de lo importado permite saber qué cambió la persona usuaria en esa ficha y qué puede refrescarse si el archivo se vuelve a leer. Otra ficha del mismo archivo tiene su propia instantánea.

Las imágenes no se guardan como blobs. Cada imagen de una ficha es un archivo en la carpeta de imágenes y una fila que guarda la ruta relativa. La portada extraída y la imagen personalizada son filas distintas.

```
stored_files
    item_files           vínculo con una o varias fichas
media_items
    item_files
    media_images         imágenes propias de la ficha
    playlist_items       apariciones en playlists

download_batches
    download_jobs        un elemento por posición de origen

file_operations          diario de copia y movimiento
schema_migrations        control de versiones del esquema
```

Artistas, álbumes y géneros se proponen como texto en la ficha para la primera versión. Normalizarlos en tablas propias mejoraría los filtros y se puede añadir después con una migración, cuando se conozcan los filtros reales.

## Tablas propuestas

Los tipos son orientativos de SQLite. `id` sería una clave entera autogenerada. Las fechas se guardarían en UTC, en texto ISO-8601, salvo que al aprobar el esquema se prefiera otro formato.

### schema_migrations

Registro de migraciones ya aplicadas.

| Columna | Idea |
| --- | --- |
| version | Número entero de la migración, clave primaria |
| name | Nombre breve del cambio |
| applied_at | Momento en que se aplicó |

### stored_files

Identidad de un archivo dentro de la carpeta administrada.

| Columna | Idea |
| --- | --- |
| id | Identificador del archivo físico |
| relative_path | Ruta relativa a la carpeta administrada. Única en la propuesta |
| size_bytes | Tamaño en la última comprobación |
| modified_at | Fecha de modificación del fichero en esa comprobación |
| content_hash | Huella del contenido, nula hasta que se elija el algoritmo |
| container | Contenedor o extensión informativa, por ejemplo `mp4` o `flac` |
| status | `available` o `missing` |
| missing_since | Momento en que se detectó la ausencia; nulo si está disponible |
| source_url | Enlace de origen si el archivo llegó por descarga; nulo si fue importado |
| created_at | Alta del registro |
| updated_at | Último cambio |

No hay modo `external` en esta propuesta. Añadirlo sería el cambio natural si se aprueba referenciar archivos fuera de la carpeta administrada. En ese caso haría falta una ruta absoluta o una raíz distinta, y `relative_path` dejaría de bastar por sí solo.

### media_items

Ficha visible de una biblioteca.

| Columna | Idea |
| --- | --- |
| id | Identificador de la ficha |
| library | `music`, `video` o `movie` |
| title | Título visible |
| artist | Texto de artista o intérprete; vacío si no aplica |
| album | Texto de álbum; vacío si no aplica |
| genre | Texto de género; vacío si no aplica |
| year | Año, nulo si se desconoce |
| description | Descripción de vídeo o película; vacía si no aplica |
| duration_ms | Duración conocida, nula hasta que el motor o el archivo la aporten |
| favorite | Entero 0 o 1 |
| visual_preference | Preferencia de imagen de una canción. Valores propuestos: `cover`, `embedded_video`, `associated_video`. Nulo en vídeos y películas |
| resume_position_ms | Posición guardada. El requisito la exige para películas; usarla también en vídeos o música queda pendiente |
| imported_snapshot | Texto JSON con los valores leídos para esta ficha |
| user_edited_fields | Texto JSON con la lista de campos que la persona ha modificado en esta ficha |
| created_at | Alta de la ficha |
| updated_at | Último cambio de la ficha |

`visual_preference` existe para no decidir en silencio si un vídeo asociado sustituye siempre a la portada. Si se aprueba otra regla, la columna puede simplificarse.

`imported_snapshot` y `user_edited_fields` son la propuesta concreta para la prioridad de los cambios personales de cada ficha. Una tabla de procedencia por campo sería más estricta y más verbosa. Conviene elegir una de las dos al aprobar el esquema.

La biblioteca vive en la ficha. Así dos fichas del mismo archivo físico pueden pertenecer a bibliotecas distintas si la función pendiente llega a confirmarse. La importación confirmada crea una sola ficha.

### item_files

Vínculo entre ficha y archivo físico.

| Columna | Idea |
| --- | --- |
| id | Identificador del vínculo |
| media_item_id | Ficha |
| stored_file_id | Archivo físico |
| role | `primary` o `associated_video` |
| created_at | Alta del vínculo |

Índice único propuesto de (`media_item_id`, `role`): una ficha tiene un contenido principal y, como mucho, un vídeo asociado.

No se propone un índice único sobre `stored_file_id`. Varias fichas pueden apuntar al mismo archivo. La aplicación, mientras B-11 no esté confirmada, crea un solo vínculo principal al importar.

Borrar una fila de `item_files` no borra `stored_files`. Borrar una ficha puede borrar sus vínculos. No puede borrar el archivo físico si queda algún vínculo, y tampoco lo borra sin autorización cuando el vínculo era el último.

### media_images

Imágenes de trabajo de una ficha.

| Columna | Idea |
| --- | --- |
| id | Identificador de la imagen |
| media_item_id | Ficha dueña |
| kind | `cover`, `poster` o `thumbnail` |
| source | `embedded`, `download` o `user` |
| relative_path | Ruta relativa a la carpeta de imágenes |
| selected | Entero 0 o 1; la imagen que se muestra para ese `kind` |
| created_at | Alta |
| updated_at | Último cambio |

Al importar, la portada incrustada se copia a la carpeta de imágenes y se registra con `source = embedded`. El archivo multimedia no se modifica.

Sustituir la portada añade una fila `source = user`, la marca como seleccionada y deja la fila `embedded` guardada. La imagen personalizada y la portada extraída son archivos distintos.

Cada ficha tiene sus propias filas. Personalizar una ficha no cambia las imágenes de otra ficha vinculada al mismo archivo físico.

Se propone un índice único parcial, o una regla equivalente en la aplicación, para que cada ficha tenga como mucho una imagen seleccionada por cada `kind`.

### playlists

| Columna | Idea |
| --- | --- |
| id | Identificador |
| name | Nombre visible |
| description | Texto opcional |
| library | Biblioteca a la que pertenece, o nulo si al final se permiten mezclas |
| source_url | Enlace de la playlist de origen, nulo si es local desde el principio |
| created_at | Alta |
| updated_at | Último cambio |

La columna `library` refleja la duda abierta de si una playlist mezcla bibliotecas. Si la respuesta es que cada playlist pertenece a una sola biblioteca, la columna pasa a ser obligatoria.

### playlist_items

Apariciones ordenadas. La misma ficha puede aparecer más de una vez.

| Columna | Idea |
| --- | --- |
| id | Identificador de la aparición, clave primaria |
| playlist_id | Playlist |
| media_item_id | Ficha incluida en esta aparición |
| position | Orden entero dentro de la playlist, empezando en 0 |

Índice único propuesto de (`playlist_id`, `position`), para que cada puesto tenga una sola aparición.

No hay unicidad de (`playlist_id`, `media_item_id`). Quitar una aparición borra esa fila y no borra la ficha, el archivo ni las demás apariciones de la misma ficha. Al reordenar se reasignan las posiciones.

### download_batches

Una petición de descarga: un vídeo suelto o una playlist completa.

| Columna | Idea |
| --- | --- |
| id | Identificador del lote |
| source_url | Enlace pedido. En una playlist, el enlace de la playlist |
| kind | `single` o `playlist` |
| library | Biblioteca de destino |
| mode | `audio` o `audio_video` |
| format | Formato pedido |
| quality | Calidad pedida |
| status | `queued`, `running`, `completed`, `partial`, `failed` o `cancelled` |
| created_at | Alta |
| updated_at | Último cambio |

`partial` cubre un lote en el que unos elementos terminaron y otros fallaron o se cancelaron. El progreso global no se guarda en una columna: se calcula con los estados y el progreso de sus elementos.

### download_jobs

Un elemento del lote, con la posición que tenía en el origen.

| Columna | Idea |
| --- | --- |
| id | Identificador del elemento |
| batch_id | Lote |
| position | Posición original, empezando en 0. Estable entre reintentos |
| source_url | Enlace del elemento |
| source_key | Identificador estable del contenido remoto, cuando la vista previa lo aporte |
| title | Título de la vista previa |
| status | `queued`, `running`, `completed`, `failed` o `cancelled` |
| progress | Valor numérico de 0 a 100, orientativo de este elemento |
| error | Mensaje breve si el estado es `failed` |
| attempt_count | Veces que se ha intentado. Un reintento suma uno y conserva la fila y la posición |
| media_item_id | Ficha creada al completar; nula antes de eso |
| stored_file_id | Archivo físico publicado al completar; nulo antes de eso |
| created_at | Alta |
| updated_at | Último cambio |

Índice único propuesto de (`batch_id`, `position`).

Un vídeo individual tiene un lote `single` y un elemento en la posición 0. Cancelar el lote cancela los elementos que siguen en cola o en curso y deja los completados. Reintentar no inserta otra fila para la misma posición.

`source_key` se deja preparado para la detección de duplicados sin fijar todavía si el duplicado se decide por enlace, por identificador del vídeo, por huella del archivo o por una combinación. El `id` del lote no es esa regla.

La playlist local resultante, cuando se cree, usa las fichas de los elementos completados ordenadas por `position`.

### file_operations

Diario de cada copia o movimiento. La fila se confirma antes de alterar el origen.

| Columna | Idea |
| --- | --- |
| id | Identificador de la operación |
| kind | `copy` o `move` |
| volume_case | `same` o `different`. En `same`, la operación puede haber usado renombrado directo o la copia de reserva |
| source_path | Ruta absoluta del origen |
| temp_relative_path | Temporal dentro de la carpeta administrada; nulo si el movimiento directo no lo necesita |
| destination_relative_path | Nombre definitivo reservado, distinto de cualquier archivo ya presente |
| source_size_bytes | Tamaño anotado antes de tocar el origen |
| stage | Ver la lista de etapas |
| media_item_id | Ficha creada al confirmar el alta; nula antes |
| stored_file_id | Archivo físico creado al confirmar el alta; nulo antes |
| error | Texto breve en `failed` o `needs_review` |
| created_at | Alta |
| updated_at | Último cambio |

Etapas propuestas: `planned`, `transferring`, `verifying`, `ready_for_db`, `db_committed`, `source_delete_pending`, `completed`, `failed`, `needs_review`.

El significado de cada etapa y lo que la recuperación puede borrar están en [ARCHITECTURE.md](ARCHITECTURE.md). La recuperación no elimina el origen en `needs_review`.

### app_settings

Pares clave-valor para preferencias locales: volumen, modo de repetición u otras que se decida conservar.

| Columna | Idea |
| --- | --- |
| key | Clave primaria de texto |
| value | Texto |

La posición de una película no va aquí. Va en la ficha, porque es propia de ese elemento.

## Relaciones

- Un archivo físico tiene uno o varios vínculos.
- Una ficha tiene un vínculo principal y, si es una canción, puede tener un vínculo de vídeo asociado.
- Una ficha tiene cero o varias imágenes propias.
- Una ficha tiene cero o varias apariciones de playlist.
- Un lote tiene uno o varios elementos ordenados por la posición de origen.
- Un elemento completado apunta, como mucho, a una ficha y a un archivo físico.
- Una operación de archivo apunta a la ficha y al archivo físico solo después de confirmar el alta.
- Borrar una aparición no borra la ficha.
- Borrar los vínculos de una ficha no borra `stored_files` mientras otra ficha lo use.
- No hay borrado en cascada desde la ficha hacia el archivo físico.

La política general de borrar una ficha, más allá de esta protección, sigue sin estar pedida como función de la primera versión. Cuando exista, tendrá que respetar la autorización para cualquier archivo físico.

## Integridad que conviene decidir al aprobar

1. Claves foráneas activadas en SQLite.
2. `library` limitado a los tres valores, tanto en fichas como en lotes.
3. `role`, `status`, `kind`, `source` y los estados de descarga y de operación limitados a los valores de esta propuesta, o a los que se aprueben en su lugar.
4. `position` única dentro de cada playlist y dentro de cada lote.
5. `relative_path` único en `stored_files`.
6. El diario se inserta y se confirma antes de renombrar o copiar el origen.
7. La transacción que crea ficha, archivo físico y vínculo ocurre cuando el destino ya está verificado. Si esa transacción falla, el origen de un movimiento sigue en su sitio.
8. `item_files.stored_file_id` no usa borrado en cascada del archivo físico.

## Migraciones posteriores

La primera migración, cuando el esquema se apruebe, crearía las tablas aceptadas de este documento. Categorías y colecciones no entrarían en ella.

Si más adelante se confirman sus reglas, una migración nueva solo añadiría tablas. No añadiría columnas a `media_items`, `stored_files`, `item_files`, `playlists` ni `playlist_items` para representarlas.

Boceto no definitivo, excluido de la primera migración:

| Tabla futura | Papel |
| --- | --- |
| `categories` | Nombre y reglas que se aprueben. Una jerarquía, si se elige, sería una columna de esta tabla nueva |
| `item_categories` | Vínculo entre ficha y categoría |
| `collections` | Agrupación distinta de la playlist, cuando se defina |
| `collection_items` | Fichas de una colección. La posición sería nula si la colección no tiene orden |

Esas cuatro tablas se ajustarían a la decisión real. Sirven para comprobar que la evolución cabe en tablas nuevas. No se crean con el esquema inicial y sus columnas no están aprobadas.

Cada cambio posterior sería una migración nueva, con su número en `schema_migrations`. No se modificaría una migración ya aplicada.

## Ejemplo de lectura

Para pintar una canción, la aplicación haría conceptualmente lo siguiente:

1. Leer la ficha en `media_items`.
2. Leer su vínculo `primary` y, si existe, el `associated_video`.
3. Leer los `stored_files` de esos vínculos.
4. Leer las imágenes de esa ficha y usar la seleccionada. La fila `embedded` sigue disponible aunque la seleccionada sea `user`.
5. Elegir la presentación con `visual_preference` y con los archivos realmente disponibles.
6. Si el archivo físico está `missing`, mostrar la ficha y indicar que el archivo no está disponible.

## Preguntas para aprobar este diseño

El esquema sigue pendiente mientras estas preguntas no tengan respuesta en [DECISIONS.md](DECISIONS.md):

1. ¿Los campos editados se guardan con una instantánea JSON o con una fila por campo?
2. ¿Artista, álbum y género se quedan como texto en la primera versión?
3. ¿Una playlist pertenece a una sola biblioteca?
4. ¿La posición de reanudación se guarda solo en películas?
5. ¿Hace falta persistir la cola y el modo aleatorio?
6. ¿Qué identifica un duplicado de descarga?
7. ¿Se acepta aplazar categorías y colecciones a una migración que solo añada tablas?
8. ¿Se aceptan `stored_files`, `item_files`, `download_batches` y `file_operations` con la forma de esta propuesta?
9. ¿Se acepta que `playlist_items` identifique cada aparición por su propio `id` y permita repetir la ficha?

La pregunta 9 ya tiene una dirección pedida para la propuesta: las repeticiones se permiten. Al aprobar el esquema habrá que aceptarla de forma explícita o sustituirla. Hasta entonces no hay tablas creadas.
