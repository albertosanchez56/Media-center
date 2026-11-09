mod commands;
#[cfg(all(debug_assertions, windows))]
mod engine;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(all(debug_assertions, windows))]
    let builder = builder
        .manage(engine::EngineManager::default())
        .invoke_handler(tauri::generate_handler![
            commands::bridge_check,
            engine::commands::engine_open,
            engine::commands::engine_stop,
            engine::commands::engine_set_pause,
            engine::commands::engine_seek,
            engine::commands::engine_status,
            engine::commands::engine_tracks,
            engine::commands::engine_select_track,
            engine::commands::engine_add_subtitle,
            engine::commands::engine_set_engine_fullscreen,
            engine::commands::engine_set_window_fullscreen,
            engine::commands::engine_set_surface,
            engine::commands::engine_set_holes,
            engine::commands::engine_window_report,
        ]);

    #[cfg(not(all(debug_assertions, windows)))]
    let builder = builder.invoke_handler(tauri::generate_handler![commands::bridge_check]);

    let app = builder
        .setup(|app| {
            if let Err(error) = storage::prepare_on_startup(app.handle()) {
                eprintln!("No se ha podido preparar el almacenamiento interno: {error}");
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Media Center");

    app.run(|_app, _event| {
        #[cfg(all(debug_assertions, windows))]
        if let tauri::RunEvent::Exit = _event {
            use tauri::Manager;
            _app.state::<engine::EngineManager>().shutdown_on_exit();
        }
    });
}
