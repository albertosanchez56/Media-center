# Media Center — Requisitos

Estado del documento: planificación inicial, con la segunda revisión aplicada.  
Última actualización: 2026-11-06.

Los términos de producto están definidos en [PROJECT.md](PROJECT.md). Cada requisito lleva uno de estos estados:

| Estado | Significado |
| --- | --- |
| Confirmado | Acordado para la primera versión |
| Previsto | Entra en la dirección del producto y todavía falta cerrar el detalle |
| Pendiente | Hace falta una decisión antes de implementarlo |
| Futuro | Queda fuera de la primera versión |

El esquema de [DATABASE.md](DATABASE.md) que da forma a estos requisitos sigue pendiente de aprobación.

## 1. Bibliotecas

| ID | Requisito | Estado |
| --- | --- | --- |
| B-01 | Existen tres bibliotecas principales: Música, Vídeos y Películas | Confirmado |
| B-02 | Al importar, la persona usuaria elige la biblioteca de destino | Confirmado |
| B-03 | Un archivo MP4 puede clasificarse como música, vídeo o película | Confirmado |
| B-04 | Cada biblioteca puede verse en lista y en cuadrícula | Confirmado |
| B-05 | Se puede buscar dentro de la biblioteca | Confirmado |
| B-06 | Se puede filtrar el contenido | Confirmado. Los campos concretos del filtro están pendientes |
| B-07 | Cada ficha puede marcarse como favorita | Confirmado |
| B-08 | Se pueden crear playlists | Confirmado |
| B-09 | Se pueden crear colecciones | Previsto. Falta definir en qué se diferencian de una playlist |
| B-10 | Habrá categorías personalizadas | Previsto. Las reglas exactas están pendientes |
| B-11 | Un mismo archivo físico puede tener fichas distintas, cada una con sus metadatos y sus imágenes | Pendiente de confirmación funcional. El modelo propuesto ya separa el archivo de las fichas para poder admitirlo |

### Detalles pendientes de biblioteca

- Qué campos cubren la búsqueda: título, artista, álbum, género, nombre de archivo u otros.
- Qué filtros existen además de la biblioteca y el favorito.
- Si una playlist puede mezclar fichas de distintas bibliotecas.
- Si una colección es una saga, una agrupación sin orden de reproducción u otra cosa.
- Si una ficha puede tener varias categorías, si las categorías son jerárquicas y si dependen de la biblioteca.
- Si la persona usuaria podrá crear una segunda ficha, en otra biblioteca, sobre un archivo que ya está en la carpeta administrada.

La propuesta de datos permite que la misma ficha aparezca varias veces en una playlist, cada vez con su posición. Esa forma pertenece al esquema todavía no aprobado.

Importar crea la ficha en la biblioteca elegida. Crear otra ficha para el mismo archivo es la función B-11 y no forma parte del comportamiento confirmado de la importación.

## 2. Reproducción

| ID | Requisito | Estado |
| --- | --- | --- |
| R-01 | Se reproducen música y vídeo en los formatos que admita el motor integrado | Confirmado. El motor es libmpv, integrado dentro del proceso de Media Center (DC-17, elegido en F1.5). La reproducción del producto no está implementada: llega con F7.1 |
| R-02 | Solo hay una reproducción activa a la vez | Confirmado |
| R-03 | La reproducción continúa al cambiar de sección | Confirmado |
| R-04 | La música usa un reproductor inferior de estilo similar a Spotify, con diseño propio | Confirmado |
| R-05 | Un panel derecho muestra la portada o el vídeo de la canción | Confirmado |
| R-06 | Una canción MP4 muestra su videoclip en el panel derecho | Confirmado |
| R-07 | El vídeo puede ampliarse y minimizarse sin reiniciar la reproducción | Confirmado. El destino visual de la vista ampliada se definirá con la interfaz. Es una función avanzada, sin implementar. F1.5 comprobó su viabilidad en el diagnóstico: la misma superficie cambia de hueco sin reiniciar |
| R-08 | Películas y vídeos pasan a un minirreproductor al cambiar de sección | Confirmado. La colocación del minirreproductor está pendiente de diseño |
| R-09 | Una película se reanuda automáticamente en la posición anterior | Confirmado |
| R-10 | Hay subtítulos, selección de pistas de audio, pantalla completa y controles de reproducción | Confirmado. El origen de los subtítulos (incrustados, archivo externo o ambos) está pendiente (DP-14). Sin implementar en el producto. F1.5 comprobó en el diagnóstico que son viables las dos fuentes de subtítulos, el cambio de pista de audio y la pantalla completa |
| R-11 | Hay cola de reproducción, reproducción aleatoria y modos de repetición | Confirmado. Qué modos de repetición se ofrecen y si la cola se guarda entre sesiones está pendiente |

