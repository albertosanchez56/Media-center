# Media Center — Progreso

Estado del documento: vivo. Se actualiza al cerrar trabajo de planificación o de desarrollo.  
Última actualización: 2026-11-06.

## Estado general

El desarrollo **ha empezado**. Las tareas F1.1 a F1.4 están hechas. Con F1.1 a F1.3, Media Center abre como aplicación de escritorio de Windows, muestra el armazón visual de cuatro zonas sin datos reales y, en desarrollo, ofrece una sección de diagnóstico que llama a un comando de prueba de Rust.

F1.4 está **hecha**: las dos integraciones experimentales de mpv (A, libmpv dentro del proceso, y B, `mpv.exe` controlado por IPC) reproducen archivos locales dentro de la ventana y se han verificado en la aplicación. Solo existen en la compilación de desarrollo, dentro de «Diagnóstico», y se conservan para F1.5. A es la candidata provisional y B la alternativa documentada. DP-01 sigue pendiente.

No hay funcionalidad de producto. No existen biblioteca, importación, reproducción para la persona usuaria, descargas ni base de datos. No se han instalado SQLite, yt-dlp ni FFmpeg como herramientas. libmpv y mpv.exe solo están en una carpeta local ignorada por Git. El esquema de [DATABASE.md](DATABASE.md) sigue pendiente de aprobación y no se ha creado ninguna migración.

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

## F1.4 Incorporar libmpv en la ventana de Media Center

Hecha el 2026-11-06, tras la revisión y aprobación de sus resultados. No se ha elegido integración y no se ha empezado F1.5. El código experimental y el diagnóstico del motor se mantienen como base de estudio para F1.5.

### Binarios de terceros

Se guardan en `src-tauri/vendor/mpv/`, ignorada por Git (`/src-tauri/vendor/` en `.gitignore`). No forman parte del repositorio.

| Archivo | Origen | Licencia | SHA-256 |
| --- | --- | --- | --- |
| `libmpv-v0.42.0-x86_64-w64-mingw32-lgpl.zip` | Versión v0.42.0 de mpv-player/mpv en GitHub | LGPL: mpv con `-Dgpl=false` y FFmpeg LGPL v3 o posterior, sin `--enable-gpl` ni `--enable-nonfree` | `8E5F77430EFC53EF175804BA6618527E3AF0448A0DCABF04D3CC72659D1A6FCE` |
| `libmpv/libmpv-2.dll` (extraída) | El archivo anterior | La misma | `94FB87247732C59915CD59808BC582E843A71C22214B7B6C1E5563AA8149CEF4` |
| `mpv-v0.42.0-x86_64-pc-windows-msvc.zip` | Versión v0.42.0 de mpv-player/mpv en GitHub | GPL (`gpl=true`). Solo para pruebas locales | `8DD69CBAB079086765FA5D873C09513B2834B85D146C1301718A757E8C0595EF` |
| `mpv/mpv.exe` (extraído) | El archivo anterior | La misma | `2431F60D186EB57C01F86D65A8B2F7EF3EEFC4AF70564A749E6B06E4D315F8F2` |

Los SHA-256 de los dos ZIP coinciden con los que publica GitHub. Las firmas `.asc` son válidas con la clave `D83F25E44BD6B948724AB75F6580132338ABD4E2`, publicada en GitHub por kasper93, mantenedor de mpv, que subió esas firmas. Se comprobaron con un llavero temporal, sin tocar el del usuario. La imagen exacta de FFmpeg que usó la compilación figura en un registro de CI que exige iniciar sesión en GitHub, así que no se ha anotado. Si mpv.exe acabara en el instalador, su licencia GPL entraría en DP-20.

### Implementación

Todo el motor experimental se compila solo con `debug_assertions` en Windows. En release no existe.

- Rust, `src-tauri/src/engine/`:
  - `mod.rs`: trait `EngineBackend`, `EngineManager` con una sola reproducción a la vez, y la orden de salto `seek <s> relative+exact`.
  - `libmpv.rs` (A): carga `libmpv-2.dll` con `libloading` al abrir, exige la API 2.x y lee los eventos en un hilo propio.
  - `mpv_process.rs` (B): lanza `mpv.exe` con `--wid`, `--no-config` y una tubería con nombre propia, y usa dos conexiones JSON IPC, una para órdenes y otra para eventos.
  - `surface.rs`: ventana hija Win32 `MediaCenterVideoSurface` donde dibuja mpv, recortada con `SetWindowRgn`.
  - `tracks.rs`: lectura de pistas y del estado del audio.
  - `commands.rs`: comandos `engine_*` de Tauri.
