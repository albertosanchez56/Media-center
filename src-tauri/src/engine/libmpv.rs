//! Opción A de DP-01: libmpv dentro del proceso, cargada en tiempo de ejecución.

use std::ffi::{CStr, CString, c_char, c_int, c_ulong, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;

use libloading::Library;

use super::EngineBackend;
use super::locate;
use super::state::{OBSERVED_PROPERTIES, ObservedFormat, PropertyValue, Shared};

const FORMAT_NONE: c_int = 0;
const FORMAT_FLAG: c_int = 3;
const FORMAT_DOUBLE: c_int = 5;

const EVENT_NONE: c_int = 0;
const EVENT_SHUTDOWN: c_int = 1;
const EVENT_START_FILE: c_int = 6;
const EVENT_END_FILE: c_int = 7;
const EVENT_FILE_LOADED: c_int = 8;
const EVENT_PROPERTY_CHANGE: c_int = 22;

const END_FILE_REASON_ERROR: c_int = 4;

type Handle = *mut c_void;

#[repr(C)]
struct MpvEvent {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct MpvEventProperty {
    name: *const c_char,
    format: c_int,
    data: *mut c_void,
}

#[repr(C)]
struct MpvEventEndFile {
    reason: c_int,
    error: c_int,
}

struct Api {
    _library: Library,
    client_api_version: unsafe extern "C" fn() -> c_ulong,
    create: unsafe extern "C" fn() -> Handle,
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    command: unsafe extern "C" fn(Handle, *mut *const c_char) -> c_int,
    get_property_string: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
    observe_property: unsafe extern "C" fn(Handle, u64, *const c_char, c_int) -> c_int,
    wait_event: unsafe extern "C" fn(Handle, f64) -> *mut MpvEvent,
    wakeup: unsafe extern "C" fn(Handle),
    terminate_destroy: unsafe extern "C" fn(Handle),
    free: unsafe extern "C" fn(*mut c_void),
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
}

static API: OnceLock<Arc<Api>> = OnceLock::new();

fn api() -> Result<Arc<Api>, String> {
    if let Some(api) = API.get() {
        return Ok(api.clone());
    }
    let path = locate::libmpv()?;
    let api = unsafe { load(&path) }
        .map_err(|error| format!("No se pudo cargar {}: {error}", path.display()))?;
    let major = unsafe { (api.client_api_version)() } >> 16;
    if major != 2 {
        return Err(format!(
            "Versión de la API de libmpv no compatible: {major}.x"
        ));
    }
    Ok(API.get_or_init(|| Arc::new(api)).clone())
}

unsafe fn load(path: &std::path::Path) -> Result<Api, libloading::Error> {
    unsafe {
        let library = Library::new(path)?;
        macro_rules! symbol {
            ($name:literal) => {
                *library.get(concat!($name, "\0").as_bytes())?
            };
        }
        Ok(Api {
            client_api_version: symbol!("mpv_client_api_version"),
            create: symbol!("mpv_create"),
            set_option_string: symbol!("mpv_set_option_string"),
            initialize: symbol!("mpv_initialize"),
            command: symbol!("mpv_command"),
            get_property_string: symbol!("mpv_get_property_string"),
            observe_property: symbol!("mpv_observe_property"),
            wait_event: symbol!("mpv_wait_event"),
            wakeup: symbol!("mpv_wakeup"),
            terminate_destroy: symbol!("mpv_terminate_destroy"),
            free: symbol!("mpv_free"),
            error_string: symbol!("mpv_error_string"),
            _library: library,
        })
    }
}

/// El cliente de libmpv es seguro entre hilos, salvo `mpv_wait_event`, que solo usa el hilo de eventos.
#[derive(Clone, Copy)]
struct MpvHandle(Handle);
unsafe impl Send for MpvHandle {}
unsafe impl Sync for MpvHandle {}

#[derive(Clone)]
struct Client {
    api: Arc<Api>,
    handle: MpvHandle,
}

impl Client {
    fn check(&self, code: c_int) -> Result<(), String> {
        if code >= 0 {
            return Ok(());
        }
        let message = unsafe { CStr::from_ptr((self.api.error_string)(code)) };
        Err(message.to_string_lossy().into_owned())
    }

    fn set_option(&self, name: &str, value: &str) -> Result<(), String> {
        let (name_c, value_c) = (cstring(name)?, cstring(value)?);
        let code = unsafe {
            (self.api.set_option_string)(self.handle.0, name_c.as_ptr(), value_c.as_ptr())
        };
        self.check(code)
            .map_err(|error| format!("Opción {name}={value}: {error}"))
    }

    fn command(&self, args: &[&str]) -> Result<(), String> {
        let owned = args
            .iter()
            .map(|arg| cstring(arg))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pointers: Vec<*const c_char> = owned.iter().map(|arg| arg.as_ptr()).collect();
        pointers.push(std::ptr::null());
        let code = unsafe { (self.api.command)(self.handle.0, pointers.as_mut_ptr()) };
        self.check(code)
    }

    fn get_property(&self, name: &str) -> Result<Option<String>, String> {
        let name_c = cstring(name)?;
        unsafe {
            let value = (self.api.get_property_string)(self.handle.0, name_c.as_ptr());
            if value.is_null() {
                return Ok(None);
            }
            let text = CStr::from_ptr(value).to_string_lossy().into_owned();
            (self.api.free)(value.cast());
            Ok(Some(text))
        }
    }
}

fn cstring(text: &str) -> Result<CString, String> {
    CString::new(text).map_err(|_| format!("Texto con un carácter nulo: {text:?}"))
}

pub struct LibmpvBackend {
    client: Client,
    stop: Arc<AtomicBool>,
    events: Option<JoinHandle<()>>,
    destroyed: bool,
}

impl LibmpvBackend {
    pub fn start(wid: i64, d3d11_flip: bool, shared: Arc<Shared>) -> Result<Self, String> {
        let api = api()?;
        let handle = unsafe { (api.create)() };
        if handle.is_null() {
            return Err("mpv_create no pudo crear el reproductor".to_string());
        }
        let client = Client {
            api,
            handle: MpvHandle(handle),
        };
        if let Err(error) = configure(&client, wid, d3d11_flip) {
            unsafe { (client.api.terminate_destroy)(handle) };
            return Err(error);
        }

        shared.update(|state| {
            state.set_build_info(
                client.get_property("mpv-version").ok().flatten(),
                client.get_property("ffmpeg-version").ok().flatten(),
                client
                    .get_property("mpv-configuration")
                    .ok()
                    .flatten()
                    .as_deref(),
            );
            true
        });

        let stop = Arc::new(AtomicBool::new(false));
        let events = {
            let (client, stop) = (client.clone(), stop.clone());
            std::thread::Builder::new()
                .name("libmpv-events".into())
                .spawn(move || event_loop(&client, &stop, &shared))
                .map_err(|error| error.to_string())?
        };
        Ok(Self {
            client,
            stop,
            events: Some(events),
            destroyed: false,
        })
    }
}

fn configure(client: &Client, wid: i64, d3d11_flip: bool) -> Result<(), String> {
    for (name, value) in [
        ("wid", wid.to_string().as_str()),
        ("idle", "yes"),
        ("keep-open", "yes"),
        ("osc", "no"),
        ("input-default-bindings", "no"),
        ("input-vo-keyboard", "no"),
        ("input-cursor", "no"),
        ("hwdec", "auto-safe"),
        ("d3d11-flip", if d3d11_flip { "yes" } else { "no" }),
    ] {
        client.set_option(name, value)?;
    }
    client.check(unsafe { (client.api.initialize)(client.handle.0) })?;
    for (index, (name, format)) in OBSERVED_PROPERTIES.iter().enumerate() {
        let name_c = cstring(name)?;
        let format = match format {
            ObservedFormat::Flag => FORMAT_FLAG,
            ObservedFormat::Double => FORMAT_DOUBLE,
        };
        client.check(unsafe {
            (client.api.observe_property)(
                client.handle.0,
                index as u64 + 1,
                name_c.as_ptr(),
                format,
            )
        })?;
    }
    Ok(())
}

fn event_loop(client: &Client, stop: &AtomicBool, shared: &Shared) {
    while !stop.load(Ordering::Acquire) {
        let event = unsafe { &*(client.api.wait_event)(client.handle.0, 0.5) };
        match event.event_id {
            EVENT_NONE => {}
            EVENT_SHUTDOWN => break,
            EVENT_START_FILE => shared.update(|state| {
                state.start_file_count += 1;
                state.eof_reached = false;
                true
            }),
            EVENT_FILE_LOADED => shared.on_file_loaded(|args| client.command(args)),
            EVENT_END_FILE => {
                let end = unsafe { &*(event.data as *const MpvEventEndFile) };
                if end.reason == END_FILE_REASON_ERROR {
                    let message = client.check(end.error).err().unwrap_or_default();
                    shared.record_error(format!("mpv no pudo reproducir el archivo: {message}"));
                }
            }
            EVENT_PROPERTY_CHANGE => {
                let property = unsafe { &*(event.data as *const MpvEventProperty) };
                let name = unsafe { CStr::from_ptr(property.name) }.to_string_lossy();
                let value = unsafe { read_value(property) };
                shared.update(|state| state.apply_property(&name, value));
            }
            _ => {}
        }
    }
}

unsafe fn read_value(property: &MpvEventProperty) -> PropertyValue {
    unsafe {
        match property.format {
            FORMAT_NONE => PropertyValue::Unavailable,
            FORMAT_FLAG => PropertyValue::Flag(*(property.data as *const c_int) != 0),
            FORMAT_DOUBLE => PropertyValue::Double(*(property.data as *const f64)),
            _ => PropertyValue::Unavailable,
        }
    }
}

impl EngineBackend for LibmpvBackend {
    fn command(&self, args: &[&str]) -> Result<(), String> {
        self.client.command(args)
    }

    fn get_property(&self, name: &str) -> Result<Option<String>, String> {
        self.client.get_property(name)
    }

    fn shutdown(&mut self) {
        if self.destroyed {
            return;
        }
        self.destroyed = true;
        self.stop.store(true, Ordering::Release);
        unsafe { (self.client.api.wakeup)(self.client.handle.0) };
        if let Some(events) = self.events.take() {
            let _ = events.join();
        }
        // mpv_wait_event no puede estar en curso al destruir el cliente; el hilo ya ha terminado.
        unsafe { (self.client.api.terminate_destroy)(self.client.handle.0) };
    }
}

impl Drop for LibmpvBackend {
    fn drop(&mut self) {
        self.shutdown();
    }
}
