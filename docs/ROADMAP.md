# Media Center — Hoja de ruta

Estado del documento: planificación inicial, con la segunda revisión aplicada. El desarrollo ha empezado con la fase 1.  
Última actualización: 2026-11-05.

Cada tarea es un incremento pequeño. No se inicia una tarea hasta cerrar las decisiones de las que depende. El seguimiento vivo está en [PROGRESS.md](PROGRESS.md).

Leyenda de estados: pendiente, bloqueada, en curso, hecha.

Las tareas hechas de la fase 0 son documentación. F1.1 dejó una aplicación de escritorio vacía. F1.2 añadió un comando de prueba de Rust invocado desde la interfaz. No hay funcionalidad de biblioteca, reproducción, importación ni descargas implementada.

## Fase 0 — Cerrar la planificación

Objetivo: dejar por escrito qué se va a construir y qué decisiones bloquean el primer código.

### F0.1 Redactar la documentación inicial

- **Estado:** hecha
- **Depende de:** nada
- **Descripción:** Crear la visión, los requisitos, la arquitectura, la propuesta de datos, esta hoja de ruta, las decisiones y el progreso.
- **Criterios de aceptación:**
  - Existen los siete documentos de `docs/`.
  - Los requisitos distinguen lo confirmado, lo previsto, lo pendiente y lo futuro.
  - El esquema de datos figura como propuesta pendiente de aprobación.
  - El progreso marca el desarrollo como no iniciado.

### F0.2 Revisar y aprobar la propuesta de datos

- **Estado:** pendiente
- **Depende de:** F0.4
- **Descripción:** Responder las preguntas finales de [DATABASE.md](DATABASE.md) y marcar el esquema como aprobado o sustituirlo.
- **Criterios de aceptación:**
  - Cada pregunta de aprobación tiene una respuesta anotada en [DECISIONS.md](DECISIONS.md).
  - [DATABASE.md](DATABASE.md) pasa a esquema aprobado o se reescribe con el acuerdo.
  - Categorías y colecciones quedan aplazadas a una migración de tablas nuevas, o se incluye otra decisión explícita.
  - La separación entre archivo físico y ficha, los lotes y el diario quedan aceptados o sustituidos por escrito.
- **Nota:** La segunda revisión no completa esta tarea. El esquema sigue sin aprobar.

### F0.3 Dejar escritas las condiciones de la integración de libmpv

- **Estado:** hecha
- **Depende de:** F0.1
- **Descripción:** Anotar las opciones y las condiciones que la integración debe cumplir, sin elegir todavía una opción.
- **Criterios de aceptación:**
  - [DECISIONS.md](DECISIONS.md) lista las opciones y las condiciones en DP-01.
  - Ninguna opción queda marcada como elegida.
  - La elección está asignada a F1.5, antes de las funciones avanzadas del reproductor.

### F0.4 Aplicar la segunda revisión de la documentación

- **Estado:** hecha
- **Depende de:** F0.1
- **Descripción:** Incorporar la separación de archivos y fichas, las apariciones repetidas, los lotes de descarga, el procedimiento seguro, la extracción confirmada de portadas, las migraciones futuras y la fase de distribución.
- **Criterios de aceptación:**
  - Los siete documentos de `docs/` reflejan esa revisión y se citan entre sí.
  - Lo pendiente sigue identificado como pendiente.
  - El esquema de SQLite no figura como aprobado.
  - El progreso no marca como implementada ninguna función de la aplicación.

## Fase 1 — Esqueleto de la aplicación e integración inicial

Objetivo: abrir una ventana de escritorio, comprobar el puente con Rust e incorporar libmpv en esa misma aplicación antes del resto del producto.

### F1.1 Crear el proyecto Tauri 2 y Angular

- **Estado:** hecha
- **Depende de:** F0.4
- **Descripción:** Generar la aplicación de escritorio con Angular, TypeScript, Tauri 2 y Rust, sin funciones de biblioteca todavía.
- **Criterios de aceptación:**
  - La aplicación abre una ventana en Windows.
  - El proyecto distingue el código de interfaz y el código Rust.
  - No se añade Spring Boot ni un servidor externo.
  - No se crea un segundo proyecto para el reproductor.