- Angular:
  - `EngineService` (`src/app/engine/`): única vía hacia los comandos `engine_*`.
  - Directiva `appVideoSurface`: sigue el recuadro reservado, envía su parte visible y desplaza la página cuando la rueda gira sobre el vídeo.
  - `DiagnosticsViewComponent` agrupa la tarjeta del puente de F1.2 y la nueva `EngineDiagnosticsComponent`.
  - `describeError` (`src/app/shared/`) lo comparten los dos servicios.
- Dependencias nuevas de Rust: `serde`, `serde_json`, `libloading` 0.9 y `windows` 0.62, ya presente en el árbol de Tauri, con las características de Win32 necesarias. No hay dependencias nuevas de npm.
- Las zonas de F1.3 no se han modificado.

### Correcciones tras la primera prueba manual

| Problema | Causa | Corrección |
| --- | --- | --- |
| El vídeo se dibujaba sobre la barra inferior al desplazar | La ventana nativa está por encima de WebView2 y el `overflow` del HTML no la recorta | La directiva calcula la parte visible del recuadro (contenedores que recortan y ventana). Rust recorta la superficie a esa parte menos los huecos. Al moverla aplica primero la zona visible en las dos posiciones, después la mueve y por último aplica la región definitiva |
| La rueda no desplazaba sobre el vídeo | Windows entrega la rueda a la ventana de vídeo de mpv, que no la reenvía | Se deshabilita la ventana de mpv con `EnableWindow(false)`, así el ratón llega a la superficie propia. Esta emite `engine-surface-wheel` y la directiva desplaza 100 px por paso el contenedor desplazable más cercano |
| Un MP4 no se abría sin un MP3 | El MP4 se escribía en el campo del vídeo asociado y «Abrir» exige el archivo principal | Etiquetas «Archivo principal (audio o vídeo)» y «Vídeo asociado (solo imagen, opcional)», aviso cuando falta el principal y limpieza de las comillas de «Copiar como ruta de acceso» |
| El audio desaparecía tras saltar ±10 s | Los saltos `relative` van al fotograma clave. Con vídeo asociado, mpv retiene el audio hasta que el vídeo lo alcanza, y con un GOP largo el MP4 cae en otro fotograma clave | Saltos `relative+exact` en A y B |

### Comprobaciones automáticas

| Comprobación | Resultado |
| --- | --- |
| `npm run build` | Correcta |
| `npm test -- --watch=false` | 53 de 53 pruebas en 11 archivos |
| `cargo fmt --check` | Correcta |
| `cargo check` y `cargo check --release` | Correctas |
| `cargo test` | 26 de 26 pruebas |

Pruebas de regresión añadidas: orden de salto exacto y rechazo de valores no finitos, recorte de la superficie (entera, bajo la barra, por arriba y durante el movimiento), estado del audio en el informe de pistas, posición `null` y rueda en el servicio, y en la directiva la parte visible, la ocultación, los huecos, el tamaño cero y un único desplazamiento por paso de rueda. En el componente: MP4 sin vídeo asociado, comillas y aviso de archivo principal.

### Verificación manual

Se hizo en `npm run tauri:dev`, a 96 ppp, con archivos sintéticos generados fuera del repositorio (`%TEMP%\mc-f14`): MP3, MP4 con y sin audio y MP4 con un fotograma clave cada 10 s. Solo se manejaron sesiones lanzadas para la prueba. La interfaz se manejó con UI Automation, y la rueda, los clics y las teclas con eventos reales de Win32. Las comprobaciones visuales se hicieron con capturas de pantalla.

Saltos de ±10 s, medidos primero con `mpv.exe` fuera de la aplicación:

| Orden | MP3 con vídeo asociado (fotograma clave cada 10 s) | MP4 independiente |
| --- | --- | --- |
| `relative` | El audio queda retenido (`audio-pts` no disponible) y la posición cae en el fotograma clave | Cae en el fotograma clave siguiente o retiene el audio 2,5 s al retroceder |
| `relative+exact` | El audio vuelve entre 1 y 35 ms después | Igual. Sin coste apreciable |