La reanudación automática está confirmada para películas. Guardar la posición de vídeos o de canciones es una decisión pendiente.

Una canción con vídeo asociado sigue contando como una sola reproducción: el audio es el de la canción y el vídeo es la imagen que la acompaña. El detalle está en la sección 3. Esa sincronización es una función avanzada del reproductor.

El trabajo de libmpv se hace dentro de Media Center. No se pide un proyecto de pruebas independiente. La integración se eligió en F1.5, antes de construir las funciones avanzadas: libmpv dentro del proceso (DC-17).

## 3. Personalización visual de canciones

| ID | Requisito | Estado |
| --- | --- | --- |
| V-01 | Si el archivo musical trae una portada incrustada, la aplicación extrae una copia a su carpeta de imágenes y la usa en la ficha | Confirmado. El archivo original no se modifica |
| V-02 | Cualquier portada puede sustituirse | Confirmado |
| V-03 | Una imagen personalizada permanece guardada con independencia de la portada extraída | Confirmado |
| V-04 | Una canción, sea MP3, FLAC, MP4 u otro formato admitido, puede tener un vídeo asociado | Confirmado |
| V-05 | Si hay un vídeo asociado, puede mostrarse en lugar de la portada o del vídeo original | Confirmado. Falta decidir si el cambio es automático o lo elige la persona usuaria en cada ficha |
| V-06 | El audio que se oye es siempre el de la canción | Confirmado |
| V-07 | El vídeo asociado se reproduce sin su propio sonido | Confirmado |
| V-08 | El vídeo asociado sigue los controles de la canción, incluidos pausa, reanudación y cambio de posición | Confirmado |
| V-09 | Si el vídeo asociado dura menos que la canción, se contempla reproducirlo en bucle | Pendiente. Está contemplado y todavía no está elegido |
| V-10 | La sincronización y el renderizado se harán con la integración de libmpv que se documente antes de esta función | Confirmado: libmpv dentro del proceso (DC-17), con el vídeo asociado añadido a la misma sesión y saltos exactos. Viabilidad comprobada en F1.4 y F1.5. La función es avanzada y no está implementada: llega con F8.2 |

Las imágenes y los metadatos personalizados pertenecen a la ficha. Dos fichas asociadas al mismo archivo físico, si B-11 llega a confirmarse, pueden mostrar portadas distintas.

## 4. Importación

| ID | Requisito | Estado |
| --- | --- | --- |
| I-01 | Se pueden importar archivos individuales, varios archivos y carpetas | Confirmado |
| I-02 | La persona usuaria puede copiar archivos a la carpeta administrada | Confirmado |
| I-03 | La persona usuaria puede mover archivos a la carpeta administrada | Confirmado |
| I-04 | Los metadatos existentes se leen al importar | Confirmado |
| I-05 | La portada incrustada se extrae a la carpeta de imágenes cuando existe | Confirmado. Coincide con V-01 |
| I-06 | La información importada se puede corregir y completar | Confirmado |
| I-07 | La aplicación detecta archivos renombrados, movidos o eliminados fuera de ella | Confirmado. El método de identificación del archivo está pendiente |
| I-08 | La ficha de un archivo desaparecido se conserva | Confirmado |
| I-09 | La aplicación no elimina archivos físicos sin autorización | Confirmado |
| I-10 | Copiar o mover sigue el procedimiento seguro de [ARCHITECTURE.md](ARCHITECTURE.md) | Confirmado |
| I-11 | Una operación de copia o movimiento incompleta se puede recuperar | Confirmado |
| I-12 | Con incertidumbre sobre el resultado, la aplicación no elimina el original de forma automática | Confirmado |

