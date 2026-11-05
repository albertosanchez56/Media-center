# Media Center — Progreso

Estado del documento: vivo. Se actualiza al cerrar trabajo de planificación o de desarrollo.  
Última actualización: 2026-11-05.

## Estado general

El desarrollo **ha empezado**. Las tareas F1.1 y F1.2 están hechas: Media Center abre como aplicación de escritorio de Windows, muestra una página mínima de Angular y esa página llama a un comando de prueba de Rust.

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

## Documentación

| Documento | Estado | Notas |
| --- | --- | --- |
| [PROJECT.md](PROJECT.md) | Redactado y revisado | Visión, alcance y vocabulario de archivo, ficha y lote |
| [REQUIREMENTS.md](REQUIREMENTS.md) | Redactado y revisado | Confirmado, previsto, pendiente y futuro |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Redactado y revisado | Estructura del repositorio actualizada con F1.1 |
| [DATABASE.md](DATABASE.md) | Propuesta sin aprobar | No crear tablas a partir de él |
| [ROADMAP.md](ROADMAP.md) | Redactado y revisado | F1.1 y F1.2 marcadas como hechas |
| [DECISIONS.md](DECISIONS.md) | Redactado y revisado | Confirmadas y pendientes |
| [PROGRESS.md](PROGRESS.md) | Actualizado | Este documento |

## Fases

| Fase | Tema | Estado |
| --- | --- | --- |
| 0 | Cerrar la planificación | F0.1, F0.3 y F0.4 hechas como documentación. F0.2 pendiente: el esquema no está aprobado |
| 1 | Esqueleto e integración inicial de libmpv | En curso. F1.1 y F1.2 hechas. F1.3 a F1.5 pendientes |
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

F1.2 está cerrada. El trabajo se detiene antes de F1.3, el armazón visual, a la espera de autorización.

## Historial

| Fecha | Qué cambió |
| --- | --- |
| 2026-11-04 | Se creó la documentación inicial de planificación. |
| 2026-11-04 | Segunda revisión aplicada en los siete documentos. El esquema sigue sin aprobar. |
| 2026-11-04 | F1.1 hecha: Angular 22.2.5 y Tauri 2 inicializados en la raíz. La aplicación abre una ventana de escritorio en Windows con la página de Angular. Sin funcionalidad de producto. |
| 2026-11-05 | F1.2 hecha: comando de prueba `bridge_check` en Rust invocado desde Angular con `@tauri-apps/api`. Respuesta, fallo provocado y error de comunicación se muestran de forma distinta sin cerrar la aplicación. |