| Comprobación | A, libmpv | B, mpv.exe |
| --- | --- | --- |
| Saltos en MP3, MP4 y MP3 con vídeo asociado | `audio-pts` a la par de la posición (0,01–0,05 s), `aid` 1, sin silenciar, volumen 100, salida `wasapi` | Igual |
| MP4 con su audio sin vídeo asociado | Se abre | Se abre |
| Vídeo asociado | Audio solo del archivo principal y vídeo externo seleccionado. Desfase 0,000 s tras pausa y saltos | Igual |
| Pausa | La posición no avanza en 2 s y continúa al reanudar | Igual |
| Desplazamiento continuo con la rueda | 0 px del vídeo sobre la barra inferior o fuera de la zona central en todas las capturas: 120 sobre el HTML, sobre el vídeo y con huecos (72) | 0 px en 120 capturas sobre el HTML, 36 sobre el vídeo y 72 entrando por abajo |
| Recorte por arriba | Correcto en capturas | Correcto en capturas |
| Ventana de vídeo de mpv | Deshabilitada; el centro del vídeo da la superficie propia | Deshabilitada aunque es de otro proceso; igual |
| Un paso de rueda sobre el vídeo | 100 px; 16 de 16 alternando sentido, sin doble desplazamiento | 100 px; 16 de 16 |
| Rueda sobre un control superpuesto | 100 px | 100 px |
| Controles HTML con huecos | Clic, opción de menú y tres teclas con ratón y teclado reales | Igual |
| Controles HTML sin huecos | El vídeo los tapa y no reciben el clic, como se espera | Igual |
| Pequeño, Mediano y Ampliado | 320×180, 580×327 y 1264×697 sin reiniciar; la posición avanza y «cargados» sigue en 1/1 | Igual y con el mismo proceso de mpv |
| 1280×800, 1199×800, 1000×760, 899×700, 656×519, maximizar, minimizar y restaurar | Superficie y recuadro coinciden (desfase 0, a veces 1 px de alto). Minimizada, la posición avanza | Igual |
| Presentación «flip» activada | El recorte y los huecos se respetan (0 px en 72 capturas) | Igual; mpv recibió `--d3d11-flip=yes` |
| Ruta inexistente | «No existe el archivo: …» y nada en reproducción | Igual |
| Detener | La reproducción termina | La reproducción termina y el proceso de mpv también |
| Cerrar Media Center con un vídeo sonando | Se cierra en 241 ms con código 0 | Se cierra en 252 ms con código 0 |
| Recursos propios tras el cierre | No queda `media-center.exe` ni su WebView2 | No queda `media-center.exe`, su WebView2, su `mpv.exe` ni la tubería `media-center-mpv-<pid>-<n>` |

La prueba de cierre se hizo el 2026-11-06 con `video-gop10-con-audio.mp4` reproduciéndose y cerrando la ventana con UI Automation. Antes y después se listaron los procesos `mpv.exe`: los dos que había de origen desconocido, ajenos a la prueba, siguieron igual y no se tocaron. Con B, el `mpv.exe` de la sesión era hijo de Media Center y desapareció con el cierre. La prueba no distingue si terminó por la orden `quit` del cierre ordenado o al destruirse su ventana: en los dos casos no queda ningún proceso huérfano.

Observaciones de la medición:

- Al restaurar desde maximizada, una lectura inmediata dio la superficie oculta con otra posición. Es una carrera de la medición: pulsar «Informe de ventanas» desplaza la página después de tomar el informe. Repetida tras una espera, la superficie coincide con el recuadro en A y en B.
- El informe cuenta siempre una «ventana suelta», `Tao Thread Event Target`. Es una ventana interna de Tauri sin área, no de mpv. Es un falso positivo del filtro del informe.

### Limitaciones que permanecen

- La superficie nativa sigue al HTML con un pequeño retraso, porque su posición viaja de Angular a Rust en cada cambio. No se ha visto ningún píxel fuera de su sitio, pero el retraso existe.
- La rueda sobre el vídeo se reenvía con un paso fijo de 100 px por cada 120 unidades. No reproduce el desplazamiento suave de WebView2 ni la aceleración del sistema.
- Para que los controles HTML se vean sobre el vídeo hay que recortarle huecos. Cualquier control que deba superponerse al vídeo necesita esa marca.
- Solo se ha probado a 96 ppp (escala del 100 %) y en un monitor. Faltan otras escalas, varios monitores y mover la ventana entre monitores con escalas distintas.
- Con «flip», el recorte se ha respetado en este equipo. Es la observación de un solo equipo y no está demostrado en otras tarjetas gráficas ni controladores. En el diagnóstico la opción figura como experimental y avisa de que su compatibilidad puede variar según el equipo.
- mpv.exe de la integración B es GPL. La DLL de A es LGPL.

