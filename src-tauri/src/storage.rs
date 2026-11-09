//! Rutas de la aplicación. No crea la base de datos ni la carpeta multimedia.
//! La base la abre el módulo `db`, en el archivo `library.db` de estas rutas.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const PRODUCTION_IDENTIFIER: &str = "com.mellk.mediacenter";
pub const DEVELOPMENT_IDENTIFIER: &str = "com.mellk.mediacenter.dev";
const PRODUCTION_FOLDER_NAME: &str = "Media Center";
const DEVELOPMENT_FOLDER_NAME: &str = "Media Center Dev";
const STORAGE_VERSION: u32 = 1;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Invalid(String),
    Missing,
    Corrupt(String),
    UnexpectedIdentifier(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "Error de archivo: {error}"),
            Self::Invalid(message) => write!(formatter, "{message}"),
            Self::Missing => write!(formatter, "No hay configuración de almacenamiento."),
            Self::Corrupt(message) => {
                write!(
                    formatter,
                    "La configuración de almacenamiento no es válida: {message}"
                )
            }
            Self::UnexpectedIdentifier(identifier) => write!(
                formatter,
                "Identificador de aplicación no previsto: {identifier}"
            ),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confirmation {
    Proposed,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedFolderConfig {
    pub path: PathBuf,
    pub confirmation: Confirmation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedFolderState {
    Proposed(PathBuf),
    Confirmed(PathBuf),
    Unavailable(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalPaths {
    pub root: PathBuf,
    pub images: PathBuf,
    pub database: PathBuf,
    pub storage_file: PathBuf,
}

impl InternalPaths {
    pub fn from_app_local_data(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            images: root.join("images"),
            database: root.join("library.db"),
            storage_file: root.join("storage.json"),
            root,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StartupStorage {
    Ready(ManagedFolderConfig),
    MissingVideosFolder,
    Unreadable,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StorageFile {
    version: u32,
    managed_folder: ManagedFolderFile,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ManagedFolderFile {
    path: String,
    confirmation: Confirmation,
}

pub fn media_folder_name(identifier: &str) -> Result<&'static str, StorageError> {
    match identifier {
        PRODUCTION_IDENTIFIER => Ok(PRODUCTION_FOLDER_NAME),
        DEVELOPMENT_IDENTIFIER => Ok(DEVELOPMENT_FOLDER_NAME),
        other => Err(StorageError::UnexpectedIdentifier(other.to_string())),
    }
}

pub fn app_local_data(local_root: &Path, identifier: &str) -> Result<PathBuf, StorageError> {
    media_folder_name(identifier)?;
    Ok(local_root.join(identifier))
}

pub fn proposed_media_folder(videos_dir: &Path, identifier: &str) -> Result<PathBuf, StorageError> {
    Ok(videos_dir.join(media_folder_name(identifier)?))
}

pub fn incoming_dir(managed: &Path) -> PathBuf {
    managed.join(".media-center").join("incoming")
}

pub fn disposable_temp_dir(system_temp: &Path, identifier: &str) -> Result<PathBuf, StorageError> {
    media_folder_name(identifier)?;
    Ok(system_temp.join(identifier))
}

pub fn ensure_internal_dirs(paths: &InternalPaths) -> Result<(), StorageError> {
    fs::create_dir_all(&paths.root)?;
    fs::create_dir_all(&paths.images)?;
    Ok(())
}

pub fn forbidden_roots(paths: &InternalPaths, executable: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = vec![paths.root.clone(), paths.images.clone()];
    if let Some(executable) = executable {
        if let Some(directory) = executable.parent() {
            roots.push(directory.to_path_buf());
        }
        if let Some(workspace) = workspace_root(executable) {
            roots.push(workspace);
        }
    }
    roots
}

pub fn validate_candidate(candidate: &Path, forbidden: &[PathBuf]) -> Result<(), StorageError> {
    if !candidate.is_absolute() {
        return Err(StorageError::Invalid(
            "La carpeta multimedia tiene que ser una ruta absoluta.".into(),
        ));
    }
    if candidate.exists() && !candidate.is_dir() {
        return Err(StorageError::Invalid(
            "La ruta elegida es un archivo, no una carpeta.".into(),
        ));
    }
    if is_reparse_point(candidate) {
        return Err(StorageError::Invalid(
            "La carpeta elegida es un enlace y no se acepta como biblioteca.".into(),
        ));
    }
    let candidate_key = normalized_path(candidate);
    for root in forbidden {
        let root_key = normalized_path(root);
        if candidate_key.starts_with(&root_key) || root_key.starts_with(&candidate_key) {
            return Err(StorageError::Invalid(
                "La carpeta multimedia no puede estar dentro de los datos de la aplicación, ni contenerlos.".into(),
            ));
        }
    }
    Ok(())
}

pub fn load_storage(path: &Path) -> Result<ManagedFolderConfig, StorageError> {
    if !path.exists() {
        return Err(StorageError::Missing);
    }
    let text = fs::read_to_string(path)?;
    let file: StorageFile =
        serde_json::from_str(&text).map_err(|error| StorageError::Corrupt(error.to_string()))?;
    if file.version != STORAGE_VERSION {
        return Err(StorageError::Corrupt(format!("versión {}", file.version)));
    }
    let folder = PathBuf::from(&file.managed_folder.path);
    if !folder.is_absolute() {
        return Err(StorageError::Corrupt(
            "la ruta guardada no es absoluta".into(),
        ));
    }
    Ok(ManagedFolderConfig {
        path: folder,
        confirmation: file.managed_folder.confirmation,
    })
}

pub fn save_storage(path: &Path, config: &ManagedFolderConfig) -> Result<(), StorageError> {
    let path_text = config.path.to_str().ok_or_else(|| {
        StorageError::Invalid("La ruta no se puede guardar como texto Unicode.".into())
    })?;
    let file = StorageFile {
        version: STORAGE_VERSION,
        managed_folder: ManagedFolderFile {
            path: path_text.to_string(),
            confirmation: config.confirmation,
        },
    };
    let bytes = serde_json::to_vec_pretty(&file)
        .map_err(|error| StorageError::Corrupt(error.to_string()))?;
    atomic_write(path, &bytes)
}

pub fn resolve_state(config: &ManagedFolderConfig) -> ManagedFolderState {
    match config.confirmation {
        Confirmation::Proposed => ManagedFolderState::Proposed(config.path.clone()),
        Confirmation::Confirmed if directory_available(&config.path) => {
            ManagedFolderState::Confirmed(config.path.clone())
        }
        Confirmation::Confirmed => ManagedFolderState::Unavailable(config.path.clone()),
    }
}

/// Crea la carpeta solo como consecuencia de una confirmación explícita.
/// El arranque no la llama. La importación sí, cuando la persona pulsa Copiar o Mover.
pub fn confirm_managed_folder(
    storage_file: &Path,
    candidate: &Path,
    forbidden: &[PathBuf],
) -> Result<ManagedFolderConfig, StorageError> {
    validate_candidate(candidate, forbidden)?;
    if candidate.exists() {
        if !directory_available(candidate) {
            return Err(StorageError::Invalid(
                "La carpeta confirmada no está disponible o no se puede escribir en ella.".into(),
            ));
        }
    } else {
        let parent = candidate.parent().ok_or_else(|| {
            StorageError::Invalid("La carpeta multimedia no tiene una carpeta superior.".into())
        })?;
        if !parent.is_dir() {
            return Err(StorageError::Invalid(
                "La carpeta superior de la biblioteca no existe.".into(),
            ));
        }
        fs::create_dir(candidate)?;
        if !directory_available(candidate) {
            return Err(StorageError::Invalid(
                "La carpeta se ha creado, pero no se puede escribir en ella.".into(),
            ));
        }
    }
    let config = ManagedFolderConfig {
        path: candidate.to_path_buf(),
        confirmation: Confirmation::Confirmed,
    };
    save_storage(storage_file, &config)?;
    Ok(config)
}

pub fn prepare_storage(
    paths: &InternalPaths,
    videos_dir: Option<&Path>,
    identifier: &str,
    forbidden: &[PathBuf],
) -> Result<StartupStorage, StorageError> {
    ensure_internal_dirs(paths)?;
    match load_storage(&paths.storage_file) {
        Ok(config) => Ok(StartupStorage::Ready(config)),
        Err(StorageError::Missing) => propose_default(paths, videos_dir, identifier, forbidden),
        Err(StorageError::Corrupt(_) | StorageError::Io(_)) => Ok(StartupStorage::Unreadable),
        Err(error) => Err(error),
    }
}

pub fn prepare_on_startup(app: &tauri::AppHandle) -> Result<(), StorageError> {
    use tauri::Manager;
    let local = app.path().app_local_data_dir().map_err(|error| {
        StorageError::Invalid(format!(
            "No se ha podido resolver el directorio local de la aplicación: {error}"
        ))
    })?;
    let paths = InternalPaths::from_app_local_data(local);
    let videos = app.path().video_dir().ok();
    let executable = std::env::current_exe().ok();
    let forbidden = forbidden_roots(&paths, executable.as_deref());
    match prepare_storage(
        &paths,
        videos.as_deref(),
        app.config().identifier.as_str(),
        &forbidden,
    )? {
        StartupStorage::Ready(config) => {
            ensure_reserved_dirs_stay_apart(&paths, &config, app.config().identifier.as_str())?;
            if let ManagedFolderState::Unavailable(path) = resolve_state(&config) {
                eprintln!(
                    "La carpeta multimedia confirmada no está disponible y se conserva: {}",
                    path.display()
                );
            }
            Ok(())
        }
        StartupStorage::MissingVideosFolder => {
            eprintln!(
                "No se ha podido resolver la carpeta de vídeos. La carpeta multimedia queda sin proponer."
            );
            Ok(())
        }
        StartupStorage::Unreadable => {
            eprintln!("storage.json no se ha podido leer y se conserva sin cambios.");
            Ok(())
        }
    }
}

fn ensure_reserved_dirs_stay_apart(
    paths: &InternalPaths,
    config: &ManagedFolderConfig,
    identifier: &str,
) -> Result<(), StorageError> {
    let incoming = incoming_dir(&config.path);
    if !incoming.starts_with(&config.path) {
        return Err(StorageError::Invalid(
            "El temporal de copia queda fuera de la carpeta multimedia.".into(),
        ));
    }
    let disposable = disposable_temp_dir(&std::env::temp_dir(), identifier)?;
    if normalized_path(&disposable).starts_with(&normalized_path(&config.path)) {
        return Err(StorageError::Invalid(
            "El temporal del sistema no puede quedar dentro de la carpeta multimedia.".into(),
        ));
    }
    if let Some(parent) = paths.root.parent() {
        let separated = app_local_data(parent, identifier)?;
        if normalized_path(&separated) != normalized_path(&paths.root) {
            return Err(StorageError::Invalid(
                "El directorio local no corresponde al identificador de la aplicación.".into(),
            ));
        }
    }
    Ok(())
}

fn propose_default(
    paths: &InternalPaths,
    videos_dir: Option<&Path>,
    identifier: &str,
    forbidden: &[PathBuf],
) -> Result<StartupStorage, StorageError> {
    let Some(videos_dir) = videos_dir else {
        return Ok(StartupStorage::MissingVideosFolder);
    };
    let proposed = proposed_media_folder(videos_dir, identifier)?;
    validate_candidate(&proposed, forbidden)?;
    let config = ManagedFolderConfig {
        path: proposed,
        confirmation: Confirmation::Proposed,
    };
    save_storage(&paths.storage_file, &config)?;
    Ok(StartupStorage::Ready(config))
}

fn directory_available(path: &Path) -> bool {
    path.is_dir() && !is_reparse_point(path) && can_write_directory(path)
}

fn can_write_directory(path: &Path) -> bool {
    let probe = path.join(format!(
        ".media-center-probe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    match OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(file) => {
            drop(file);
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn atomic_write(destination: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let parent = destination
        .parent()
        .ok_or_else(|| StorageError::Invalid("La configuración no tiene directorio.".into()))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join("storage.json.tmp");
    let backup = parent.join("storage.json.bak");
    {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    if let Err(error) = replace_file(&temporary, destination, &backup) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn replace_file(temporary: &Path, destination: &Path, backup: &Path) -> Result<(), StorageError> {
    if !destination.exists() {
        fs::rename(temporary, destination)?;
        return Ok(());
    }
    let _ = fs::remove_file(backup);
    fs::rename(destination, backup)?;
    if let Err(error) = fs::rename(temporary, destination) {
        let restored = fs::rename(backup, destination);
        if restored.is_err() {
            return Err(StorageError::Invalid(
                "No se ha podido guardar la configuración ni restaurar la anterior.".into(),
            ));
        }
        return Err(error.into());
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

fn workspace_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|directory| {
            directory.join("package.json").is_file()
                && directory
                    .join("src-tauri")
                    .join("tauri.conf.json")
                    .is_file()
        })
        .map(Path::to_path_buf)
}

fn normalized_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        windows_normalized_path(path)
    }
    #[cfg(not(windows))]
    {
        path.components().collect()
    }
}

#[cfg(windows)]
fn windows_normalized_path(path: &Path) -> PathBuf {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows::Win32::Storage::FileSystem::GetFullPathNameW;
    use windows::Win32::UI::WindowsAndMessaging::CharLowerBuffW;
    use windows::core::PCWSTR;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut buffer = vec![0u16; 32 * 1024];
    let length = unsafe { GetFullPathNameW(PCWSTR(wide.as_ptr()), Some(&mut buffer[..]), None) };
    if length == 0 || length as usize >= buffer.len() {
        return path.to_path_buf();
    }
    let mut chars = buffer[..length as usize].to_vec();
    unsafe {
        CharLowerBuffW(&mut chars[..]);
    }
    let text = OsString::from_wide(&chars);
    let text = text.to_string_lossy();
    let without_prefix = text
        .strip_prefix(r"\\?\")
        .unwrap_or(text.as_ref())
        .replace('/', r"\");
    PathBuf::from(without_prefix)
}

#[cfg(windows)]
pub(crate) fn is_reparse_point(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, GetFileAttributesW, INVALID_FILE_ATTRIBUTES,
    };
    use windows::core::PCWSTR;

    if !path.exists() {
        return false;
    }
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let attributes = unsafe { GetFileAttributesW(PCWSTR(wide.as_ptr())) };
    attributes != INVALID_FILE_ATTRIBUTES && (attributes & FILE_ATTRIBUTE_REPARSE_POINT.0) != 0
}

#[cfg(not(windows))]
pub(crate) fn is_reparse_point(_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Scratch {
        path: PathBuf,
    }

    impl Scratch {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("mc-f21-{}-{nanos}-{unique}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn config(path: &Path, confirmation: Confirmation) -> ManagedFolderConfig {
        ManagedFolderConfig {
            path: path.to_path_buf(),
            confirmation,
        }
    }

    #[test]
    fn development_and_production_paths_do_not_overlap() {
        let local = PathBuf::from(r"C:\Users\example\AppData\Local");
        let videos = PathBuf::from(r"C:\Users\example\Videos");
        let temp = PathBuf::from(r"C:\Users\example\AppData\Local\Temp");
        let production_data = app_local_data(&local, PRODUCTION_IDENTIFIER).unwrap();
        let development_data = app_local_data(&local, DEVELOPMENT_IDENTIFIER).unwrap();
        assert!(!production_data.starts_with(&development_data));
        assert!(!development_data.starts_with(&production_data));
        let production_media = proposed_media_folder(&videos, PRODUCTION_IDENTIFIER).unwrap();
        let development_media = proposed_media_folder(&videos, DEVELOPMENT_IDENTIFIER).unwrap();
        assert!(production_media.ends_with(PRODUCTION_FOLDER_NAME));
        assert!(development_media.ends_with(DEVELOPMENT_FOLDER_NAME));
        assert_ne!(production_media, development_media);
        assert_ne!(
            disposable_temp_dir(&temp, PRODUCTION_IDENTIFIER).unwrap(),
            disposable_temp_dir(&temp, DEVELOPMENT_IDENTIFIER).unwrap()
        );
        assert!(media_folder_name("com.otro.identificador").is_err());
    }

    #[test]
    fn startup_proposes_the_media_folder_without_creating_it() {
        let scratch = Scratch::new();
        let videos = scratch.path.join("vídeos con espacio");
        fs::create_dir_all(&videos).unwrap();
        let internal = InternalPaths::from_app_local_data(scratch.path.join("app"));
        let outcome =
            prepare_storage(&internal, Some(&videos), PRODUCTION_IDENTIFIER, &[]).unwrap();
        let StartupStorage::Ready(saved) = outcome else {
            panic!("se esperaba una ruta propuesta");
        };
        assert_eq!(saved.confirmation, Confirmation::Proposed);
        assert!(saved.path.ends_with(PRODUCTION_FOLDER_NAME));
        assert!(saved.path.to_string_lossy().contains("vídeos con espacio"));
        assert!(!saved.path.exists());
        assert!(internal.images.is_dir());
        assert!(!internal.database.exists());
        let loaded = load_storage(&internal.storage_file).unwrap();
        assert_eq!(loaded, saved);
        assert!(matches!(
            resolve_state(&loaded),
            ManagedFolderState::Proposed(_)
        ));
    }

    #[test]
    fn a_failed_write_keeps_the_previous_configuration() {
        let scratch = Scratch::new();
        let storage = scratch.path.join("storage.json");
        let first = config(&scratch.path.join("Biblioteca"), Confirmation::Proposed);
        save_storage(&storage, &first).unwrap();
        let before = fs::read_to_string(&storage).unwrap();
        fs::create_dir(scratch.path.join("storage.json.tmp")).unwrap();
        let second = config(&scratch.path.join("Otra"), Confirmation::Confirmed);
        assert!(save_storage(&storage, &second).is_err());
        assert_eq!(fs::read_to_string(&storage).unwrap(), before);
        assert_eq!(load_storage(&storage).unwrap(), first);
    }

    #[test]
    fn an_unreadable_file_is_not_replaced() {
        let scratch = Scratch::new();
        let internal = InternalPaths::from_app_local_data(scratch.path.join("app"));
        fs::create_dir_all(&internal.root).unwrap();
        fs::write(&internal.storage_file, "{ esto no es json").unwrap();
        let before = fs::read_to_string(&internal.storage_file).unwrap();
        let videos = scratch.path.join("videos");
        fs::create_dir_all(&videos).unwrap();
        let outcome =
            prepare_storage(&internal, Some(&videos), PRODUCTION_IDENTIFIER, &[]).unwrap();
        assert_eq!(outcome, StartupStorage::Unreadable);
        assert_eq!(fs::read_to_string(&internal.storage_file).unwrap(), before);
        assert!(!videos.join(PRODUCTION_FOLDER_NAME).exists());
    }

    #[test]
    fn confirming_creates_the_folder_and_a_missing_volume_stays_confirmed() {
        let scratch = Scratch::new();
        let internal = InternalPaths::from_app_local_data(scratch.path.join("app"));
        ensure_internal_dirs(&internal).unwrap();
        let library = scratch.path.join("disco").join(PRODUCTION_FOLDER_NAME);
        fs::create_dir_all(library.parent().unwrap()).unwrap();
        let saved = confirm_managed_folder(&internal.storage_file, &library, &[]).unwrap();
        assert_eq!(saved.confirmation, Confirmation::Confirmed);
        assert!(library.is_dir());
        assert!(matches!(
            resolve_state(&saved),
            ManagedFolderState::Confirmed(_)
        ));
        fs::remove_dir_all(&library).unwrap();
        let loaded = load_storage(&internal.storage_file).unwrap();
        assert_eq!(loaded.confirmation, Confirmation::Confirmed);
        assert!(matches!(
            resolve_state(&loaded),
            ManagedFolderState::Unavailable(_)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn real_local_app_data_keeps_development_and_production_apart() {
        let Some(local) = std::env::var_os("LOCALAPPDATA") else {
            return;
        };
        let local = PathBuf::from(local);
        let production = app_local_data(&local, PRODUCTION_IDENTIFIER).unwrap();
        let development = app_local_data(&local, DEVELOPMENT_IDENTIFIER).unwrap();
        assert_ne!(normalized_path(&production), normalized_path(&development));
        assert!(!normalized_path(&production).starts_with(&normalized_path(&development)));
        assert!(!normalized_path(&development).starts_with(&normalized_path(&production)));
    }

    #[test]
    fn reserved_directories_of_one_profile_stay_apart() {
        let scratch = Scratch::new();
        let root = scratch.path.join(PRODUCTION_IDENTIFIER);
        let paths = InternalPaths::from_app_local_data(&root);
        let media = scratch.path.join("vídeos").join(PRODUCTION_FOLDER_NAME);
        let saved = config(&media, Confirmation::Proposed);
        ensure_reserved_dirs_stay_apart(&paths, &saved, PRODUCTION_IDENTIFIER).unwrap();
        assert!(!media.exists());
    }

    #[test]
    fn a_media_folder_cannot_live_inside_application_data() {
        let scratch = Scratch::new();
        let internal = InternalPaths::from_app_local_data(scratch.path.join("app"));
        fs::create_dir_all(&internal.images).unwrap();
        let forbidden = forbidden_roots(&internal, None);
        let inside = internal.images.join(PRODUCTION_FOLDER_NAME);
        assert!(validate_candidate(&inside, &forbidden).is_err());
        let relative = PathBuf::from(PRODUCTION_FOLDER_NAME);
        assert!(validate_candidate(&relative, &forbidden).is_err());
        let outside = scratch.path.join("biblioteca");
        assert!(validate_candidate(&outside, &forbidden).is_ok());
        assert_eq!(
            incoming_dir(&outside),
            outside.join(".media-center").join("incoming")
        );
    }

    #[test]
    fn the_workspace_that_contains_the_executable_is_forbidden() {
        let scratch = Scratch::new();
        let workspace = scratch.path.join("repo");
        fs::create_dir_all(workspace.join("src-tauri")).unwrap();
        fs::write(workspace.join("package.json"), "{}").unwrap();
        fs::write(workspace.join("src-tauri").join("tauri.conf.json"), "{}").unwrap();
        let executable = workspace
            .join("src-tauri")
            .join("target")
            .join("debug")
            .join("app.exe");
        let internal = InternalPaths::from_app_local_data(scratch.path.join("app"));
        let forbidden = forbidden_roots(&internal, Some(&executable));
        assert!(validate_candidate(&workspace.join("medios"), &forbidden).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn a_directory_link_is_rejected_when_the_system_allows_creating_one() {
        let scratch = Scratch::new();
        let target = scratch.path.join("real");
        let link = scratch.path.join("enlace");
        fs::create_dir(&target).unwrap();
        if std::os::windows::fs::symlink_dir(&target, &link).is_err() {
            return;
        }
        assert!(validate_candidate(&link, &[]).is_err());
    }
}