### F1.2 Comprobar el puente entre interfaz y Rust

- **Estado:** hecha
- **Depende de:** F1.1
- **Descripción:** Añadir un comando sencillo de Rust invocado desde Angular y devolver un resultado visible.
- **Criterios de aceptación:**
  - Una acción de la interfaz llama a Rust y muestra la respuesta.
  - El fallo del comando, si se provoca, se muestra sin cerrar la aplicación.

### F1.3 Montar el armazón visual

- **Estado:** pendiente
- **Depende de:** F1.1
- **Descripción:** Colocar menú lateral, zona central, panel derecho y barra inferior, con tema oscuro y sin datos reales.
- **Criterios de aceptación:**
  - Las cuatro zonas se reconocen con la ventana ancha.
  - Al reducir la ventana, el contenido se reajusta y sigue pudiéndose usar el menú y la zona central.
  - La barra inferior y el panel derecho permanecen reservados a la reproducción musical.

### F1.4 Incorporar libmpv en la ventana de Media Center

- **Estado:** pendiente
- **Depende de:** F1.1 y F0.3
- **Descripción:** Estudiar las opciones de DP-01 dentro de esta aplicación y conseguir reproducir un archivo local en su ventana. Es el comienzo del trabajo de integración, no un prototipo aparte.
- **Criterios de aceptación:**
  - Un archivo local se abre en la ventana de Media Center.
  - Queda anotado qué opciones se han probado y qué se ha observado.
  - DP-01 sigue sin cerrarse en esta tarea.
  - No existe un proyecto de pruebas independiente para el reproductor.

### F1.5 Elegir y documentar la integración concreta

- **Estado:** pendiente
- **Depende de:** F1.4
- **Descripción:** Cerrar DP-01 con la opción que cumpla las condiciones ya escritas. Esta tarea ocurre antes de las funciones avanzadas del reproductor.
- **Criterios de aceptación:**
  - DP-01 pasa a confirmada, con la opción elegida y las opciones descartadas.
  - La decisión explica cómo se ampliará y se minimizará el vídeo sin reiniciar.
  - La decisión explica cómo el vídeo asociado permanecerá en silencio y seguirá la posición de la canción.
  - Las tareas F7.4, F7.6, F8.2 y F8.3 siguen sin empezarse hasta este cierre.

## Fase 2 — Persistencia

Objetivo: crear la base aprobada, en una ruta decidida, mediante migraciones.

### F2.1 Decidir dónde viven la base, los medios y las imágenes

- **Estado:** pendiente
- **Depende de:** F0.2
- **Descripción:** Fijar el directorio de SQLite, el de la carpeta administrada y el de las imágenes.
- **Criterios de aceptación:**
  - Las tres rutas quedan escritas en [DECISIONS.md](DECISIONS.md).
  - Esas rutas quedan fuera de Git.
  - La aplicación puede crearlas si no existen.

### F2.2 Aplicar la primera migración

- **Estado:** bloqueada
- **Depende de:** F0.2 y F2.1
- **Descripción:** Crear el módulo de acceso a SQLite y la migración inicial con las tablas aprobadas.
- **Criterios de aceptación:**
  - Al arrancar, la base existe y contiene la versión de migración aplicada.
  - Volver a arrancar no vuelve a aplicar esa migración ni modifica los datos.
  - La migración no incluye las tablas futuras de categorías ni de colecciones.
  - Una migración posterior de prueba queda registrada con un número nuevo. Esa prueba puede retirarse antes de dar la fase por cerrada si no forma parte del esquema aprobado.

### F2.3 Ajustes locales mínimos

- **Estado:** bloqueada
- **Depende de:** F2.2
- **Descripción:** Leer y guardar un ajuste sencillo, como el volumen, en la tabla de ajustes aprobada.
- **Criterios de aceptación:**
  - El valor guardado se recupera al reiniciar la aplicación.
  - Un valor ausente usa un valor por defecto sin fallar.

## Fase 3 — Importación