### Candidata provisional

A, libmpv dentro de la aplicación, es la candidata provisional desde el 2026-11-06. No es la integración elegida: DP-01 sigue pendiente y se cierra en F1.5.

### Qué cubre F1.4 y qué queda para después

F1.4 ha comprobado que un archivo local se reproduce en la ventana de Media Center con dos integraciones, y ha medido lo anotado arriba: superficie, recorte, rueda, tamaños, saltos, vídeo asociado, errores y cierre. Todo ocurre en el diagnóstico de desarrollo, con rutas escritas a mano.

F1.4 **no** ha construido ni probado:

- el reproductor del producto: controlador único en Rust, barra inferior, panel derecho y reproducción que sobrevive a la navegación (F7.1 a F7.3);
- ampliar y minimizar el vídeo de una canción en el diseño real (F7.4). Los tamaños del recuadro de diagnóstico solo prueban que se puede cambiar el tamaño sin reiniciar;
- cola, aleatorio y repetición (F7.5);
- minirreproductor, pantalla completa, elección de pista de audio y subtítulos (F7.6);
- reanudar películas (F7.7);
- asociar un vídeo a una ficha y su preferencia o duración distinta (F8.1 a F8.3). F1.4 solo probó el vídeo asociado escrito a mano.

### Pruebas pendientes para las funciones avanzadas

No se ha implementado nada de esto. La lista recoge lo que habrá que comprobar y de qué depende. Las pruebas de viabilidad con la candidata sirven para cerrar DP-01 en F1.5. La implementación completa corresponde a las tareas indicadas.

| Función | Pruebas necesarias | Depende de |
| --- | --- | --- |
| Subtítulos | Listar las pistas de subtítulos de un MKV o MP4 con varias, activar una, cambiarla y desactivarla sin reiniciar. Comprobar que se dibujan dentro de la superficie, respetan el recorte y los huecos y se escalan con el tamaño. Probar textos con acentos y alfabetos no latinos. Si DP-14 admite archivos externos, cargar un `.srt` y un `.ass` aparte | DP-14 (origen de los subtítulos), R-10, archivos de prueba con subtítulos y F7.6 |
| Pistas de audio | Con un archivo de dos o más pistas de audio, cambiar de pista sin perder la posición ni reiniciar. Con vídeo asociado, confirmar que solo suena el archivo principal | R-10 y F7.6 |
| Pantalla completa | Entrar y salir sin reiniciar la reproducción: misma posición y «cargados» en 1/1. Comprobar si la propiedad `fullscreen` de mpv tiene efecto con `--wid` o si hay que poner a pantalla completa la ventana de Tauri con la superficie ocupándola; no se ha probado. Comprobar Esc, los controles HTML con huecos y la vuelta al tamaño anterior, también en un segundo monitor | F7.6 y la API de ventana de Tauri |
| Escalas y monitores | Escalas del 125 %, 150 % y 200 %, comprobando que la superficie coincide con el recuadro (la directiva convierte a píxeles físicos con `devicePixelRatio`). Dos monitores con escalas distintas y la ventana movida de uno a otro durante la reproducción | Un equipo o una configuración con esas escalas y monitores |
| Ampliar y minimizar en el diseño real | Pasar el vídeo de una canción del panel derecho a la vista ampliada y volver, sin reiniciar el audio | F7.3, F7.4 y F1.5 |
| Minirreproductor | Película o vídeo que continúa en un minirreproductor al cambiar de sección y vuelve sin reiniciar | F7.2, F7.6, DP-15 y F1.5 |
| Vídeo asociado del producto | Vídeo asociado en silencio que sigue a la canción en pausa, reanudación y saltos, con vuelta a la portada | F8.1, F8.2, F7.3 y F1.5 |
| Reproductor definitivo | Una sola reproducción al navegar, estado publicado desde Rust y cierre ordenado del motor | F7.1, F7.2 y F1.5 |
| «flip» de D3D11 en otros equipos | Repetir la prueba de recorte en otra tarjeta gráfica o controlador antes de activarla por defecto | Otro equipo de prueba |
| Distribución | Licencia de la DLL que viaje en el instalador y su procedencia verificada | DP-20 y F12 |

