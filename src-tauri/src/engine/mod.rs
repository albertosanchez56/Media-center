//! Motor multimedia experimental de F1.4. Solo se compila en desarrollo y en Windows.

pub mod commands;
mod ipc;
mod libmpv;
mod locate;
mod mpv_process;
mod state;
mod surface;
mod tracks;

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use tauri::{AppHandle, Emitter};

use libmpv::LibmpvBackend;
use mpv_process::MpvProcessBackend;
use state::{STATE_EVENT, Shared, lock};
use surface::{ClientPoint, SurfacePlacement, SurfaceRect, VideoSurface, WindowReport};
use tracks::TrackReport;

pub use state::{EngineKind, EngineState};

/// Operaciones mínimas comunes a las dos integraciones; el resto se expresa con órdenes de mpv.
trait EngineBackend: Send {
    fn command(&self, args: &[&str]) -> Result<(), String>;
    fn get_property(&self, name: &str) -> Result<Option<String>, String>;
    fn shutdown(&mut self);
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRequest {
    pub kind: EngineKind,
    pub path: String,
    pub associated_video: Option<String>,
    pub d3d11_flip: bool,
}

/// Pistas que se eligen desde el diagnóstico; el vídeo lo decide el vídeo asociado.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TrackSelection {
    Audio,
    Subtitle,
}

struct Session {
    backend: Box<dyn EngineBackend>,
    shared: Arc<Shared>,
}

/// Gestor de la única reproducción. Abrir otra sesión detiene la anterior antes de crearla.
#[derive(Default)]
pub struct EngineManager {
    session: Mutex<Option<Session>>,
    surface: Mutex<Option<VideoSurface>>,
    placement: Mutex<Option<SurfacePlacement>>,
    holes: Mutex<Vec<SurfaceRect>>,
}

impl EngineManager {
    pub fn open(&self, app: &AppHandle, request: OpenRequest) -> Result<EngineState, String> {
        require_file(&request.path)?;
        if let Some(video) = &request.associated_video {
            require_file(video)?;
        }

        let mut session = lock(&self.session);
        if let Some(mut previous) = session.take() {
            previous.backend.shutdown();
        }

        let wid = self.with_surface(app, |surface| Ok(surface.wid()))?;
        self.place_surface(app, true)?;

        let shared = Shared::new(
            app.clone(),
            EngineState {
                kind: Some(request.kind),
                path: Some(request.path.clone()),
                associated_video: request.associated_video.clone(),
                ..Default::default()
            },
        );
        let started: Result<Box<dyn EngineBackend>, String> = match request.kind {
            EngineKind::Libmpv => LibmpvBackend::start(wid, request.d3d11_flip, shared.clone())
                .map(|b| Box::new(b) as _),
            EngineKind::MpvProcess => {
                MpvProcessBackend::start(wid, request.d3d11_flip, shared.clone())
                    .map(|b| Box::new(b) as _)
            }
        };
        let backend = match started {
            Ok(backend) => backend,
            Err(error) => {
                drop(session);
                let _ = self.place_surface(app, false);
                return Err(error);
            }
        };
        if let Err(error) = backend.command(&["loadfile", &request.path]) {
            let mut backend = backend;
            backend.shutdown();
            drop(session);
            let _ = self.place_surface(app, false);
            return Err(format!("mpv rechazó el archivo: {error}"));
        }

        let snapshot = shared.snapshot();
        *session = Some(Session { backend, shared });
        Ok(snapshot)
    }

    pub fn stop(&self, app: &AppHandle) -> Result<(), String> {
        let previous = lock(&self.session).take();
        if let Some(mut previous) = previous {
            previous.backend.shutdown();
        }
        self.place_surface(app, false)?;
        let _ = app.emit(STATE_EVENT, EngineState::default());
        Ok(())
    }

    /// Al salir se ejecuta en el hilo principal, así que no puede tocar la superficie.
    pub fn shutdown_on_exit(&self) {
        if let Some(mut previous) = lock(&self.session).take() {
            previous.backend.shutdown();
        }
    }

