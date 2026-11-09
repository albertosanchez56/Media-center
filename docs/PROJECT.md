# Media Center — Visión del proyecto

Estado del documento: planificación inicial, con el esquema de datos aprobado.  
Última actualización: 2026-11-09.

Este documento describe qué es Media Center, a quién se dirige la primera versión y qué queda fuera de ella. Los requisitos detallados están en [REQUIREMENTS.md](REQUIREMENTS.md). Las decisiones cerradas y las que siguen abiertas están en [DECISIONS.md](DECISIONS.md).

El esquema de SQLite descrito en [DATABASE.md](DATABASE.md) está aprobado (DC-18). La migración 1 crea la base al arrancar. El volumen se guarda en `app_settings`. F3.1 ya permite elegir archivos o una carpeta y la biblioteca de destino, sin copiarlos ni crear fichas. La copia, el alta y el resto de las fases posteriores no han empezado.

## Qué es

Media Center es una aplicación de escritorio para Windows. Su interfaz será moderna, de tema oscuro, inspirada en la claridad de Spotify y con diseño propio.

Sirve para importar, descargar cuando el uso esté autorizado, organizar, personalizar y reproducir música, vídeos y películas.

La aplicación es local. La primera versión funciona sin cuentas de usuario y sin servidores externos.

## Objetivo

Concentrar en un solo programa la biblioteca personal y la reproducción:

- Elegir si cada importación entra en Música, Vídeos o Películas.
- Conservar y corregir la información de cada ficha.
- Reproducir sin detener la reproducción al cambiar de sección.
- Personalizar portadas y, en las canciones, el vídeo que las acompaña.
- Descargar contenido de enlaces compatibles de YouTube solo cuando los permisos, los derechos y las condiciones aplicables lo permitan.

## Primera versión

La primera versión es la aplicación de escritorio descrita en los requisitos. Incluye las tres bibliotecas, la importación hacia una carpeta administrada, la reproducción, la personalización visual de canciones, el descargador y los metadatos guardados en SQLite.

La entrega también prepara el instalador de Windows, la comprobación del paquete, las licencias de los componentes externos y la documentación de instalación. Ese trabajo está en la fase 12 de [ROADMAP.md](ROADMAP.md). El formato del instalador y el empaquetado de las herramientas externas siguen pendientes.

Forman parte de la dirección del producto, con el detalle todavía abierto:

- Categorías personalizadas.
- Colecciones, como concepto distinto de las playlists.
- Escritura opcional de etiquetas dentro de archivos compatibles.
- Usar un mismo archivo físico en varias bibliotecas. Está confirmado (DC-21) y se implementa en F3.5.

El diseño visual concreto se definirá por etapas. La estructura general de la ventana sí está descrita en los requisitos: menú lateral, zona central, panel derecho de reproducción musical y controles globales.

## Fuera de la primera versión

Estas ideas quedan registradas como ampliaciones futuras. No forman parte del alcance actual:

- Aplicación móvil conectada al ordenador mediante streaming.
- Identificación automática con MusicBrainz o TMDB.
- Letras sincronizadas.
- Ecualizador avanzado.
- Estadísticas de escucha o visionado.
- Otras mejoras de organización y personalización que todavía no se han definido.

Una aplicación móvil con streaming necesitaría un punto de conexión en el ordenador. Eso queda fuera de la primera versión, que no incorpora servidores externos.

## Principios

- El archivo físico y la ficha son identidades distintas. Los metadatos personalizados y las imágenes de cada ficha pertenecen a esa ficha.
- Editar una ficha no modifica el archivo original. La portada incrustada se extrae a la carpeta de imágenes de Media Center y el archivo multimedia queda igual.
- Una imagen personalizada se guarda aparte de la portada extraída.
- Los cambios hechos por la persona usuaria tienen prioridad sobre los metadatos leídos del archivo o de una descarga.
- La aplicación no elimina archivos físicos sin autorización.
- Eliminar una ficha no borra un archivo físico que otra ficha todavía utiliza.
- Copiar o mover no elimina el original mientras el resultado sea incierto.
- Si un archivo desaparece, se renombra o se mueve fuera de la aplicación, las fichas que lo usaban se conservan.
- El material personal no se incluye en Git.

