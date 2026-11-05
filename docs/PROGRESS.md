# Media Center — Progreso

Estado del documento: vivo. Se actualiza al cerrar trabajo de planificación o de desarrollo.  
Última actualización: 2026-11-05.

## Estado general

El desarrollo **ha empezado**. Las tareas F1.1, F1.2 y F1.3 están hechas: Media Center abre como aplicación de escritorio de Windows, muestra el armazón visual de cuatro zonas sin datos reales y, en desarrollo, ofrece una sección de diagnóstico que llama a un comando de prueba de Rust.

No hay funcionalidad de producto. No existen biblioteca, importación, reproducción, descargas ni base de datos. No se han instalado SQLite, libmpv, yt-dlp ni FFmpeg. El esquema de [DATABASE.md](DATABASE.md) sigue pendiente de aprobación y no se ha creado ninguna migración.

El [README.md](../README.md) describe el proyecto, en inglés, como planificación y desarrollo inicial.

## F1.1 Crear el proyecto Tauri 2 y Angular

Hecha el 2026-11-04.

| Pieza | Versión |
| --- | --- |
| Node.js | 22.23.4 |
| npm | 10.9.9 |
| Angular CLI y framework | 22.2.5 y 22.2.x |
| TypeScript | 6.0 |
| Rust y Cargo | 1.99.1, toolchain `stable-x86_64-pc-windows-msvc` |
| Tauri CLI | 2.12.4 |
| Crate `tauri` | 2.12.5 |

Configuración:

- Angular es una SPA con SCSS, sin SSR ni enrutado. Compila en `dist/media-center/browser`.
- Tauri usa el identificador `com.mellk.mediacenter`, con una ventana «Media Center» de 1280×800 redimensionable.
- `frontendDist` apunta a `../dist/media-center/browser` y `devUrl` a `http://localhost:4200`.
- `bundle.active` está en `false`. El formato del instalador sigue pendiente en DP-19.
- Dependencias de Rust: solo `tauri` y `tauri-build`. No hay plugins.
- Dependencias de npm: el núcleo de Angular necesario y `@tauri-apps/cli`. No se instalaron `@angular/router`, `@angular/forms` ni `@tauri-apps/api`.
- La CSP de Tauri queda en `null`, como en la plantilla. Se revisará más adelante.

Comandos:

| Comando | Uso |
| --- | --- |
| `npm run start` | Servidor de desarrollo de Angular |
| `npm run build` | Compilación de Angular |
| `npm test` | Pruebas de Angular con Vitest |
| `npm run tauri:dev` | Aplicación de escritorio en desarrollo |
| `npm run tauri:build` | Ejecutable release sin instalador |

Comprobaciones ejecutadas:

| Comprobación | Resultado |
| --- | --- |
| `npm run build` | Correcta. Genera `dist/media-center/browser/index.html` |
| `npm test -- --watch=false` | 2 de 2 pruebas superadas |
| `cargo fmt --check` | Correcta |
| `cargo check` | Correcta |
| `npm run tauri:build` | Correcta. Genera `src-tauri/target/release/media-center.exe` |
| `npm run tauri:dev` | Abre la ventana «Media Center» con la página de Angular, comprobada con una captura de pantalla |
| `git status` | Solo archivos nuevos. `docs/`, `.cursor/`, `README.md` y `.gitignore` sin cambios. `node_modules/`, `dist/`, `.angular/` y `src-tauri/target/` ignorados |
| `npm install` | 0 vulnerabilidades |

Nota de entorno: Avast inspecciona el HTTPS de este equipo. npm funciona con `NODE_OPTIONS=--use-system-ca`, definida para el usuario. El terminal del agente de Cursor no hereda esa variable, así que en ese terminal hay que definirla en cada proceso.

## F1.2 Comprobar el puente entre interfaz y Rust

Hecha el 2026-11-05.

Implementación:

- Rust: comando `bridge_check(fail: bool) -> Result<String, String>` en `src-tauri/src/commands.rs`, registrado en `lib.rs` con `invoke_handler`. Con `fail = false` devuelve «Respuesta de Rust: puente operativo». Con `fail = true` devuelve un error que empieza por el código `INTENTIONAL_FAILURE: `.
- Angular: `BridgeService` (`src/app/bridge/bridge.service.ts`) es el único punto que llama a `invoke`. Devuelve un resultado de tres tipos: respuesta, fallo provocado (error con el código `INTENTIONAL_FAILURE: `) o error de comunicación (cualquier otro rechazo, incluida la ausencia de Tauri).
- Interfaz: el componente `App` muestra los botones «Probar conexión con Rust» y «Provocar un fallo». Ambos se deshabilitan mientras hay una comprobación en curso. La respuesta se marca en verde, el fallo provocado en ámbar y el error de comunicación en rojo.
- Dependencia nueva de npm: `@tauri-apps/api` 2.12.4, la API oficial para invocar comandos. No se añadieron crates ni plugins, y `capabilities/default.json` no cambió: los comandos propios de la aplicación no necesitan permisos adicionales.