En el mismo volumen se valora un movimiento directo y seguro, con un registro previo de la operación. Entre volúmenes distintos la aplicación copia a un archivo temporal, verifica la copia y solo después, con la operación completada, puede eliminar el original. El procedimiento también cubre un nombre ya ocupado, la falta de espacio, una interrupción y un fallo de SQLite.

Referenciar un archivo y dejarlo en su carpeta original, sin copiarlo ni moverlo, es una posibilidad sin confirmar. Hasta que se decida, la importación confirmada consiste en copiar o mover hacia la carpeta administrada.

También está pendiente la ubicación de esa carpeta: directorio de datos de la aplicación, carpeta elegida por la persona usuaria, u otra opción.

## 5. Descargador

| ID | Requisito | Estado |
| --- | --- | --- |
| D-01 | Se pueden descargar enlaces compatibles de YouTube cuando los permisos, los derechos y las condiciones aplicables lo permitan | Confirmado |
| D-02 | Se puede descargar un vídeo individual | Confirmado |
| D-03 | Se puede descargar una playlist completa | Confirmado |
| D-04 | Antes de descargar se muestra una vista previa del contenido y de sus metadatos | Confirmado |
| D-05 | Se puede elegir solo audio, o audio y vídeo | Confirmado |
| D-06 | Se puede elegir formato, calidad y biblioteca de destino | Confirmado. Los formatos y calidades ofrecidos dependen de las herramientas y están pendientes de concreción |
| D-07 | Se pueden descargar miniaturas | Confirmado |
| D-08 | Hay cola de descargas, progreso, cancelación y reintentos | Confirmado |
| D-09 | Se detectan duplicados | Confirmado. La regla de duplicado está pendiente |
| D-10 | Una playlist importada puede generar una playlist local | Confirmado |

La propuesta de datos, todavía sin aprobar, agrupa cada petición en un lote. Cada elemento conserva la posición que tenía en el origen. Así se pueden representar el progreso global, una cancelación, un reintento y el orden de la playlist local. Un vídeo individual es un lote de un solo elemento. El identificador de lote no sustituye a la regla de duplicados, que sigue pendiente.

El descargador de la primera versión trabaja con enlaces compatibles de YouTube. Otros sitios que yt-dlp pueda resolver no forman parte del alcance confirmado.

La interfaz debe dejar claro que la descarga corresponde a quien usa la aplicación y que está sujeta a permisos, derechos y condiciones del servicio.

## 6. Metadatos

| ID | Requisito | Estado |
| --- | --- | --- |
| M-01 | Los datos de la biblioteca se guardan en SQLite | Confirmado |
| M-02 | Las imágenes se guardan en disco y SQLite guarda su referencia | Confirmado |
| M-03 | De una canción se pueden editar título, artista, álbum, género, portada y otros campos relevantes | Confirmado. La lista cerrada de campos está pendiente de aprobación junto con el esquema |
| M-04 | De un vídeo o una película se pueden editar la información, el póster y la descripción | Confirmado |
| M-05 | Un cambio personalizado prevalece sobre el metadato importado o descargado | Confirmado |
| M-06 | Editar metadatos en la aplicación no modifica el archivo original | Confirmado como comportamiento por defecto |
| M-07 | Se contempla escribir etiquetas en archivos compatibles como acción opcional | Previsto. La herramienta, los formatos y el momento de ofrecerla están pendientes |
| M-08 | Los metadatos personalizados y las imágenes pertenecen a la ficha, no al archivo físico compartido | Confirmado |
| M-09 | Eliminar una ficha no borra un archivo físico que otra ficha utiliza | Confirmado |

