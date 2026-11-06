//! Opción B de DP-01: un proceso `mpv.exe` controlado por su JSON IPC en una tubería con nombre.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::EngineBackend;
use super::ipc::{self, IpcMessage};
use super::locate;
use super::state::{OBSERVED_PROPERTIES, Shared, lock};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const QUIT_TIMEOUT: Duration = Duration::from_secs(3);

static NEXT_PIPE: AtomicU64 = AtomicU64::new(1);

/// Conexión para órdenes. Cada orden espera su respuesta antes de soltar el cerrojo, así que la
/// tubería nunca se llena con respuestas sin leer y no se lee y escribe a la vez en el mismo handle.
struct CommandChannel {
    writer: File,
    reader: BufReader<File>,
    next_id: u64,
}

impl CommandChannel {
    fn request(&mut self, args: &[&str]) -> Result<serde_json::Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.writer
            .write_all(ipc::command_line(args, id).as_bytes())
            .map_err(|error| format!("No se pudo escribir en la tubería de mpv: {error}"))?;
        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .reader
                .read_line(&mut line)
                .map_err(|error| format!("No se pudo leer la tubería de mpv: {error}"))?;
            if read == 0 {
                return Err("mpv cerró la conexión".to_string());
            }
            if let IpcMessage::Reply {
                request_id,
                error,
                data,
            } = ipc::parse_line(&line)
                && request_id == id
            {
                return if error == "success" {
                    Ok(data)
                } else {
                    Err(error)
                };
            }
        }
    }
}

#[derive(Clone)]
struct Commands(Arc<Mutex<CommandChannel>>);

impl Commands {
    fn run(&self, args: &[&str]) -> Result<(), String> {
        lock(&self.0).request(args).map(|_| ())
    }

    fn get_property(&self, name: &str) -> Result<Option<String>, String> {
        match lock(&self.0).request(&["get_property", name]) {
            Ok(data) => Ok(ipc::value_to_property_string(&data)),
            Err(error) if error == "property unavailable" => Ok(None),
            Err(error) => Err(error),
        }
    }
}

pub struct MpvProcessBackend {
    child: Child,
    commands: Commands,
    events: Option<JoinHandle<()>>,
    stopped: bool,
}

impl MpvProcessBackend {
    pub fn start(wid: i64, d3d11_flip: bool, shared: Arc<Shared>) -> Result<Self, String> {
        let executable = locate::mpv_executable()?;
        let pipe = format!(
            r"\\.\pipe\media-center-mpv-{}-{}",
            std::process::id(),
            NEXT_PIPE.fetch_add(1, Ordering::Relaxed)
        );
        let mut child = Command::new(&executable)
            .args([
                format!("--wid={wid}"),
                format!("--input-ipc-server={pipe}"),
                format!("--d3d11-flip={}", if d3d11_flip { "yes" } else { "no" }),
            ])
            .args([
                "--no-config",
                "--idle=yes",
                "--keep-open=yes",
                "--osc=no",
                "--input-default-bindings=no",
                "--input-vo-keyboard=no",
                "--input-cursor=no",
                "--hwdec=auto-safe",
                "--no-terminal",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| format!("No se pudo iniciar {}: {error}", executable.display()))?;

        match connect(&pipe, &shared, child.id()) {
            Ok((commands, events)) => Ok(Self {
                child,
                commands,
                events: Some(events),
                stopped: false,
            }),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(error)
            }
        }
    }
}

fn open_pipe(pipe: &str) -> Result<File, String> {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    loop {
        match OpenOptions::new().read(true).write(true).open(pipe) {
            Ok(file) => return Ok(file),
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(format!("No se pudo conectar con mpv por {pipe}: {error}")),
        }
    }
}

fn connect(
    pipe: &str,
    shared: &Arc<Shared>,
    process_id: u32,
) -> Result<(Commands, JoinHandle<()>), String> {
    let command_pipe = open_pipe(pipe)?;
    let reader = BufReader::new(
        command_pipe
            .try_clone()
            .map_err(|error| error.to_string())?,
    );
    let commands = Commands(Arc::new(Mutex::new(CommandChannel {
        writer: command_pipe,
        reader,
        next_id: 0,
    })));
    commands.run(&["disable_event", "all"])?;

    let mut event_pipe = open_pipe(pipe)?;
    for (index, (name, _)) in OBSERVED_PROPERTIES.iter().enumerate() {
        event_pipe
            .write_all(ipc::observe_line(index as u64 + 1, name).as_bytes())
            .map_err(|error| error.to_string())?;
    }

    shared.update(|state| {
        state.process_id = Some(process_id);
        state.set_build_info(
            commands.get_property("mpv-version").ok().flatten(),
            commands.get_property("ffmpeg-version").ok().flatten(),
            commands
                .get_property("mpv-configuration")
                .ok()
                .flatten()
                .as_deref(),
        );
        true
    });

    let events = {
        let (commands, shared) = (commands.clone(), shared.clone());
        std::thread::Builder::new()
            .name("mpv-ipc-events".into())
            .spawn(move || event_loop(event_pipe, &commands, &shared))
            .map_err(|error| error.to_string())?
    };
    Ok((commands, events))
}

fn event_loop(pipe: File, commands: &Commands, shared: &Shared) {
    for line in BufReader::new(pipe).lines() {
        let Ok(line) = line else { break };
        match ipc::parse_line(&line) {
            IpcMessage::PropertyChange { name, value } => {
                shared.update(|state| state.apply_property(&name, value));
            }
            IpcMessage::StartFile => shared.update(|state| {
                state.start_file_count += 1;
                state.eof_reached = false;
                true
            }),
            IpcMessage::FileLoaded => shared.on_file_loaded(|args| commands.run(args)),
            IpcMessage::EndFile { reason, error } if reason == "error" => {
                shared.record_error(format!(
                    "mpv no pudo reproducir el archivo: {}",
                    error.unwrap_or_default()
                ));
            }
            IpcMessage::Shutdown => break,
            _ => {}
        }
    }
}

impl EngineBackend for MpvProcessBackend {
    fn command(&self, args: &[&str]) -> Result<(), String> {
        self.commands.run(args)
    }

    fn get_property(&self, name: &str) -> Result<Option<String>, String> {
        self.commands.get_property(name)
    }

    fn shutdown(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        let _ = self.commands.run(&["quit"]);
        let deadline = Instant::now() + QUIT_TIMEOUT;
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(events) = self.events.take() {
            let _ = events.join();
        }
    }
}

impl Drop for MpvProcessBackend {
    fn drop(&mut self) {
        self.shutdown();
    }
}
