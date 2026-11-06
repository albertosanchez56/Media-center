use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub const STATE_EVENT: &str = "engine-state";

/// Las actualizaciones de posición llegan en cada fotograma; la interfaz solo necesita unas pocas por segundo.
const EMIT_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EngineKind {
    Libmpv,
    MpvProcess,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    pub kind: Option<EngineKind>,
    pub path: Option<String>,
    pub associated_video: Option<String>,
    pub time_pos: Option<f64>,
    pub duration: Option<f64>,
    pub pause: bool,
    pub avsync: Option<f64>,
    pub eof_reached: bool,
    pub start_file_count: u32,
    pub file_loaded_count: u32,
    pub process_id: Option<u32>,
    pub mpv_version: Option<String>,
    pub ffmpeg_version: Option<String>,
    pub lgpl_build: Option<bool>,
    pub last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropertyValue {
    Unavailable,
    Flag(bool),
    Double(f64),
}

/// Propiedades que ambos motores observan, con el formato en que se leen.
pub const OBSERVED_PROPERTIES: [(&str, ObservedFormat); 5] = [
    ("time-pos", ObservedFormat::Double),
    ("duration", ObservedFormat::Double),
    ("pause", ObservedFormat::Flag),
    ("avsync", ObservedFormat::Double),
    ("eof-reached", ObservedFormat::Flag),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedFormat {
    Flag,
    Double,
}

impl PropertyValue {
    fn as_f64(self) -> Option<f64> {
        match self {
            Self::Double(value) => Some(value),
            _ => None,
        }
    }

    fn as_bool(self) -> bool {
        matches!(self, Self::Flag(true))
    }
}

impl EngineState {
    /// Aplica un cambio de propiedad y dice si merece avisar a la interfaz de inmediato.
    pub fn apply_property(&mut self, name: &str, value: PropertyValue) -> bool {
        match name {
            "time-pos" => {
                self.time_pos = value.as_f64();
                false
            }
            "avsync" => {
                self.avsync = value.as_f64();
                false
            }
            "duration" => {
                self.duration = value.as_f64();
                true
            }
            "pause" => {
                self.pause = value.as_bool();
                true
            }
            "eof-reached" => {
                self.eof_reached = value.as_bool();
                true
            }
            _ => false,
        }
    }

    pub fn set_build_info(
        &mut self,
        mpv_version: Option<String>,
        ffmpeg_version: Option<String>,
        configuration: Option<&str>,
    ) {
        self.mpv_version = mpv_version;
        self.ffmpeg_version = ffmpeg_version;
        self.lgpl_build = configuration.map(|config| config.contains("gpl=false"));
    }
}

/// Estado de una sesión compartido entre el gestor y el hilo de eventos del motor.
pub struct Shared {
    app: AppHandle,
    state: Mutex<EngineState>,
    pending_associated: Mutex<Option<String>>,
    last_emit: Mutex<Option<Instant>>,
}

impl Shared {
    pub fn new(app: AppHandle, initial: EngineState) -> Arc<Self> {
        let pending = initial.associated_video.clone();
        Arc::new(Self {
            app,
            state: Mutex::new(initial),
            pending_associated: Mutex::new(pending),
            last_emit: Mutex::new(None),
        })
    }

    pub fn snapshot(&self) -> EngineState {
        lock(&self.state).clone()
    }

    /// Modifica el estado y lo emite si el cambio es importante o si ha pasado el intervalo mínimo.
    pub fn update(&self, change: impl FnOnce(&mut EngineState) -> bool) {
        let (important, snapshot) = {
            let mut state = lock(&self.state);
            let important = change(&mut state);
            (important, state.clone())
        };
        let mut last = lock(&self.last_emit);
        let due = last.is_none_or(|at| at.elapsed() >= EMIT_INTERVAL);
        if important || due {
            *last = Some(Instant::now());
            let _ = self.app.emit(STATE_EVENT, snapshot);
        }
    }

    pub fn record_error(&self, message: impl Into<String>) {
        let message = message.into();
        self.update(|state| {
            state.last_error = Some(message);
            true
        });
    }

    /// Cuenta la carga y añade el vídeo asociado pendiente como pista externa de la misma instancia.
    pub fn on_file_loaded(&self, run: impl Fn(&[&str]) -> Result<(), String>) {
        self.update(|state| {
            state.file_loaded_count += 1;
            true
        });
        let associated = lock(&self.pending_associated).take();
        if let Some(video) = associated
            && let Err(error) = run(&["video-add", &video, "select"])
        {
            self.record_error(format!("No se pudo añadir el vídeo asociado: {error}"));
        }
    }
}

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_changes_are_not_urgent() {
        let mut state = EngineState::default();
        assert!(!state.apply_property("time-pos", PropertyValue::Double(12.5)));
        assert_eq!(state.time_pos, Some(12.5));
    }

    #[test]
    fn pause_changes_are_urgent() {
        let mut state = EngineState::default();
        assert!(state.apply_property("pause", PropertyValue::Flag(true)));
        assert!(state.pause);
    }

    #[test]
    fn unavailable_values_clear_the_field() {
        let mut state = EngineState {
            avsync: Some(0.2),
            ..Default::default()
        };
        state.apply_property("avsync", PropertyValue::Unavailable);
        assert_eq!(state.avsync, None);
    }

    #[test]
    fn build_info_detects_lgpl_configuration() {
        let mut state = EngineState::default();
        state.set_build_info(None, None, Some("-Dgpl=false -Dlua=luajit"));
        assert_eq!(state.lgpl_build, Some(true));
        state.set_build_info(None, None, Some("-Dgpl=true"));
        assert_eq!(state.lgpl_build, Some(false));
    }
}