La lista exacta de columnas sigue en la propuesta de [DATABASE.md](DATABASE.md) y no está aprobada.

## 7. Interfaz

| ID | Requisito | Estado |
| --- | --- | --- |
| U-01 | Tema oscuro, aspecto moderno y adaptación al tamaño de la ventana | Confirmado |
| U-02 | Menú lateral izquierdo con las bibliotecas y las playlists | Confirmado |
| U-03 | Zona central para el contenido | Confirmado |
| U-04 | Panel derecho para la reproducción musical | Confirmado |
| U-05 | Controles globales de reproducción | Confirmado |
| U-06 | El diseño concreto se define de forma progresiva | Confirmado como forma de trabajo |

Queda pendiente de diseño cómo conviven la barra musical, el visionado de una película o un vídeo, y el minirreproductor. También queda pendiente el aspecto visual final: tipografía, espaciado, iconos y componentes.

## 8. Restricciones de la primera versión

| ID | Restricción | Estado |
| --- | --- | --- |
| C-01 | Aplicación de escritorio para Windows | Confirmado |
| C-02 | Uso local, sin cuentas | Confirmado |
| C-03 | Sin servidores externos | Confirmado |
| C-04 | Sin Spring Boot | Confirmado |
| C-05 | Los cambios de esquema de SQLite se hacen con migraciones versionadas | Confirmado como criterio de desarrollo |
| C-06 | El material multimedia personal no se versiona en Git | Confirmado |
| C-07 | Categorías y colecciones, cuando se confirmen, se añaden con migraciones que crean tablas nuevas | Previsto como forma de evolución. Sus reglas siguen pendientes |

## 9. Entrega

| ID | Trabajo | Estado |
| --- | --- | --- |
| E-01 | Preparar un instalador de Windows | Previsto en la fase 12. El formato concreto está pendiente |
| E-02 | Comprobar el paquete generado | Previsto en la fase 12 |
| E-03 | Incluir las licencias de los componentes externos | Previsto en la fase 12 |
| E-04 | Documentar la instalación | Previsto en la fase 12. Depende de las rutas de datos y de si las herramientas externas van dentro del paquete |

Estos trabajos no están implementados. Están identificados en [ROADMAP.md](ROADMAP.md).

## 10. Ampliaciones futuras

| ID | Ampliación | Estado |
| --- | --- | --- |
| F-01 | Aplicación móvil conectada al ordenador mediante streaming | Futuro |
| F-02 | Identificación automática con MusicBrainz o TMDB | Futuro |
| F-03 | Letras sincronizadas | Futuro |
| F-04 | Ecualizador avanzado | Futuro |
| F-05 | Estadísticas | Futuro |
| F-06 | Otras mejoras de organización y personalización | Futuro |

## 11. Criterios de aceptación globales

Una entrega de la primera versión, cuando el desarrollo llegue a ese punto, tendrá que poder demostrar lo siguiente:

1. Importar un archivo a la biblioteca elegida, copiándolo o moviéndolo con el procedimiento seguro, y ver su ficha con los metadatos leídos y, si existía, la portada extraída. Si el resultado no queda verificado, el original sigue en su sitio.
2. Corregir esa ficha sin alterar el archivo y comprobar que una lectura posterior conserva la corrección. Una imagen personalizada no sustituye el archivo de la portada extraída.
3. Reproducir una canción, cambiar de sección y seguir oyéndola con los controles globales.
4. Mostrar portada o vídeo en el panel derecho y ampliar o minimizar el vídeo sin reiniciar.
5. Reanudar una película en la posición en la que se dejó.
6. Asociar un vídeo a una canción, oír solo el audio de la canción y ver el vídeo asociado siguiendo los controles.
7. Previsualizar un enlace compatible de YouTube y descargarlo, si el uso está autorizado, hacia la biblioteca elegida. Una playlist conserva el orden de origen en el lote.
8. Mantener la ficha cuando el archivo ya no está en la ruta conocida, sin borrar nada por iniciativa de la aplicación.

Ninguno de estos recorridos está hecho. El desarrollo no ha empezado.
