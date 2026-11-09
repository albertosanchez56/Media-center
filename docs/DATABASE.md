# Media Center — Esquema inicial de datos

Estado del documento: esquema inicial aprobado el 2026-11-09 (DC-18).  
Última actualización: 2026-11-09.

Este documento es el esquema de la primera migración. Las rutas están decididas en DC-22. Las tablas no están creadas: falta la tarea F2.2. No se modifica una migración ya aplicada. Categorías y colecciones llegan después, en tablas nuevas (DC-16).

Los requisitos están en [REQUIREMENTS.md](REQUIREMENTS.md). Las decisiones cerradas y las que siguen abiertas están en [DECISIONS.md](DECISIONS.md). El procedimiento que el diario tiene que poder recuperar está en [ARCHITECTURE.md](ARCHITECTURE.md).

## Criterios del esquema

- SQLite guarda la biblioteca. Las imágenes viven en disco y la base guarda la referencia.
- El archivo físico (`stored_files`), la ficha (`media_items`) y el vínculo (`item_files`) son registros distintos.
- Un mismo archivo físico puede tener varias fichas, en bibliotecas distintas, sin duplicar el fichero (DC-21). La importación confirmada crea una sola ficha. Ofrecer la segunda es F3.5.
- Un archivo puede quedar referenciado en su ruta original, sin copiarlo ni moverlo (DC-19). La columna existe desde la primera migración. La función se implementa en F9.3.
- Los metadatos personalizados y las imágenes pertenecen a cada ficha. Editar la ficha no reescribe el archivo multimedia.
- Los campos que la persona ha editado prevalecen sobre una lectura posterior. La duración real del archivo no es uno de esos campos.
- La portada incrustada se extrae a la carpeta de imágenes. La imagen personalizada se guarda aparte y no sustituye el archivo extraído (DC-12).
- Eliminar una ficha quita sus vínculos, sus apariciones y sus filas de imagen. No elimina el archivo físico, aunque sea la última ficha que lo usaba.
- Una copia de imagen gestionada por Media Center puede borrarse cuando ya no tiene referencias y ninguna operación pendiente la nombra. Si el borrado del archivo falla después de confirmar SQLite, la ficha no se deshace y no se borran otros archivos.
- Hay tres bibliotecas fijas: Música, Vídeos y Películas.
- Una canción puede tener un vídeo asociado, por vínculo, sin duplicar ese archivo.
- La presentación visual por defecto es automática. La persona puede elegir otra opción en la ficha (DC-20).
- Las playlists son listas ordenadas. La misma ficha puede ocupar varias posiciones. En la primera versión cada playlist pertenece a una biblioteca. El campo admite nulo para listas mixtas futuras, sin reconstruir la tabla.
- Las descargas se agrupan en lotes. Cada elemento guarda su estado. El lote guarda la petición de cancelación. El porcentaje de progreso no se persiste.
- Un duplicado de descarga se detecta y se avisa. No se descarta en silencio. La señal que lo define sigue pendiente (DP-06).
- Toda copia o movimiento queda en un diario antes de alterar el origen. Una referencia externa no entra en ese diario, porque no modifica el disco.
- Antes de eliminar el original de un movimiento, el contenido del destino tiene que coincidir con el del origen. El tamaño y la fecha no bastan. Ante cualquier incertidumbre, el original se conserva.
- `content_hash` puede quedar vacío en la importación. Se calcula bajo demanda o en segundo plano. En una película grande no es un paso obligatorio del alta.
- El esquema evoluciona con migraciones versionadas y claves foráneas.

## Qué deja fuera la primera migración

No entran en la primera migración, y siguen pendientes de sus propias decisiones:

- Categorías personalizadas (DP-02).
- Colecciones (DP-03).
- Letras, estadísticas, identificadores de MusicBrainz o TMDB.
- Cola de reproducción persistida y búsqueda FTS5 (DP-12).
- La organización interna de la carpeta administrada, salvo el directorio reservado `.media-center\incoming` para los temporales recuperables (DC-22).
- Una columna de número de pista. Cabrá después con `ADD COLUMN`.

La sección «Migraciones posteriores» muestra cómo podrían añadirse categorías y colecciones. Ese boceto no está aprobado y no forma parte de la primera migración.

## Enfoque general

Una **ficha** es lo que la persona ve en una biblioteca. Un **archivo físico** es un fichero en disco: dentro de la carpeta administrada o, cuando la referencia se implemente, en su ruta original. Un **vínculo** une una ficha con un archivo y le da un papel: contenido principal o vídeo asociado.