Objetivo: incorporar archivos a la biblioteca elegida y crear su ficha, con el procedimiento seguro.

### F3.1 Elegir archivos, carpetas y biblioteca

- **Estado:** pendiente
- **Depende de:** F1.2
- **Descripción:** Abrir el diálogo del sistema para uno o varios archivos y para una carpeta, y pedir la biblioteca de destino.
- **Criterios de aceptación:**
  - Se puede elegir un archivo, varios archivos o una carpeta.
  - La biblioteca de destino es una de las tres y la elige la persona usuaria.
  - Cancelar el diálogo no crea fichas ni copia archivos.

### F3.2 Copiar o mover a la carpeta administrada

- **Estado:** bloqueada
- **Depende de:** F2.1, F2.2 y F3.1
- **Descripción:** Ejecutar la operación elegida según DC-14 y registrar el archivo físico y la ficha principal.
- **Criterios de aceptación:**
  - El diario de la operación queda confirmado antes de alterar el origen.
  - En el mismo volumen, un movimiento directo se usa cuando el nombre está libre y el renombrado puede recuperarse. Si no, se usa copia, verificación y borrado posterior del original.
  - Entre volúmenes distintos se copia a un temporal, se verifica el tamaño y el original solo se elimina después del alta confirmada.
  - Un nombre ocupado no se sobrescribe.
  - Sin espacio suficiente el original permanece y no queda un temporal a medias sin registrar.
  - Un fallo de SQLite no elimina el original.
  - Al volver a abrir la aplicación, una operación incompleta se recupera o pasa a revisión. En revisión el original no se borra solo.
  - Copiar deja el original en su sitio.
  - La aplicación no borra otros archivos.

### F3.3 Leer metadatos y portada incrustada

- **Estado:** bloqueada
- **Depende de:** F3.2 y de la herramienta de lectura de etiquetas
- **Descripción:** Rellenar la ficha con lo que el archivo ya contenga y extraer la portada incrustada a la carpeta de imágenes.
- **Criterios de aceptación:**
  - Un archivo con título, artista y portada muestra esos datos en la ficha nueva.
  - La portada extraída queda referenciada desde la ficha y el archivo multimedia conserva tamaño y fecha de modificación.
  - Un archivo sin metadatos crea igualmente la ficha, con el nombre de archivo como título provisional.
  - La imagen personalizada, cuando exista más adelante, no ocupa el sitio del archivo de la portada extraída.

Elegir la herramienta de etiquetas es una decisión previa a esta tarea. Puede hacerse durante la fase 3, antes de implementarla. DC-12 ya fija el comportamiento.

### F3.4 Importar un MP4 a cualquiera de las tres bibliotecas

- **Estado:** bloqueada
- **Depende de:** F3.2
- **Descripción:** Comprobar que el contenedor no decide la biblioteca.
- **Criterios de aceptación:**
  - El mismo tipo de archivo MP4 puede guardarse como música, como vídeo o como película según la elección hecha al importar.
  - La importación crea una ficha y aparece en la biblioteca elegida.

### F3.5 Segunda ficha sobre el mismo archivo físico

- **Estado:** bloqueada
- **Depende de:** la confirmación funcional DP-18 y de F3.2
- **Descripción:** Si se confirma, crear otra ficha en otra biblioteca reutilizando el archivo físico ya administrado. Si se descarta, cerrar la tarea como no aplicable.
- **Criterios de aceptación:**
  - Con la opción aprobada, la segunda ficha tiene sus propios metadatos y sus propias imágenes, y el fichero no se duplica.
  - Eliminar una de las dos fichas deja el archivo disponible para la otra.
  - Con la opción descartada, la importación sigue creando una sola ficha y la tarea se documenta como cerrada sin esa función.

## Fase 4 — Consulta de la biblioteca

Objetivo: ver, buscar y filtrar lo importado.

### F4.1 Listar una biblioteca en lista y en cuadrícula

