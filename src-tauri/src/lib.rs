mod commands;
mod db;
#[cfg(all(debug_assertions, windows))]
mod engine;
mod import;
mod storage;
mod tags;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(import::ImportRuntime::default());

    #[cfg(all(debug_assertions, windows))]
    let builder = builder
        .manage(engine::EngineManager::default())
        .invoke_handler(tauri::generate_handler![
            commands::bridge_check,
            import::commands::import_folder_status,
            import::commands::import_start,
            import::commands::import_cancel,
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
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::bridge_check,
        import::commands::import_folder_status,
        import::commands::import_start,
        import::commands::import_cancel,
    ]);

    let app = builder
        .setup(|app| {
            use tauri::Manager;
            if let Err(error) = storage::prepare_on_startup(app.handle()) {
                let message = format!(
                    "No se ha podido preparar el almacenamiento interno: {error}. La base de datos no se abre."
                );
                eprintln!("{message}");
                return Err(message.into());
            }
            let local = app.path().app_local_data_dir().map_err(|error| {
                let message = format!(
                    "No se ha podido resolver el directorio local de la aplicación: {error}. La base de datos no se abre."
                );
                eprintln!("{message}");
                message
            })?;
            let database_path = storage::InternalPaths::from_app_local_data(&local).database;
            let database = match db::initialize(&database_path) {
                Ok(database) => database,
                Err(error) => {
                    let message = format!(
                        "No se ha podido inicializar la base de datos en {}. Los datos existentes se conservan. {error}",
                        database_path.display()
                    );
                    eprintln!("{message}");
                    return Err(message.into());
                }
            };
            if let Err(error) = database.check_ready() {
                let message = format!(
                    "La base de datos en {} no ha quedado operativa. Los datos existentes se conservan. {error}",
                    database_path.display()
                );
                eprintln!("{message}");
                return Err(message.into());
            }
            if let Err(error) = import::recover_on_startup(&database, &local) {
                eprintln!(
                    "No se han podido revisar las importaciones pendientes: {error}. No se borra ningún archivo."
                );
            }
            app.manage(database);
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