Los campos visibles salen de la ficha. `imported_snapshot` guarda lo leído para esa ficha. `user_edited_fields` lista lo que la persona ha cambiado en ella. Otra ficha del mismo archivo tiene su propia instantánea. `duration_ms` no entra en esa lista: la actualizan el archivo y el motor.

Las imágenes no se guardan como blobs. Cada imagen de una ficha es un archivo en la carpeta de imágenes y una fila con la ruta relativa.

```
stored_files
    item_files           vínculo con una o varias fichas
media_items
    item_files
    media_images         imágenes propias de la ficha
    playlist_items       apariciones en playlists

download_batches
    download_jobs        un elemento por posición de origen

file_operations          diario de copia, movimiento y publicación de descargas
schema_migrations        control de versiones del esquema
app_settings             ajustes locales
```

Artista, álbum y género son texto en la ficha. Normalizarlos en tablas propias puede hacerse en una migración posterior, cuando se conozcan los filtros reales (DP-12).

## Reglas comunes de SQLite

La primera migración crea las tablas de este documento como tablas `STRICT`. `id` es una clave entera autogenerada. Las fechas se guardan en texto ISO-8601 UTC.

Cada conexión abre con claves foráneas activas, diario WAL, `synchronous = FULL` y un tiempo de espera si la base está ocupada. El diario de una operación se confirma con esa sincronización antes de renombrar o copiar el origen.

Las migraciones son SQL numerado, embebido en la aplicación. Cada una se aplica en una transacción y se registra en `schema_migrations`. Cambiar un enumerado de un `CHECK` más adelante exige una migración que reconstruya la tabla y lo deje documentado.