- **Estado:** bloqueada
- **Depende de:** F3.2 y F1.3
- **Descripción:** Mostrar las fichas de la biblioteca seleccionada en el menú lateral.
- **Criterios de aceptación:**
  - Música, Vídeos y Películas muestran solo sus fichas.
  - La persona puede alternar entre lista y cuadrícula.
  - Una biblioteca vacía muestra un estado vacío comprensible.

### F4.2 Buscar

- **Estado:** bloqueada
- **Depende de:** F4.1 y de los campos de búsqueda acordados
- **Descripción:** Filtrar las fichas visibles por un texto.
- **Criterios de aceptación:**
  - El texto encuentra fichas por los campos que se hayan acordado.
  - Borrar el texto restaura el listado de la biblioteca.
  - Una búsqueda sin resultados tiene un estado propio.

### F4.3 Filtros y favoritos

- **Estado:** bloqueada
- **Depende de:** F4.1
- **Descripción:** Marcar favoritos y aplicar los filtros que se hayan definido.
- **Criterios de aceptación:**
  - Marcar y desmarcar un favorito se conserva al reiniciar.
  - El filtro de favoritos muestra solo las fichas marcadas.
  - Cada filtro adicional acordado se puede activar y quitar.

### F4.4 Ficha de detalle

- **Estado:** bloqueada
- **Depende de:** F4.1
- **Descripción:** Abrir una ficha y ver sus metadatos, su imagen y el estado del archivo.
- **Criterios de aceptación:**
  - La ficha muestra los datos guardados y la imagen seleccionada.
  - Si el archivo físico está marcado como no disponible, la ficha lo indica y sigue pudiéndose abrir.

## Fase 5 — Edición de metadatos e imágenes

Objetivo: corregir la información de la ficha sin tocar el archivo.

### F5.1 Editar los campos de la ficha

- **Estado:** bloqueada
- **Depende de:** F4.4 y F0.2
- **Descripción:** Guardar cambios de título y del resto de campos aprobados para ese tipo de ficha.
- **Criterios de aceptación:**
  - Los cambios se ven al reabrir la ficha y al reiniciar.
  - El archivo multimedia conserva su contenido anterior.
  - Una nueva lectura del archivo no pisa los campos editados de esa ficha.

### F5.2 Sustituir portada o póster

- **Estado:** bloqueada
- **Depende de:** F5.1
- **Descripción:** Elegir una imagen nueva para una canción, un vídeo o una película.
- **Criterios de aceptación:**
  - La imagen nueva se guarda en la carpeta de imágenes, pertenece a esa ficha y pasa a ser la seleccionada.
  - La portada extraída sigue existiendo como imagen no seleccionada.
  - La ficha muestra la imagen nueva en lista o cuadrícula y en el detalle.
  - El archivo multimedia no se modifica.

### F5.3 Completar información de vídeo y película

- **Estado:** bloqueada
- **Depende de:** F5.1
- **Descripción:** Editar descripción y póster de vídeos y películas.
- **Criterios de aceptación:**
  - La descripción y el póster se conservan en SQLite y en disco, asociados a esa ficha.
  - Una canción no exige descripción para poderse guardar.

## Fase 6 — Playlists

Objetivo: agrupar fichas en un orden de reproducción.

### F6.1 Crear y eliminar playlists

- **Estado:** bloqueada
- **Depende de:** F4.1 y de la decisión sobre si una playlist mezcla bibliotecas
- **Descripción:** Altas, cambios de nombre y bajas de playlists en el menú lateral.
- **Criterios de aceptación:**
  - Una playlist nueva aparece en el menú lateral.
  - Se puede renombrar.
  - Eliminar la playlist no elimina las fichas ni sus archivos.

### F6.2 Ordenar elementos de una playlist

- **Estado:** bloqueada
- **Depende de:** F6.1
- **Descripción:** Añadir, quitar y reordenar apariciones.
- **Criterios de aceptación:**
  - La misma ficha puede aparecer más de una vez.
  - Cada aparición conserva su propia posición al reiniciar.
  - Quitar una aparición deja las demás, la ficha y el archivo físico.
  - Reordenar cambia las posiciones sin perder apariciones.

### F6.3 Colecciones