### Criterios de aceptación de F1.4

| Criterio de [ROADMAP.md](ROADMAP.md) | Resultado |
| --- | --- |
| Un archivo local se abre en la ventana de Media Center | Cumplido con A y con B |
| Queda anotado qué opciones se han probado y qué se ha observado | Cumplido en este documento y en DP-01 |
| DP-01 sigue sin cerrarse | Cumplido: A es solo candidata provisional |
| No existe un proyecto de pruebas independiente para el reproductor | Cumplido: todo vive en la aplicación. Los scripts de medición y los medios sintéticos son temporales y están fuera del repositorio |

## Documentación

| Documento | Estado | Notas |
| --- | --- | --- |
| [PROJECT.md](PROJECT.md) | Redactado y revisado | Visión, alcance y vocabulario de archivo, ficha y lote |
| [REQUIREMENTS.md](REQUIREMENTS.md) | Redactado y revisado | Confirmado, previsto, pendiente y futuro |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Redactado y revisado | Incluye el motor experimental de F1.4 |
| [DATABASE.md](DATABASE.md) | Propuesta sin aprobar | No crear tablas a partir de él |
| [ROADMAP.md](ROADMAP.md) | Redactado y revisado | F1.1, F1.2, F1.3 y F1.4 hechas |
| [DECISIONS.md](DECISIONS.md) | Redactado y revisado | Confirmadas y pendientes |
| [PROGRESS.md](PROGRESS.md) | Actualizado | Este documento |

## Fases

| Fase | Tema | Estado |
| --- | --- | --- |
| 0 | Cerrar la planificación | F0.1, F0.3 y F0.4 hechas como documentación. F0.2 pendiente: el esquema no está aprobado |
| 1 | Esqueleto e integración inicial de libmpv | En curso. F1.1, F1.2, F1.3 y F1.4 hechas. F1.5 pendiente |
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
| [DP-01](DECISIONS.md) Integración concreta de libmpv | Funciones avanzadas del reproductor. Se cierra en F1.5. Las observaciones de F1.4 están en DECISIONS |
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

F1.4 está cerrada. El siguiente paso es planificar F1.5, la elección documentada de la integración que cierra DP-01, partiendo de A como candidata provisional, B como alternativa y las pruebas pendientes de la sección de F1.4. El trabajo se detiene antes de F1.5 a la espera de autorización.

## Historial

| Fecha | Qué cambió |
| --- | --- |
| 2026-11-04 | Se creó la documentación inicial de planificación. |
| 2026-11-04 | Segunda revisión aplicada en los siete documentos. El esquema sigue sin aprobar. |
| 2026-11-04 | F1.1 hecha: Angular 22.2.5 y Tauri 2 inicializados en la raíz. La aplicación abre una ventana de escritorio en Windows con la página de Angular. Sin funcionalidad de producto. |
| 2026-11-05 | F1.2 hecha: comando de prueba `bridge_check` en Rust invocado desde Angular con `@tauri-apps/api`. Respuesta, fallo provocado y error de comunicación se muestran de forma distinta sin cerrar la aplicación. |
| 2026-11-05 | F1.3 hecha: armazón visual oscuro con menú lateral, zona central, panel derecho y barra inferior, adaptable desde 640 × 480 px. La comprobación de F1.2 pasa a la sección «Diagnóstico», visible solo en desarrollo. |
| 2026-11-06 | F1.4 en revisión: integraciones experimentales A (libmpv) y B (mpv.exe por IPC) en el diagnóstico de desarrollo. Corregidos el vídeo sobre la barra al desplazar, la rueda sobre el vídeo, la apertura de un MP4 solo y el audio tras los saltos. DP-01 sigue pendiente. |
| 2026-11-06 | F1.4: etiqueta de «flip» corregida, prueba de cierre con reproducción activa en A y B, A como candidata provisional y pruebas pendientes para las funciones avanzadas documentadas. |
| 2026-11-06 | F1.4 hecha tras la aprobación de sus resultados. A es la candidata provisional y B la alternativa. DP-01 sigue abierta hasta F1.5. |