Comprobaciones ejecutadas:

| Comprobación | Resultado |
| --- | --- |
| `npm run build` | Correcta |
| `npm test -- --watch=false` | 10 de 10 pruebas superadas: las 2 anteriores, 4 del componente y 4 del servicio con `mockIPC` de `@tauri-apps/api/mocks` |
| `cargo fmt --check` | Correcta |
| `cargo check` | Correcta |
| `cargo test` | 2 de 2 pruebas superadas |
| `npm run tauri:dev`, botón «Probar conexión con Rust» | La ventana muestra «Respuesta de Rust: puente operativo» |
| `npm run tauri:dev`, botón «Provocar un fallo» | La ventana muestra el fallo provocado en ámbar y la aplicación sigue abierta |
| Página abierta fuera de Tauri, en un navegador | Muestra «Error de comunicación con Tauri» en rojo sin bloquear la página |

Los botones de la ventana de escritorio se pulsaron con UI Automation de Windows y el resultado se comprobó leyendo el texto de la ventana y con capturas. El deshabilitado de los botones durante la llamada se comprueba con una prueba unitaria: en la aplicación real el comando responde demasiado rápido para verlo a simple vista.

Desde F1.3, esta comprobación vive en la sección «Diagnóstico» del armazón.

## F1.3 Montar el armazón visual

Hecha el 2026-11-05.

Implementación:

- Estilos globales: tokens de color, espacio, radios, tipografía y tamaños como propiedades CSS en `src/styles/_tokens.scss`, y mixins de puntos de corte en `src/styles/_breakpoints.scss`. `angular.json` añade `src/styles` a `includePaths`. Tema oscuro único.
- Navegación: `NavigationService` (`src/app/navigation/`) guarda la sección activa en un signal. Las bibliotecas Música, Vídeos y Películas se definen en `navigation.ts`. Música es la sección inicial. No se usa `@angular/router`.
- Componentes compartidos: `IconComponent`, con iconos SVG propios en línea y sin dependencias, y `EmptyStateComponent`.
- Zonas, en `src/app/layout/`:
  - `SidebarComponent`: logotipo y nombre, las tres bibliotecas, la sección de playlists vacía y la entrada «Diagnóstico».
  - `LibraryViewComponent`: encabezado de la biblioteca activa y estado vacío dentro de un contenedor reservado para la futura lista o cuadrícula.
  - `NowPlayingPanelComponent`: «Ahora suena» con un hueco cuadrado para portada o vídeo y el estado «Sin reproducción».
  - `PlayerBarComponent`: «Nada en reproducción», botones anterior, reproducir y siguiente, barra de progreso y volumen, todos deshabilitados.
- `App` es una rejilla CSS con las cuatro zonas. Cada zona tiene su propio desplazamiento y `min-width: 0`.
- Diagnóstico: la interfaz de F1.2 pasa sin cambios de lógica a `BridgeDiagnosticsComponent` (`src/app/diagnostics/`). La entrada solo aparece cuando `isDevMode()` es verdadero, así que no existe en la compilación de producción.
- `tauri.conf.json`: tamaño mínimo de la ventana de 640 × 480 px.
- No se añadieron dependencias.

Comportamiento según el ancho de la ventana:

| Ancho | Menú lateral | Panel derecho | Barra inferior |
| --- | --- | --- | --- |
| 1200 px o más | 240 px, con textos | 320 px | Completa |
| De 900 a 1199 px | 220 px, con textos | 260 px | Completa |
| Menos de 900 px | Barra de iconos de 72 px. Las playlists muestran solo su icono | Oculto | Sin volumen ni texto secundario |

En la barra de iconos los textos siguen en el DOM, ocultos visualmente, y cada botón tiene `title`, así que conserva su nombre accesible. Por debajo de 900 px el panel derecho conserva su área propia en la rejilla y su componente independiente: más adelante podrá mostrarse como panel desplegable o vista alternativa. Eso no está implementado.

Comprobaciones ejecutadas:

| Comprobación | Resultado |
| --- | --- |
| `npm run build` | Correcta |
| `npm test -- --watch=false` | 24 de 24 pruebas superadas en 8 archivos. Las 4 pruebas de la interfaz de F1.2 pasan al spec de diagnóstico y las 4 del servicio no cambian |
| `cargo fmt --check` | Correcta |
| `cargo check` | Correcta |
| `cargo test` | 2 de 2 pruebas superadas |
| `npm run tauri:dev` a 1280 × 800, 1199, 1000, 900, 899, 700 y 640 × 480 px de contenido | Cuatro zonas a partir de 900 px. Cambio de menú y panel en 1200 y en 900 px. Sin solapamientos ni desplazamiento horizontal, también en 640 × 480 |
| Tamaño mínimo | La ventana declara un mínimo de 656 × 519 px exteriores, es decir, 640 × 480 px de contenido con el marco de Windows |
| Navegación | «Vídeos» y «Películas» cambian el título y el estado vacío. «Diagnóstico» muestra la comprobación, y Rust responde con su mensaje y con el fallo provocado |
| Teclado y accesibilidad | Con el menú en iconos, Tab recorre «Música», «Vídeos» y «Películas» con sus nombres, el foco se ve y Enter abre la biblioteca. Los controles de reproducción están deshabilitados y fuera del orden de tabulación |

La ventana se redimensionó con la API de Win32. Los botones se pulsaron con UI Automation y con teclas enviadas a la ventana, y los resultados se comprobaron con capturas. El tamaño mínimo se leyó con el mensaje `WM_GETMINMAXINFO` en lugar de arrastrar el borde a mano.

## Documentación

| Documento | Estado | Notas |
| --- | --- | --- |
| [PROJECT.md](PROJECT.md) | Redactado y revisado | Visión, alcance y vocabulario de archivo, ficha y lote |
| [REQUIREMENTS.md](REQUIREMENTS.md) | Redactado y revisado | Confirmado, previsto, pendiente y futuro |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Redactado y revisado | Estructura del repositorio actualizada con F1.1 |
| [DATABASE.md](DATABASE.md) | Propuesta sin aprobar | No crear tablas a partir de él |
| [ROADMAP.md](ROADMAP.md) | Redactado y revisado | F1.1, F1.2 y F1.3 marcadas como hechas |
| [DECISIONS.md](DECISIONS.md) | Redactado y revisado | Confirmadas y pendientes |
| [PROGRESS.md](PROGRESS.md) | Actualizado | Este documento |

## Fases

| Fase | Tema | Estado |
| --- | --- | --- |
| 0 | Cerrar la planificación | F0.1, F0.3 y F0.4 hechas como documentación. F0.2 pendiente: el esquema no está aprobado |
| 1 | Esqueleto e integración inicial de libmpv | En curso. F1.1, F1.2 y F1.3 hechas. F1.4 y F1.5 pendientes |
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

El detalle de cada tarea está en [ROADMAP.md](ROADMAP.md). Ninguna tarea está en curso en este momento.

## Decisiones que siguen pendientes

| Decisión | Antes de |
| --- | --- |
| [DP-17](DECISIONS.md) Aprobar el esquema | Crear la migración inicial |
| [DP-09](DECISIONS.md) Rutas de datos | Crear la base, la carpeta administrada y la guía de instalación |
| [DP-10](DECISIONS.md) Herramienta de etiquetas | Extraer portadas y rellenar fichas al importar |
| [DP-01](DECISIONS.md) Integración concreta de libmpv | Funciones avanzadas del reproductor. Se cierra en F1.5 |
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

Decisiones ya cerradas: extracción de portadas (DC-12), datos propios de cada ficha (DC-13), copia y movimiento seguros (DC-14), momento de la integración de libmpv (DC-15) y evolución de categorías y colecciones por tablas nuevas (DC-16).

## Siguiente paso

F1.3 está cerrada. El trabajo se detiene antes de F1.4, la incorporación de libmpv en la ventana, a la espera de autorización.

## Historial

| Fecha | Qué cambió |
| --- | --- |
| 2026-11-04 | Se creó la documentación inicial de planificación. |
| 2026-11-04 | Segunda revisión aplicada en los siete documentos. El esquema sigue sin aprobar. |
| 2026-11-04 | F1.1 hecha: Angular 22.2.5 y Tauri 2 inicializados en la raíz. La aplicación abre una ventana de escritorio en Windows con la página de Angular. Sin funcionalidad de producto. |
| 2026-11-05 | F1.2 hecha: comando de prueba `bridge_check` en Rust invocado desde Angular con `@tauri-apps/api`. Respuesta, fallo provocado y error de comunicación se muestran de forma distinta sin cerrar la aplicación. |
| 2026-11-05 | F1.3 hecha: armazón visual oscuro con menú lateral, zona central, panel derecho y barra inferior, adaptable desde 640 × 480 px. La comprobación de F1.2 pasa a la sección «Diagnóstico», visible solo en desarrollo. |