- **Estado:** bloqueada
- **Depende de:** una definición de colección en [DECISIONS.md](DECISIONS.md)
- **Descripción:** Implementar colecciones según la definición que se apruebe, mediante una migración que solo añada tablas nuevas.
- **Criterios de aceptación:**
  - Los criterios funcionales se escribirán al aprobar la definición.
  - La migración no modifica columnas de las tablas iniciales para representar la colección.
  - Hasta entonces, la tarea no se da por empezada.

### F6.4 Categorías personalizadas

- **Estado:** bloqueada
- **Depende de:** las reglas de categorías en [DECISIONS.md](DECISIONS.md)
- **Descripción:** Implementar categorías según las reglas que se aprueben, mediante una migración que solo añada tablas nuevas.
- **Criterios de aceptación:**
  - Los criterios funcionales se escribirán al aprobar las reglas.
  - La migración no modifica columnas de las tablas iniciales para representar la categoría.
  - Hasta entonces, la tarea no se da por empezada.

## Fase 7 — Reproducción

Objetivo: oír y ver con un solo controlador que sobrevive a la navegación, sobre la integración ya elegida.

F1.4 y F1.5 se hacen al comienzo del desarrollo. Las tareas de esta fase completan el reproductor después de esa elección. No sustituyen a F1.4 ni piden otro proyecto.

### F7.1 Reproducir el archivo principal

- **Estado:** bloqueada
- **Depende de:** F1.5, F3.2 y F1.2
- **Descripción:** Abrir el archivo principal de una ficha con la integración ya documentada.
- **Criterios de aceptación:**
  - Una canción de un formato que el motor admita se oye.
  - Un vídeo o una película de un formato admitido se ve y se oye.
  - Si el motor rechaza el formato, la ficha permanece y la interfaz explica que no puede reproducirse.

### F7.2 Mantener una sola reproducción al navegar

- **Estado:** bloqueada
- **Depende de:** F7.1 y F1.3
- **Descripción:** Encargar el estado de reproducción a Rust y reflejarlo en los controles globales.
- **Criterios de aceptación:**
  - Iniciar otra ficha detiene la anterior. No se oyen dos elementos a la vez.
  - Cambiar de biblioteca o de playlist no detiene lo que está sonando.
  - Pausa, reanudación y salto de posición actúan sobre ese mismo elemento.

### F7.3 Barra musical y panel derecho

- **Estado:** bloqueada
- **Depende de:** F7.2
- **Descripción:** Mostrar la canción actual en la barra inferior y su portada o su vídeo en el panel derecho.
- **Criterios de aceptación:**
  - Durante una canción se ven el título y los controles en la barra inferior.
  - El panel derecho muestra la imagen seleccionada de la ficha.
  - Una canción MP4 sin vídeo asociado muestra su vídeo en el panel derecho.

### F7.4 Ampliar y minimizar el vídeo

- **Estado:** bloqueada
- **Depende de:** F7.3 y F1.5
- **Descripción:** Cambiar el tamaño de presentación del vídeo de la canción con la integración elegida.
- **Criterios de aceptación:**
  - Ampliar y después minimizar conserva la posición de reproducción.
  - El audio no se reinicia al cambiar de tamaño.

### F7.5 Cola, aleatorio y repetición

- **Estado:** bloqueada
- **Depende de:** F7.2 y de los modos de repetición acordados
- **Descripción:** Preparar la cola de la selección actual y aplicar aleatorio y repetición. La cola respeta cada aparición de una playlist.
- **Criterios de aceptación:**
  - Al terminar un elemento empieza el siguiente de la cola, si lo hay.
  - Dos apariciones de la misma ficha se reproducen como dos puestos distintos.
  - El modo aleatorio cambia el orden de esa cola.
  - Cada modo de repetición acordado se puede activar y desactivar, y su efecto se observa al terminar el elemento o la cola.

### F7.6 Películas y vídeos en primer plano y en minirreproductor