La clave de ruta `path_key` la calcula la aplicación antes de insertar. Usa separadores `/`, prescinde del prefijo `\\?\`, normaliza a NFC y pliega mayúsculas con la API de Windows. El `NOCASE` de SQLite solo pliega ASCII y no sirve para distinguir `café` de `CAFÉ`. El índice único es sobre esa clave ya normalizada. Además, la aplicación pregunta al sistema de archivos antes de reservar un nombre, por los nombres cortos 8.3 y los espacios finales.

## Tablas

### schema_migrations

| Columna | Restricción |
| --- | --- |
| version | Entero, clave primaria |
| name | Nombre breve del cambio |
| applied_at | Momento en que se aplicó |

### stored_files

Identidad de un archivo en disco.

| Columna | Restricción |
| --- | --- |
| id | Clave primaria |
| storage | `managed` o `external` |
| path | Ruta tal como la devuelve el sistema. Relativa a la carpeta administrada si `storage` es `managed`. Absoluta si es `external` |
| path_key | Clave normalizada. Única |
| size_bytes | Tamaño en la última comprobación |
| modified_at | Fecha de modificación del fichero en esa comprobación |
| content_hash | Huella del contenido, anulable. No es obligatoria al importar |
| file_id | Identificador de archivo del volumen, anulable. En NTFS puede sobrevivir a un renombrado dentro del mismo volumen. En FAT, exFAT, red o al copiar entre volúmenes puede no existir o cambiar |
| container | Contenedor o extensión informativa, por ejemplo `mp4` o `flac` |
| status | `available` o `missing` |
| missing_since | Momento en que se detectó la ausencia. Nulo si está disponible |
| created_at | Alta del registro |
| updated_at | Último cambio |

La aplicación valida que `path` sea relativa cuando `storage` es `managed` y absoluta cuando es `external`. Una ruta que cae dentro de la carpeta administrada no se guarda como `external`: se reutiliza la fila existente.

Una fila `external` no se mueve, no se renombra y no se borra por una operación de la aplicación. La importación por referencia no crea fila en `file_operations`.

`content_hash` se rellena bajo demanda o en segundo plano. El alta de una película grande no espera a esa huella. El algoritmo concreto sigue sin fijarse; la columna no cambia cuando se elija. La comparación de contenido que autoriza borrar un original puede calcular la huella en ese momento y guardarla aquí.

El enlace de una descarga no se guarda en esta tabla. Vive en `download_jobs`.

### media_items

Ficha visible de una biblioteca.

| Columna | Restricción |
| --- | --- |
| id | Clave primaria |
| library | `music`, `video` o `movie`. Obligatoria |
| title | Título visible |
| artist | Texto de artista o intérprete. Vacío si no aplica |
| album | Texto de álbum. Vacío si no aplica |
| genre | Texto de género. Vacío si no aplica |
| year | Año, nulo si se desconoce |
| description | Descripción de vídeo o película. Vacía si no aplica |
| duration_ms | Duración conocida, nula hasta que el archivo o el motor la aporten. No es editable por la persona |
| favorite | Entero 0 o 1 |
| visual_preference | Nula, `cover`, `original_video` o `associated_video`. Nula significa la regla automática de DC-20 |
| resume_position_ms | Posición guardada, anulable. La aplicación solo la escribe en películas mientras DP-13 siga abierta |
| imported_snapshot | Texto JSON válido con los valores leídos para esta ficha |
| user_edited_fields | Texto JSON válido con los campos que la persona ha modificado. No incluye `duration_ms` |
| created_at | Alta de la ficha |
| updated_at | Último cambio de la ficha |

Campos que la persona puede editar: título, artista, álbum, género, año, descripción, favorito, preferencia visual e imágenes. Una relectura del archivo refresca el resto, incluida la duración.

La biblioteca vive en la ficha. Dos fichas del mismo archivo pueden pertenecer a bibliotecas distintas.

### item_files

Vínculo entre ficha y archivo físico.

| Columna | Restricción |
| --- | --- |
| id | Clave primaria |
| media_item_id | Clave foránea a `media_items`, `ON DELETE CASCADE` |
| stored_file_id | Clave foránea a `stored_files`, `ON DELETE RESTRICT` |
| role | `primary` o `associated_video` |
| created_at | Alta del vínculo |

Índice único parcial de `role = 'primary'`: una ficha, un contenido principal. Índice único parcial de `role = 'associated_video'`: como mucho un vídeo asociado. No hay índice único sobre `stored_file_id`.

La aplicación solo crea `associated_video` en fichas de música. Quitar la asociación borra el vínculo, no el archivo.

`ON DELETE RESTRICT` impide borrar un archivo físico mientras exista un vínculo. Cuando ya no queda ninguno, el archivo tampoco se borra solo: hace falta una autorización explícita (DC-07).

### media_images

Imágenes de trabajo de una ficha. El tipo visual se deduce de la biblioteca: portada, miniatura o póster. No hay columna `kind`.

| Columna | Restricción |
| --- | --- |
| id | Clave primaria |
| media_item_id | Clave foránea a `media_items`, `ON DELETE CASCADE` |
| source | `embedded`, `download` o `user` |
| relative_path | Ruta relativa a la carpeta de imágenes, generada por la aplicación |
| selected | Entero 0 o 1 |
| created_at | Alta |
| updated_at | Último cambio |

Índice único parcial: una sola fila con `selected = 1` por ficha. La fila `embedded` permanece, con `selected = 0`, cuando la seleccionada es `user` (DC-12).

Al importar, la portada incrustada se copia a la carpeta de imágenes y se registra con `source = embedded`. El archivo multimedia no se modifica. Primero se escribe el archivo de imagen y después la fila.

Cada ficha tiene sus propias filas. Personalizar una ficha no cambia las imágenes de otra ficha del mismo archivo físico.

Borrado del archivo de imagen:

1. Se confirma en SQLite la retirada de la fila, o de la ficha que la arrastra en cascada.
2. Solo entonces se intenta borrar el archivo, y solo si ninguna fila de `media_images` sigue usando esa ruta y ninguna `file_operations` pendiente la nombra.
3. Se borra una ruta identificada cada vez.
4. Si el borrado físico falla, la transacción de SQLite no se deshace. La ficha queda como quedó confirmada. No se borran otros archivos para compensar. El archivo que no se pudo quitar permanece y puede retirarse más adelante con las mismas comprobaciones.
5. Al arrancar no se recorre la carpeta de imágenes para borrar huérfanos.

### playlists

| Columna | Restricción |
| --- | --- |
| id | Clave primaria |
| name | Nombre visible |
| description | Texto opcional, anulable |
| library | Anulable: `music`, `video` o `movie` |
| source_url | Enlace de la playlist de origen. Nulo si es local desde el principio |
| created_at | Alta |
| updated_at | Último cambio |

En la primera versión la aplicación rechaza crear una playlist con `library` nula. Con biblioteca, rechaza apariciones de fichas de otra biblioteca. No hay un `CHECK` que cruce tablas: es una regla de la aplicación. Una playlist mixta futura usará `NULL` y no necesita migración.

### playlist_items

Apariciones ordenadas. La misma ficha puede aparecer más de una vez.

| Columna | Restricción |
| --- | --- |
| id | Clave primaria de la aparición |
| playlist_id | Clave foránea a `playlists`, `ON DELETE CASCADE` |
| media_item_id | Clave foránea a `media_items`, `ON DELETE CASCADE` |
| position | Orden entero dentro de la playlist, desde 0. Único junto con `playlist_id` |

No hay unicidad de (`playlist_id`, `media_item_id`). Quitar una aparición borra esa fila y no borra la ficha, el archivo ni las demás apariciones. Borrar la ficha borra sus apariciones.

Reordenar se hace en una transacción de dos pasos, porque SQLite comprueba la unicidad fila a fila.

### download_batches

Una petición de descarga: un vídeo suelto o una playlist completa.

| Columna | Restricción |
| --- | --- |
| id | Clave primaria |
| source_url | Enlace pedido. En una playlist, el enlace de la playlist |
| kind | `single` o `playlist` |
| library | `music`, `video` o `movie`. Obligatoria |
| mode | `audio` o `audio_video` |
| format | Formato pedido |
| quality | Calidad pedida |
| cancel_requested | Entero 0 o 1. La intención de cancelar el lote, y sobrevive a un cierre |
| created_at | Alta |
| updated_at | Último cambio |

No hay columna de estado ni de progreso del lote. Se calculan con los elementos. `cancel_requested` no se calcula: es la petición de la persona.

Al arrancar, si `cancel_requested` está activo, los elementos que hayan quedado en `queued` o `running` pasan a `cancelled`.

### download_jobs

Un elemento del lote, con su estado persistente y la posición que tenía en el origen.

| Columna | Restricción |
| --- | --- |
| id | Clave primaria |
| batch_id | Clave foránea a `download_batches`, `ON DELETE CASCADE` |
| position | Posición original, desde 0. Estable entre reintentos. Única junto con `batch_id` |
| source_url | Enlace del elemento |
| source_key | Identificador estable del contenido remoto, cuando la vista previa lo aporte. Anulable. Índice no único |
| title | Título de la vista previa |
| status | `queued`, `running`, `completed`, `failed` o `cancelled` |
| error | Mensaje breve si el estado es `failed`. Anulable |
| attempt_count | Veces que se ha intentado |
| media_item_id | Ficha creada al completar. Anulable. `ON DELETE SET NULL` |
| stored_file_id | Archivo físico publicado al completar. Anulable. `ON DELETE SET NULL` |
| created_at | Alta |
| updated_at | Último cambio |

No hay columna de porcentaje. El progreso de un elemento en curso es volátil y se comunica por eventos. El estado sí se guarda.

Un vídeo individual tiene un lote `single` y un elemento en la posición 0. Reintentar reutiliza la fila y suma `attempt_count`. Al arrancar, un elemento `running` de un lote que no está cancelado vuelve a `queued`: el proceso que lo descargaba ya no existe. Los estados `completed`, `failed` y `cancelled` se conservan.

`source_key` prepara la detección de duplicados. No es único: la persona puede confirmar una descarga repetida. La señal que define el duplicado sigue en DP-06. El comportamiento ya acordado es avisar y no descartar el contenido en silencio.

La publicación del archivo usa `file_operations` con `kind = 'download'`. No hay original de la persona usuaria que borrar. La playlist local, cuando se cree, usa las fichas de los elementos completados ordenadas por `position`.

### file_operations

Diario de cada copia, movimiento o publicación de una descarga. La fila se confirma antes de alterar el origen.

| Columna | Restricción |
| --- | --- |
| id | Clave primaria |
| kind | `copy`, `move` o `download` |
| method | `rename` o `copy` |
| library | Biblioteca de destino. Permite completar el alta en la recuperación |
| source_path | Ruta absoluta del origen. En una descarga, el temporal producido por la herramienta, no un archivo de la persona |
| source_size_bytes | Tamaño anotado antes de tocar el origen |
| source_modified_at | Fecha de modificación anotada. Anulable en una descarga |
| temp_relative_path | Temporal dentro de la carpeta administrada. Nulo si el movimiento directo no lo usa |
| destination_relative_path | Nombre definitivo reservado, distinto de cualquier archivo ya presente |
| stage | Una de las etapas de la lista siguiente |
| media_item_id | Ficha creada al confirmar el alta. Anulable. `ON DELETE SET NULL` |
| stored_file_id | Archivo físico creado al confirmar el alta. Anulable. `ON DELETE SET NULL` |
| error | Texto breve en `failed` o `needs_review`. Anulable |
| created_at | Alta |
| updated_at | Último cambio |

Etapas: `planned`, `transferring`, `verifying`, `ready_for_db`, `db_committed`, `source_delete_pending`, `completed`, `failed`, `needs_review`.

Una importación por referencia no crea fila. Si la transacción del alta falla, no queda nada a medias en disco.

Recuperación, al arrancar y repetible sobre el mismo estado:

- Origen intacto y temporal ausente o a medias: se borra solo ese temporal y la operación pasa a `failed`. El original se conserva.
- Destino presente y alta sin confirmar: se completa el alta una sola vez. Si `media_item_id` ya existe, no se crea otra ficha. El original se conserva.
- Alta confirmada, movimiento, destino presente y origen todavía presente: el original solo se borra si una comparación de contenido demuestra que el destino coincide con el origen. El tamaño y `source_modified_at` no autorizan ese borrado por sí solos. Si la comparación no puede completarse, no coincide, o el diario no basta para explicar el estado, la operación pasa a `needs_review` y el original se conserva.
- Una operación `completed`, `failed` o `needs_review` no se vuelve a ejecutar. `needs_review` espera a la persona.

El detalle de cada etapa está en [ARCHITECTURE.md](ARCHITECTURE.md).

### app_settings

| Columna | Restricción |
| --- | --- |
| key | Texto, clave primaria |
| value | Texto |

La posición de una película no se guarda aquí. Va en `media_items.resume_position_ms`.

## Relaciones al borrar

- Borrar una ficha borra sus vínculos, sus apariciones y sus filas de imagen. Los archivos de imagen se intentan borrar después, con la política de `media_images`. El archivo físico permanece.
- Borrar un archivo físico está prohibido mientras un vínculo lo señale. Sin vínculos, solo una acción autorizada puede borrarlo.
- Borrar una playlist borra sus apariciones. Las fichas siguen.
- Borrar un lote borra sus elementos. Las fichas y los archivos ya publicados siguen. Si se borra la ficha, el elemento deja esas referencias a nulo.
- Borrar una ficha no borra el historial de `file_operations` ni de `download_jobs`: las referencias quedan a nulo.

## Identidad de un archivo

Las columnas están desde la primera migración. La reasignación automática de una ruta sigue pendiente (DP-05) y no se implementa antes de F9.2. Mientras tanto, si el archivo falta en su ruta, pasa a `missing` y las fichas se conservan.

Cuando F9.2 fije la regla, las señales disponibles serán, en este orden de uso previsto:

1. La ruta, con el mismo tamaño y la misma fecha, sigue disponible.
2. Si falta en la ruta, `file_id` puede reconocerlo tras un renombrado dentro del mismo volumen que conserve ese identificador.
3. Si no hay `file_id`, o el archivo cambió de volumen o de sistema de archivos, se comparan candidatos del mismo tamaño mediante `content_hash`, calculada en ese momento si todavía es nula.
4. Una sola coincidencia segura actualiza la ruta. Varias, o ninguna, dejan el archivo en `missing`. No se crea una ficha nueva.

## Migraciones posteriores

La primera migración crea las tablas de este documento y no crea las siguientes. Si se confirman sus reglas, una migración nueva solo añadirá tablas. No añadirá columnas de categoría o de colección a `media_items`, `stored_files`, `item_files`, `playlists` ni `playlist_items`.

Boceto excluido de la primera migración y no aprobado:

| Tabla futura | Papel |
| --- | --- |
| `categories` | Nombre y reglas que se aprueben |
| `item_categories` | Vínculo entre ficha y categoría |
| `collections` | Agrupación distinta de la playlist, cuando se defina |
| `collection_items` | Fichas de una colección |

Cada cambio posterior es una migración nueva, con su número en `schema_migrations`.

## Ejemplo de lectura

Para pintar una canción, la aplicación:

1. Lee la ficha en `media_items`.
2. Lee su vínculo `primary` y, si existe, el `associated_video`.
3. Lee los `stored_files` de esos vínculos.
4. Lee las imágenes de esa ficha y usa la seleccionada. La fila `embedded` sigue disponible aunque la seleccionada sea `user`.
5. Elige la presentación con DC-20: si `visual_preference` es nula, vídeo asociado si existe, si no el vídeo del propio archivo si corresponde, si no la portada. Un valor de la columna es la elección explícita.
6. Si el archivo físico está `missing`, muestra la ficha e indica que el archivo no está disponible.
