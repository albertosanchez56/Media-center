# Media Center — Progreso

Estado del documento: vivo. Se actualiza al cerrar trabajo de planificación o de desarrollo.  
Última actualización: 2026-11-04.

## Estado general

El desarrollo de la aplicación **no ha empezado**.

Hay documentación de planificación, incluida la segunda revisión, y no hay código de producción. No se han instalado dependencias, no existe proyecto Angular, Tauri ni Rust, y no hay una base de datos creada. El esquema de [DATABASE.md](DATABASE.md) sigue pendiente de aprobación. No se ha creado ninguna migración.

El [README.md](../README.md) describe el proyecto, en inglés, como planificación y desarrollo inicial. En el repositorio, el desarrollo de la aplicación todavía no tiene código.

Ninguna función de biblioteca, reproducción, descarga o instalación está implementada. Documentarla no la marca como hecha.

## Documentación

| Documento | Estado | Notas |
| --- | --- | --- |
| [PROJECT.md](PROJECT.md) | Redactado y revisado | Visión, alcance y vocabulario de archivo, ficha y lote |
| [REQUIREMENTS.md](REQUIREMENTS.md) | Redactado y revisado | Confirmado, previsto, pendiente y futuro |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Redactado y revisado | Procedimiento seguro, recuperación y lotes |
| [DATABASE.md](DATABASE.md) | Propuesta sin aprobar | Separación de archivo y ficha incluida. No crear tablas a partir de él |
| [ROADMAP.md](ROADMAP.md) | Redactado y revisado | Fases 0 a 12. La integración de libmpv queda al comienzo del desarrollo |
| [DECISIONS.md](DECISIONS.md) | Redactado y revisado | Confirmadas y pendientes |
| [PROGRESS.md](PROGRESS.md) | Redactado y revisado | Este documento |

## Fases

| Fase | Tema | Estado |
| --- | --- | --- |
| 0 | Cerrar la planificación | F0.1, F0.3 y F0.4 hechas como documentación. F0.2 pendiente: el esquema no está aprobado |
| 1 | Esqueleto e integración inicial de libmpv | No iniciada |
| 2 | Persistencia | No iniciada. Bloqueada por la aprobación del esquema y por la ruta de los datos |
| 3 | Importación | No iniciada. La segunda ficha por archivo sigue bloqueada |
| 4 | Consulta de la biblioteca | No iniciada |
| 5 | Edición de metadatos e imágenes | No iniciada |
| 6 | Playlists | No iniciada. Colecciones y categorías bloqueadas |
| 7 | Reproducción | No iniciada. Las funciones avanzadas esperan la elección de F1.5 |
| 8 | Vídeo asociado | No iniciada |
| 9 | Archivos cambiados fuera de la aplicación | No iniciada |
| 10 | Descargas autorizadas | No iniciada |
| 11 | Etiquetas opcionales y cierre funcional | No iniciada |
| 12 | Distribución para Windows | No iniciada |

El detalle de cada tarea está en [ROADMAP.md](ROADMAP.md). Ninguna tarea de implementación está en curso.

## Decisiones que siguen pendientes

| Decisión | Antes de |
| --- | --- |
| [DP-17](DECISIONS.md) Aprobar el esquema | Crear la migración inicial |
| [DP-09](DECISIONS.md) Rutas de datos | Crear la base, la carpeta administrada y la guía de instalación |
| [DP-10](DECISIONS.md) Herramienta de etiquetas | Extraer portadas y rellenar fichas al importar |
| [DP-01](DECISIONS.md) Integración concreta de libmpv | Funciones avanzadas del reproductor. Se cierra en F1.5, al comienzo del desarrollo, y hoy sigue abierta |
| [DP-04](DECISIONS.md) Referencias externas | Ofrecer un modo distinto de copiar y mover |
| [DP-05](DECISIONS.md) Identidad de un archivo renombrado | Reconciliar rutas automáticamente |
| [DP-06](DECISIONS.md) Duplicados de descarga | Avisar de una descarga repetida |
| [DP-07](DECISIONS.md) Vídeo asociado más corto | Elegir bucle, último fotograma o preferencia de la ficha |
| [DP-08](DECISIONS.md) Preferencia visual de la canción | Decidir si el vídeo asociado sustituye solo la imagen |
| [DP-12](DECISIONS.md) Búsqueda, filtros, repetición y cola | Concretar esos controles |
| [DP-13](DECISIONS.md) Reanudación fuera de las películas | Guardar la posición de vídeos o canciones |
| [DP-14](DECISIONS.md) Origen de los subtítulos | Elegir pistas incrustadas, archivos externos o ambos |
| [DP-15](DECISIONS.md) Barra musical y minirreproductor | Cerrar el diseño de esa convivencia |
| [DP-16](DECISIONS.md) Formatos y calidades de descarga | Mostrar la lista en la interfaz |
| [DP-18](DECISIONS.md) Varias fichas para un mismo archivo | Ofrecer esa función. El modelo propuesto ya la admite |
| [DP-02](DECISIONS.md) y [DP-03](DECISIONS.md) Categorías y colecciones | Definir sus reglas. La migración posterior solo añadiría tablas nuevas |
| [DP-11](DECISIONS.md) Escritura opcional de etiquetas | Ofrecer esa acción aparte |
| [DP-19](DECISIONS.md) Formato del instalador | Generar el paquete de Windows |
| [DP-20](DECISIONS.md) Empaquetado de herramientas externas | Decidir si viajan dentro del instalador |

Decisiones ya cerradas que esta revisión incorpora: extracción de portadas (DC-12), datos propios de cada ficha (DC-13), copia y movimiento seguros (DC-14), momento de la integración de libmpv (DC-15) y evolución de categorías y colecciones por tablas nuevas (DC-16). El motor previsto sigue siendo libmpv y la opción concreta no está elegida.

## Comprobaciones

No hay pruebas automatizadas ni aplicación que ejecutar. En esta actualización se ha revisado la coherencia entre los siete documentos. No se ha comprobado ningún comportamiento de producto.

## Siguiente paso

La segunda revisión de la documentación está aplicada. El trabajo se detiene aquí, antes de la fase 1.

Cuando se autorice el desarrollo, el siguiente trabajo sería el esqueleto de la aplicación y la integración inicial de libmpv dentro de esa misma ventana. La migración de SQLite espera a que el esquema se apruebe. No forma parte de este cierre.

## Historial

| Fecha | Qué cambió |
| --- | --- |
| 2026-11-04 | Se creó la documentación inicial de planificación. El desarrollo de la aplicación permanece no iniciado. |
| 2026-11-04 | Segunda revisión aplicada en los siete documentos: archivo físico separado de la ficha, apariciones repetidas, lotes de descarga, movimiento seguro con recuperación, extracción de portadas confirmada, migraciones futuras de categorías y colecciones, integración temprana de libmpv y fase 12 de distribución. El esquema sigue sin aprobar y ninguna función está implementada. |