- **Estado:** bloqueada
- **Depende de:** F7.2, F1.5 y del diseño de convivencia de los reproductores
- **Descripción:** Ver la película o el vídeo en la sección correspondiente y reducirlo al salir de ella.
- **Criterios de aceptación:**
  - Al cambiar de sección, la película o el vídeo sigue en un minirreproductor.
  - Volver a su sección recupera la presentación principal sin reiniciar.
  - Pantalla completa, elección de pista de audio y subtítulos disponibles funcionan sobre esa reproducción.

### F7.7 Reanudar películas

- **Estado:** bloqueada
- **Depende de:** F7.6 y F2.2
- **Descripción:** Guardar la posición de una película y usarla la siguiente vez.
- **Criterios de aceptación:**
  - Cerrar la reproducción de una película y volver a abrirla empieza cerca de la posición guardada.
  - La posición se conserva después de reiniciar la aplicación.
  - Una canción no cambia de comportamiento por esta tarea.

## Fase 8 — Vídeo asociado a una canción

Objetivo: acompañar una canción con un vídeo elegido, sin usar el audio de ese vídeo.

### F8.1 Asociar y quitar un vídeo

- **Estado:** bloqueada
- **Depende de:** F5.1 y F3.2
- **Descripción:** Elegir un archivo de vídeo como vídeo asociado de una canción y poder retirar el vínculo.
- **Criterios de aceptación:**
  - La canción guarda el vínculo al vídeo asociado.
  - Quitar el vínculo no borra el archivo si otra ficha lo utiliza, ni sin autorización expresa.
  - El archivo de la canción no se modifica.

### F8.2 Reproducir el vídeo asociado en silencio

- **Estado:** bloqueada
- **Depende de:** F8.1, F7.3 y F1.5
- **Descripción:** Mostrar el vídeo asociado en lugar de la portada o del vídeo original, según la preferencia de la ficha.
- **Criterios de aceptación:**
  - Se oye el audio de la canción.
  - No se oye el audio del vídeo asociado.
  - Pausar, reanudar y saltar de posición mueven a la vez el audio y el vídeo.
  - Se puede volver a la portada o al vídeo original si la ficha lo permite.

### F8.3 Duración distinta del vídeo asociado

- **Estado:** bloqueada
- **Depende de:** F8.2 y de la decisión sobre el bucle
- **Descripción:** Aplicar la política aprobada cuando el vídeo dura menos que la canción.
- **Criterios de aceptación:**
  - Los criterios concretos se completarán al elegir si el vídeo se repite, se detiene en el último fotograma o queda a elección de cada ficha.
  - La política elegida no corta el audio de la canción.

## Fase 9 — Archivos cambiados fuera de la aplicación

Objetivo: reconocer desapariciones y movimientos sin borrar fichas ni archivos.

### F9.1 Marcar archivos desaparecidos

- **Estado:** bloqueada
- **Depende de:** F3.2 y del método de comprobación
- **Descripción:** Detectar que el archivo físico ya no está en su ruta.
- **Criterios de aceptación:**
  - Las fichas vinculadas siguen existiendo.
  - La interfaz indica que el archivo no está disponible.
  - La aplicación no borra fichas, imágenes ni otros archivos por este motivo.

### F9.2 Reconocer un archivo renombrado o movido

- **Estado:** bloqueada
- **Depende de:** F9.1 y de la decisión de identidad del archivo
- **Descripción:** Volver a enlazar el archivo físico cuando el mismo contenido aparece en otra ruta conocida.
- **Criterios de aceptación:**
  - Se actualiza la ruta del archivo físico.
  - Las fichas que ya lo usaban conservan sus metadatos editados y sus imágenes.
  - La reconciliación no crea una ficha nueva.
  - Si la identidad no es concluyente, el archivo permanece como no disponible y la aplicación no adivina el emparejamiento.

### F9.3 Referencias externas

- **Estado:** bloqueada
- **Depende de:** la decisión de permitir o descartar archivos fuera de la carpeta administrada
- **Descripción:** Si se aprueba, importar sin copiar ni mover y reconciliar esas rutas. Si se descarta, cerrar la tarea como no aplicable.
- **Criterios de aceptación:**
  - Con la opción aprobada, un archivo referenciado se reproduce desde su ruta original y el registro guarda esa relación.
  - Con la opción descartada, la importación sigue ofreciendo solo copiar y mover, y la tarea se documenta como cerrada sin esa función.