## Tecnologías

| Pieza | Papel | Situación |
| --- | --- | --- |
| Angular y TypeScript | Interfaz | Elegida |
| Tauri 2 y Rust | Aplicación de escritorio, archivos, procesos y persistencia | Elegida |
| SQLite | Biblioteca y metadatos | Elegida. El esquema inicial está aprobado (DC-18) y la migración 1 lo crea al arrancar |
| libmpv | Motor multimedia | Elegido e integrado dentro del proceso de la aplicación (DC-17, F1.5). Falta decidir cómo viaja en el instalador |
| yt-dlp y FFmpeg | Descargas autorizadas | Elegidos como herramientas. Falta decidir si viajan dentro del instalador |
| Lector y editor de etiquetas | Leer metadatos y portadas y, de forma opcional, escribir etiquetas | La extracción de portadas está confirmada. La herramienta concreta sigue pendiente |

La primera versión no usa Spring Boot ni otro servidor de aplicación.

## Personas de uso

Uso personal en un ordenador con Windows. No hay cuentas, perfiles remotos ni sincronización entre dispositivos en esta versión.

## Vocabulario

Estos términos se usan con el mismo significado en el resto de la documentación.

| Término | Significado |
| --- | --- |
| Biblioteca | Una de las tres secciones principales: Música, Vídeos o Películas |
| Ficha | Registro visible en una biblioteca, con sus metadatos, su favorito y sus imágenes |
| Archivo físico | Fichero en disco, con una identidad propia, independiente de cuántas fichas lo usen. Puede estar en la carpeta administrada o, cuando se implemente la referencia externa, en su ruta original |
| Vínculo | Relación entre una ficha y un archivo físico, con un papel: contenido principal o vídeo asociado |
| Carpeta administrada | Directorio elegido por la persona para los archivos copiados o movidos. La ruta propuesta es la carpeta conocida de vídeos más `Media Center`, y no se crea hasta confirmarla |
| Carpeta de imágenes | Directorio de portadas, pósters y miniaturas de la aplicación |
| Playlist | Lista ordenada de fichas |
| Aparición | Una entrada de una ficha dentro de una playlist. La misma ficha puede tener varias apariciones, cada una con su posición |
| Colección | Agrupación mencionada en los requisitos; su diferencia exacta respecto de una playlist está pendiente |
| Categoría | Clasificación personalizada prevista; sus reglas están pendientes |
| Portada extraída | Copia de la imagen incrustada, guardada en la carpeta de imágenes |
| Imagen personalizada | Imagen elegida por la persona usuaria para una ficha, independiente de la portada extraída |
| Vídeo asociado | Vídeo elegido para una canción; se ve durante la reproducción y no aporta el audio |
| Lote de descarga | Grupo de elementos pedidos juntos: un vídeo suelto o una playlist |
| Operación de archivo | Copia o movimiento registrado para poderse recuperar si se interrumpe |
| Descarga autorizada | Descarga de un enlace compatible con YouTube realizada dentro de los permisos, derechos y condiciones aplicables |

## Documentos relacionados

| Documento | Contenido |
| --- | --- |
| [REQUIREMENTS.md](REQUIREMENTS.md) | Requisitos confirmados, previstos y pendientes |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Estructura prevista de la aplicación y procedimientos de archivos |
| [DATABASE.md](DATABASE.md) | Esquema inicial aprobado. La migración 1 está creada |
| [ROADMAP.md](ROADMAP.md) | Fases, tareas y criterios de aceptación |
| [DECISIONS.md](DECISIONS.md) | Decisiones confirmadas y decisiones abiertas |
| [PROGRESS.md](PROGRESS.md) | Estado de avance |
