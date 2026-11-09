//! Comandos de importación. La copia corre en un hilo propio y la interfaz
//! recibe el estado de cada archivo. Solo puede haber una importación a la vez.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::db::Database;
use crate::storage::{self, ManagedFolderState};

use super::{ImportHooks, ItemReport, Mode, ensure_managed_folder, import_paths};

pub struct ImportRuntime {
    active: AtomicBool,
    cancel: AtomicBool,
}

impl Default for ImportRuntime {
    fn default() -> Self {
        Self {
            active: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
        }
    }
}

impl ImportRuntime {
    pub fn try_begin(&self) -> bool {
        self.cancel.store(false, Ordering::SeqCst);
        self.active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn finish(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    pub fn cancel_flag(&self) -> &AtomicBool {
        &self.cancel
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    pub library: String,
    pub mode: String,
    pub kind: String,
    pub paths: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    items: Vec<ItemReport>,
    finished: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStatus {
    pub path: String,
    pub state: String,
}

#[tauri::command]
pub fn import_folder_status(app: AppHandle) -> Result<FolderStatus, String> {
    let storage_file = storage_file(&app)?;
    match storage::load_storage(&storage_file) {
        Ok(config) => {
            let state = match storage::resolve_state(&config) {
                ManagedFolderState::Proposed(_) => "proposed",
                ManagedFolderState::Confirmed(_) => "confirmed",
                ManagedFolderState::Unavailable(_) => "unavailable",
            };
            Ok(FolderStatus {
                path: config.path.display().to_string(),
                state: state.to_string(),
            })
        }
        Err(storage::StorageError::Missing) => Ok(FolderStatus {
            path: String::new(),
            state: "missing".into(),
        }),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
pub fn import_cancel(app: AppHandle) -> Result<(), String> {
    app.state::<ImportRuntime>().request_cancel();
    Ok(())
}

#[tauri::command]
pub fn import_start(app: AppHandle, request: StartRequest) -> Result<(), String> {
    if request.paths.is_empty() {
        return Err("No hay archivos para importar.".into());
    }
    if !matches!(request.library.as_str(), "music" | "video" | "movie") {
        return Err("La biblioteca de destino no es válida.".into());
    }
    if !matches!(request.mode.as_str(), "copy" | "move") {
        return Err("La operación tiene que ser copiar o mover.".into());
    }
    if !matches!(request.kind.as_str(), "files" | "folder") {
        return Err("La selección tiene que ser archivos o una carpeta.".into());
    }
    let runtime = app.state::<ImportRuntime>();
    if !runtime.try_begin() {
        return Err("Ya hay una importación en curso.".into());
    }
    let storage_file = match storage_file(&app) {
        Ok(path) => path,
        Err(error) => {
            runtime.finish();
            return Err(error);
        }
    };
    let forbidden = forbidden_roots(&app);
    let managed = match ensure_managed_folder(&storage_file, &forbidden) {
        Ok(path) => path,
        Err(error) => {
            runtime.finish();
            return Err(error);
        }
    };
    let app_data = match app.path().app_local_data_dir() {
        Ok(path) => path,
        Err(error) => {
            runtime.finish();
            return Err(format!(
                "No se ha podido resolver la carpeta de la aplicación: {error}"
            ));
        }
    };
    let worker = app.clone();
    let started = std::thread::Builder::new()
        .name("media-center-import".into())
        .spawn(move || run_job(worker, managed, app_data, request));
    if let Err(error) = started {
        runtime.finish();
        return Err(format!("No se ha podido iniciar la importación: {error}"));
    }
    Ok(())
}

fn run_job(app: AppHandle, managed: PathBuf, app_data: PathBuf, request: StartRequest) {
    let runtime = app.state::<ImportRuntime>();
    let database = app.state::<Database>();
    struct Guard<'a>(&'a ImportRuntime);
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            self.0.finish();
        }
    }
    let _guard = Guard(&runtime);
    let mode = if request.mode == "move" {
        Mode::Move
    } else {
        Mode::Copy
    };
    let paths: Vec<PathBuf> = request.paths.iter().map(PathBuf::from).collect();
    let mut hooks = ImportHooks::default();
    let emitter = app.clone();
    let reports = import_paths(
        &database,
        &managed,
        &app_data,
        &request.library,
        mode,
        &paths,
        request.kind == "folder",
        runtime.cancel_flag(),
        &mut hooks,
        &mut |items| {
            let _ = emitter.emit(
                "import-progress",
                Progress {
                    items: items.to_vec(),
                    finished: false,
                },
            );
        },
    );
    let _ = app.emit(
        "import-progress",
        Progress {
            items: reports,
            finished: true,
        },
    );
}

fn storage_file(app: &AppHandle) -> Result<PathBuf, String> {
    let local = app.path().app_local_data_dir().map_err(|error| {
        format!("No se ha podido resolver la carpeta de la aplicación: {error}")
    })?;
    Ok(storage::InternalPaths::from_app_local_data(local).storage_file)
}

fn forbidden_roots(app: &AppHandle) -> Vec<PathBuf> {
    let Ok(local) = app.path().app_local_data_dir() else {
        return Vec::new();
    };
    let paths = storage::InternalPaths::from_app_local_data(local);
    let executable = std::env::current_exe().ok();
    storage::forbidden_roots(&paths, executable.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_import_is_rejected_while_one_is_active() {
        let runtime = ImportRuntime::default();
        assert!(runtime.try_begin());
        assert!(!runtime.try_begin());
        runtime.finish();
        assert!(runtime.try_begin());
    }
}