## Fase 10 — Descargas autorizadas

Objetivo: incorporar enlaces compatibles de YouTube con vista previa, lotes y ficha resultante.

### F10.1 Comprobar yt-dlp y FFmpeg

- **Estado:** pendiente
- **Depende de:** F1.2
- **Descripción:** Localizar ambas herramientas y explicar su ausencia.
- **Criterios de aceptación:**
  - Si están disponibles, la aplicación muestra que puede descargar.
  - Si falta alguna, la interfaz lo indica y el resto de la biblioteca sigue funcionando.

### F10.2 Vista previa de un enlace

- **Estado:** bloqueada
- **Depende de:** F10.1
- **Descripción:** Mostrar título y metadatos disponibles antes de descargar un vídeo o una playlist.
- **Criterios de aceptación:**
  - Un enlace individual muestra su vista previa sin crear un archivo.
  - Un enlace de playlist muestra los elementos y su orden.
  - Un enlace no válido muestra el error y no crea una ficha.

### F10.3 Descargar audio o audio y vídeo

- **Estado:** bloqueada
- **Depende de:** F10.2, F3.3 y F2.1
- **Descripción:** Crear un lote y un elemento por posición de origen, y descargar hacia la biblioteca elegida.
- **Criterios de aceptación:**
  - Se puede elegir solo audio, o audio y vídeo.
  - Se elige la biblioteca de destino antes de empezar.
  - Un vídeo suelto queda como lote de un elemento.
  - Cada elemento de una playlist conserva la posición de la vista previa.
  - Al completar un elemento, el archivo está en la carpeta administrada y existen su archivo físico y su ficha.
  - La miniatura descargada queda como imagen de esa ficha.
  - Durante la descarga se ve el progreso del elemento y el progreso global del lote.

### F10.4 Cancelar y reintentar

- **Estado:** bloqueada
- **Depende de:** F10.3
- **Descripción:** Controlar un elemento o el lote entero.
- **Criterios de aceptación:**
  - Cancelar un elemento detiene ese elemento y no deja una ficha a medias.
  - Cancelar el lote detiene lo que sigue pendiente y conserva lo ya completado, con sus posiciones.
  - Un elemento fallido puede reintentarse en la misma fila y la misma posición.
  - Otro elemento del lote puede continuar después de una cancelación o un fallo.

### F10.5 Duplicados

- **Estado:** bloqueada
- **Depende de:** F10.3 y de la regla de duplicados
- **Descripción:** Avisar o impedir una descarga que coincida con la regla aprobada.
- **Criterios de aceptación:**
  - Un duplicado según la regla aprobada se señala antes de crear otra ficha.
  - La persona puede cancelar esa descarga.
  - Lo que no cumpla la regla se descarga con normalidad.
  - El identificador del lote no se usa como regla de duplicado.

### F10.6 Playlist local desde una playlist importada

- **Estado:** bloqueada
- **Depende de:** F10.3 y F6.2
- **Descripción:** Crear una playlist de Media Center con las fichas descargadas, en la posición original del lote.
- **Criterios de aceptación:**
  - Las fichas completadas quedan en el orden de su posición de origen.
  - Los elementos fallidos no desaparecen del lote y no se presentan como descargados.
  - Una misma ficha puede ocupar más de un puesto si el origen la repetía.

## Fase 11 — Etiquetas opcionales y cierre funcional

### F11.1 Escribir etiquetas bajo petición

- **Estado:** bloqueada
- **Depende de:** F5.1 y de la decisión sobre escritura de etiquetas
- **Descripción:** Ofrecer, como acción aparte, guardar los metadatos de la ficha dentro de un archivo compatible.
- **Criterios de aceptación:**
  - La acción no ocurre al editar la ficha ni al extraer la portada.
  - Hace falta una confirmación antes de modificar el archivo.
  - Si el formato no es compatible, el archivo queda igual y la interfaz lo explica.
  - Tras una escritura aceptada, los campos escritos coinciden con la ficha.