    pub fn command(&self, args: &[&str]) -> Result<(), String> {
        let session = lock(&self.session);
        let session = session
            .as_ref()
            .ok_or("No hay ninguna reproducción abierta")?;
        session.backend.command(args)
    }

    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        let command = seek_command(seconds)?;
        let args: Vec<&str> = command.iter().map(String::as_str).collect();
        self.command(&args)
    }

    /// `None` desactiva la pista: sin subtítulos o sin audio.
    pub fn select_track(&self, kind: TrackSelection, id: Option<i64>) -> Result<(), String> {
        let command = track_command(kind, id)?;
        let args: Vec<&str> = command.iter().map(String::as_str).collect();
        self.command(&args)
    }

    pub fn add_subtitle(&self, path: &str) -> Result<(), String> {
        require_file(path)?;
        self.command(&["sub-add", path, "select"])
    }

    /// Propiedad `fullscreen` de mpv; con `--wid` se prueba si tiene algún efecto.
    pub fn set_engine_fullscreen(&self, enabled: bool) -> Result<(), String> {
        self.command(&["set", "fullscreen", if enabled { "yes" } else { "no" }])
    }

    pub fn status(&self) -> EngineState {
        lock(&self.session)
            .as_ref()
            .map(|session| session.shared.snapshot())
            .unwrap_or_default()
    }

    pub fn tracks(&self) -> Result<TrackReport, String> {
        let session = lock(&self.session);
        let session = session
            .as_ref()
            .ok_or("No hay ninguna reproducción abierta")?;
        tracks::read_tracks(|name| session.backend.get_property(name))
    }

    /// Guarda el hueco que reserva la interfaz y la parte que se ve; `None` si no se ve nada.
    /// La superficie solo se muestra con una sesión abierta.
    pub fn set_placement(
        &self,
        app: &AppHandle,
        placement: Option<SurfacePlacement>,
    ) -> Result<(), String> {
        *lock(&self.placement) = placement.map(SurfacePlacement::validate).transpose()?;
        let active = lock(&self.session).is_some();
        self.place_surface(app, active)
    }

    pub fn set_holes(&self, app: &AppHandle, holes: Vec<SurfaceRect>) -> Result<(), String> {
        *lock(&self.holes) = holes
            .into_iter()
            .map(SurfaceRect::validate)
            .collect::<Result<Vec<_>, _>>()?;
        let active = lock(&self.session).is_some();
        self.place_surface(app, active)
    }

    pub fn window_report(
        &self,
        app: &AppHandle,
        points: Vec<ClientPoint>,
    ) -> Result<WindowReport, String> {
        let mpv_process = self.status().process_id;
        self.with_surface(app, |surface| surface.report(app, mpv_process, points))
    }

    fn with_surface<T>(
        &self,
        app: &AppHandle,
        action: impl FnOnce(&VideoSurface) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut surface = lock(&self.surface);
        if surface.is_none() {
            *surface = Some(VideoSurface::create(app)?);
        }
        action(surface.as_ref().expect("superficie recién creada"))
    }

    fn place_surface(&self, app: &AppHandle, visible: bool) -> Result<(), String> {
        let placement = if visible {
            *lock(&self.placement)
        } else {
            None
        };
        let holes = lock(&self.holes).clone();
        let surface = lock(&self.surface);
        match surface.as_ref() {
            Some(surface) => surface.place(app, placement, holes),
            None => Ok(()),
        }
    }
}

/// Salto relativo exacto. Sin `exact`, un vídeo asociado salta a su fotograma clave anterior
/// mientras el audio del archivo principal salta al destino, y mpv retiene el audio hasta que el
/// vídeo lo alcanza: segundos de silencio tras cada salto.
fn seek_command(seconds: f64) -> Result<[String; 3], String> {
    if !seconds.is_finite() {
        return Err("Desplazamiento no válido".to_string());
    }
    Ok([
        "seek".to_string(),
        seconds.to_string(),
        "relative+exact".to_string(),
    ])
}

fn track_command(kind: TrackSelection, id: Option<i64>) -> Result<[String; 3], String> {
    let property = match kind {
        TrackSelection::Audio => "aid",
        TrackSelection::Subtitle => "sid",
    };
    let value = match id {
        Some(id) if id >= 1 => id.to_string(),
        Some(_) => return Err("Número de pista no válido".to_string()),
        None => "no".to_string(),
    };
    Ok(["set".to_string(), property.to_string(), value])
}

fn require_file(path: &str) -> Result<(), String> {
    if Path::new(path).is_file() {
        Ok(())
    } else {
        Err(format!("No existe el archivo: {path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeks_are_exact_so_an_associated_video_does_not_hold_back_the_audio() {
        assert_eq!(
            seek_command(-10.0).unwrap(),
            ["seek", "-10", "relative+exact"]
        );
        assert_eq!(seek_command(2.5).unwrap()[1], "2.5");
    }

    #[test]
    fn rejects_non_finite_seeks() {
        assert!(seek_command(f64::NAN).is_err());
        assert!(seek_command(f64::INFINITY).is_err());
    }

    #[test]
    fn selects_audio_and_subtitle_tracks_by_id() {
        assert_eq!(
            track_command(TrackSelection::Audio, Some(2)).unwrap(),
            ["set", "aid", "2"]
        );
        assert_eq!(
            track_command(TrackSelection::Subtitle, Some(1)).unwrap(),
            ["set", "sid", "1"]
        );
    }

    #[test]
    fn no_track_turns_the_subtitles_off() {
        assert_eq!(
            track_command(TrackSelection::Subtitle, None).unwrap(),
            ["set", "sid", "no"]
        );
    }

    #[test]
    fn rejects_track_ids_below_one() {
        assert!(track_command(TrackSelection::Audio, Some(0)).is_err());
        assert!(track_command(TrackSelection::Subtitle, Some(-3)).is_err());
    }

    #[test]
    fn missing_files_are_reported_with_their_path() {
        let error = require_file("C:/no/existe.srt").unwrap_err();
        assert_eq!(error, "No existe el archivo: C:/no/existe.srt");
    }
}
