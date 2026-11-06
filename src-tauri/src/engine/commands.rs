//! Comandos del diagnóstico del motor. Se ejecutan fuera del hilo principal porque la superficie
//! nativa necesita ese hilo libre para crearse y moverse.

use tauri::{AppHandle, Manager, WebviewWindow};

use super::surface::{ClientPoint, SurfacePlacement, SurfaceRect, WindowReport};
use super::tracks::TrackReport;
use super::{EngineManager, EngineState, OpenRequest, TrackSelection};

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    task: impl FnOnce(&AppHandle, &EngineManager) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let manager = app.state::<EngineManager>();
        task(&app, &manager)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn engine_open(app: AppHandle, request: OpenRequest) -> Result<EngineState, String> {
    blocking(app, move |app, manager| manager.open(app, request)).await
}

#[tauri::command]
pub async fn engine_stop(app: AppHandle) -> Result<(), String> {
    blocking(app, |app, manager| manager.stop(app)).await
}

#[tauri::command]
pub async fn engine_set_pause(app: AppHandle, paused: bool) -> Result<(), String> {
    blocking(app, move |_, manager| {
        manager.command(&["set", "pause", if paused { "yes" } else { "no" }])
    })
    .await
}

#[tauri::command]
pub async fn engine_seek(app: AppHandle, seconds: f64) -> Result<(), String> {
    blocking(app, move |_, manager| manager.seek(seconds)).await
}

#[tauri::command]
pub async fn engine_status(app: AppHandle) -> Result<EngineState, String> {
    blocking(app, |_, manager| Ok(manager.status())).await
}

#[tauri::command]
pub async fn engine_tracks(app: AppHandle) -> Result<TrackReport, String> {
    blocking(app, |_, manager| manager.tracks()).await
}

#[tauri::command]
pub async fn engine_select_track(
    app: AppHandle,
    kind: TrackSelection,
    id: Option<i64>,
) -> Result<(), String> {
    blocking(app, move |_, manager| manager.select_track(kind, id)).await
}

#[tauri::command]
pub async fn engine_add_subtitle(app: AppHandle, path: String) -> Result<(), String> {
    blocking(app, move |_, manager| manager.add_subtitle(&path)).await
}

#[tauri::command]
pub async fn engine_set_engine_fullscreen(app: AppHandle, enabled: bool) -> Result<(), String> {
    blocking(app, move |_, manager| {
        manager.set_engine_fullscreen(enabled)
    })
    .await
}

/// Pantalla completa de la ventana de Tauri. Devuelve el estado que queda.
#[tauri::command]
pub async fn engine_set_window_fullscreen(
    window: WebviewWindow,
    enabled: bool,
) -> Result<bool, String> {
    window
        .set_fullscreen(enabled)
        .and_then(|()| window.is_fullscreen())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn engine_set_surface(
    app: AppHandle,
    placement: Option<SurfacePlacement>,
) -> Result<(), String> {
    blocking(app, move |app, manager| {
        manager.set_placement(app, placement)
    })
    .await
}

#[tauri::command]
pub async fn engine_set_holes(app: AppHandle, holes: Vec<SurfaceRect>) -> Result<(), String> {
    blocking(app, move |app, manager| manager.set_holes(app, holes)).await
}

#[tauri::command]
pub async fn engine_window_report(
    app: AppHandle,
    points: Vec<ClientPoint>,
) -> Result<WindowReport, String> {
    blocking(app, move |app, manager| manager.window_report(app, points)).await
}