### F11.2 Recorrido de la primera versión

- **Estado:** bloqueada
- **Depende de:** las tareas confirmadas de las fases 1 a 10 que no se hayan aplazado por decisión
- **Descripción:** Recorrer los criterios globales de [REQUIREMENTS.md](REQUIREMENTS.md) con archivos de prueba.
- **Criterios de aceptación:**
  - Se cumplen los ocho criterios globales de aceptación.
  - Las funciones aplazadas por una decisión explícita no se exigen en este recorrido.
  - Los fallos encontrados quedan anotados antes de dar el recorrido por cerrado.

## Fase 12 — Distribución para Windows

Objetivo: preparar el instalador, comprobar el paquete, reunir las licencias y documentar la instalación. Esta fase no sustituye a las funciones anteriores.

### F12.1 Preparar el instalador de Windows

- **Estado:** bloqueada
- **Depende de:** F1.1 y de DP-19
- **Descripción:** Configurar el empaquetado de Windows de la aplicación de escritorio.
- **Criterios de aceptación:**
  - El formato elegido en DP-19 genera un paquete.
  - El paquete no incluye bibliotecas, descargas ni bases personales.
  - El paquete no instala un servidor.

### F12.2 Comprobar el paquete generado

- **Estado:** bloqueada
- **Depende de:** F12.1 y F11.2
- **Descripción:** Instalar, arrancar y desinstalar el paquete en un entorno de prueba.
- **Criterios de aceptación:**
  - La instalación termina y la aplicación abre una ventana.
  - Tras desinstalar, la aplicación deja de arrancar desde el acceso instalado.
  - Los medios personales de prueba no viajan dentro del paquete.

### F12.3 Reunir las licencias de los componentes externos

- **Estado:** bloqueada
- **Depende de:** F1.1, F1.5 y DP-20
- **Descripción:** Inventariar las licencias de las dependencias de la aplicación y de libmpv, FFmpeg, yt-dlp y la herramienta de etiquetas que se hayan incorporado.
- **Criterios de aceptación:**
  - Cada componente incluido tiene su licencia localizada.
  - El inventario distingue lo que viaja en el paquete de lo que la instalación documenta como requisito externo.
  - El inventario se entrega con la aplicación o con la documentación de instalación.

### F12.4 Documentar la instalación

- **Estado:** bloqueada
- **Depende de:** F12.1, DP-09 y DP-20
- **Descripción:** Redactar cómo se instala Media Center en Windows y dónde quedan los datos.
- **Criterios de aceptación:**
  - La guía indica el sistema de destino y los pasos de instalación.
  - La guía indica la ruta de la base, de la carpeta administrada y de las imágenes decidida en DP-09.
  - La guía explica si libmpv, FFmpeg y yt-dlp van dentro del paquete o deben estar disponibles aparte.
  - La guía no describe un servidor ni una cuenta de usuario.

## Fuera de esta hoja de ruta

No tienen tareas de implementación en la primera versión:

- Aplicación móvil y streaming.
- MusicBrainz y TMDB.
- Letras sincronizadas.
- Ecualizador avanzado.
- Estadísticas.

Podrán entrar en una hoja de ruta posterior cuando se decida abordarlas.

## Orden recomendado

1. La documentación de planificación queda en F0.1, F0.3 y F0.4. F0.2, la aprobación del esquema, sigue pendiente y es previa a la migración.
2. Al autorizar el desarrollo, empezar por la fase 1: ventana, puente, armazón, integración de libmpv en esa ventana y elección documentada en F1.5.
3. Construir la persistencia (fase 2) solo después de aprobar el esquema.
4. Importar, consultar, editar y crear playlists (fases 3 a 6).
5. Completar la reproducción y el vídeo asociado (fases 7 y 8) con la integración ya elegida.
6. Añadir la reconciliación de archivos y las descargas por lotes (fases 9 y 10).
7. Dejar la escritura opcional de etiquetas y el recorrido funcional (fase 11).
8. Preparar el instalador, las comprobaciones, las licencias y la guía (fase 12).
