//! Importación a la carpeta administrada.
//!
//! Hay dos procedimientos distintos. El renombrado directo no se usa con los
//! archivos de la persona usuaria: `file_operations` no puede guardar el
//! identificador del archivo ni el del volumen, y la migración 1 no se modifica.
//! Sin esos datos, un renombrado no se puede recuperar. El movimiento real copia,
//! publica sin sobrescribir, confirma el alta y solo entonces borra el original
//! con el manejador que se ha comprobado. Una fila `method = rename` queda en
//! `needs_review` y no borra nada.

mod journal;
mod win;

pub mod commands;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};

use serde::Serialize;

use crate::db::Database;
use crate::storage::{self, Confirmation, ManagedFolderState};

pub use commands::ImportRuntime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Copy,
    Move,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemReport {
    pub source: String,
    pub state: String,
    pub message: String,
}

pub struct Pause {
    pub started: Barrier,
    pub release: Barrier,
}

pub struct ImportHooks {
    pub chunk_bytes: Option<usize>,
    pub interrupt_after_bytes: Option<u64>,
    pub before_publish: Option<Box<dyn FnMut(&Path)>>,
    pub fail_commit: bool,
    pub pause: Option<Arc<Pause>>,
    pub free_bytes: Option<u64>,
}

impl Default for ImportHooks {
    fn default() -> Self {
        Self {
            chunk_bytes: None,
            interrupt_after_bytes: None,
            before_publish: None,
            fail_commit: false,
            pause: None,
            free_bytes: None,
        }
    }
}

pub fn import_paths(
    db: &Database,
    managed: &Path,
    app_data: &Path,
    library: &str,
    mode: Mode,
    paths: &[PathBuf],
    folder: bool,
    cancel: &AtomicBool,
    hooks: &mut ImportHooks,
    report: &mut dyn FnMut(&[ItemReport]),
) -> Vec<ItemReport> {
    if !matches!(library, "music" | "video" | "movie") {
        return vec![report_of(
            "",
            "failed",
            "La biblioteca de destino no es válida.",
        )];
    }
    if paths.is_empty() {
        return vec![report_of("", "failed", "No hay archivos para importar.")];
    }
    if let Err(error) = ensure_incoming(managed) {
        return vec![report_of("", "failed", &error)];
    }

    let mut items = if folder {
        let mut collected = Vec::new();
        for path in paths {
            collected.extend(expand_folder(path, managed, app_data));
        }
        collected
    } else {
        paths
            .iter()
            .map(|path| Candidate::File(path.clone()))
            .collect()
    };
    if items.is_empty() {
        items.push(Candidate::Note(
            paths
                .first()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            "La carpeta no contiene archivos importables.".into(),
        ));
    }

    let mut reports = Vec::new();
    let mut incoming = items.into_iter();
    while let Some(item) = incoming.next() {
        if cancel.load(Ordering::SeqCst) {
            reports.push(cancelled_report(item));
            for rest in incoming {
                reports.push(cancelled_report(rest));
            }
            break;
        }
        let next = match item {
            Candidate::Note(source, message) => report_of(&source, "skipped", &message),
            Candidate::File(path) => {
                import_file(db, managed, app_data, library, mode, &path, cancel, hooks)
            }
        };
        reports.push(next);
        report(&reports);
        if cancel.load(Ordering::SeqCst) {
            for rest in incoming {
                reports.push(cancelled_report(rest));
            }
            break;
        }
    }
    reports
}

pub fn recover(db: &Database, managed: &Path) -> Result<(), String> {
    for operation in journal::pending(db)? {
        if operation.method == "rename" {
            journal::set_stage(
                db,
                operation.id,
                "needs_review",
                Some(journal::RENAMED_WITHOUT_IDENTITY),
            )?;
            continue;
        }
        if operation.kind == "download" || operation.method != "copy" {
            journal::set_stage(
                db,
                operation.id,
                "needs_review",
                Some("La operación no pertenece a la importación y no se modifica ningún archivo."),
            )?;
            continue;
        }
        recover_copy(db, managed, &operation)?;
    }
    Ok(())
}

pub fn recover_on_startup(db: &Database, app_local_data: &Path) -> Result<(), String> {
    let storage_file = storage::InternalPaths::from_app_local_data(app_local_data).storage_file;
    let config = match storage::load_storage(&storage_file) {
        Ok(config) => config,
        Err(storage::StorageError::Missing) => return Ok(()),
        Err(error) => {
            return Err(format!(
                "No se lee la carpeta multimedia y las operaciones pendientes se conservan: {error}"
            ));
        }
    };
    if config.confirmation != Confirmation::Confirmed {
        return Ok(());
    }
    match storage::resolve_state(&config) {
        ManagedFolderState::Confirmed(path) => recover(db, &path),
        _ => Ok(()),
    }
}

pub fn ensure_managed_folder(
    storage_file: &Path,
    forbidden: &[PathBuf],
) -> Result<PathBuf, String> {
    let config = storage::load_storage(storage_file).map_err(|error| match error {
        storage::StorageError::Missing => "No hay una carpeta multimedia propuesta.".to_string(),
        other => other.to_string(),
    })?;
    match storage::resolve_state(&config) {
        ManagedFolderState::Confirmed(path) => Ok(path),
        ManagedFolderState::Proposed(path) => {
            storage::confirm_managed_folder(storage_file, &path, forbidden)
                .map(|config| config.path)
                .map_err(|error| error.to_string())
        }
        ManagedFolderState::Unavailable(path) => Err(format!(
            "La carpeta multimedia no está disponible: {}",
            path.display()
        )),
    }
}

enum Candidate {
    File(PathBuf),
    Note(String, String),
}

fn cancelled_report(item: Candidate) -> ItemReport {
    let source = match item {
        Candidate::File(path) => path.display().to_string(),
        Candidate::Note(source, _) => source,
    };
    report_of(&source, "failed", "Importación cancelada.")
}

fn import_file(
    db: &Database,
    managed: &Path,
    app_data: &Path,
    library: &str,
    mode: Mode,
    source: &Path,
    cancel: &AtomicBool,
    hooks: &mut ImportHooks,
) -> ItemReport {
    let source_text = source.display().to_string();
    if let Err(error) = validate_source(source, managed, app_data) {
        return report_of(&source_text, "failed", &error);
    }
    let key = match win::path_key(source) {
        Ok(key) => key,
        Err(error) => return report_of(&source_text, "failed", &error),
    };
    match journal::path_key_available(db, &key) {
        Ok(true) => {
            return report_of(
                &source_text,
                "skipped",
                "Este archivo ya está en la biblioteca.",
            );
        }
        Ok(false) => {}
        Err(error) => return report_of(&source_text, "failed", &error),
    }
    match journal::source_already_imported(db, &source_text) {
        Ok(true) => {
            return report_of(
                &source_text,
                "skipped",
                "Este archivo ya está en la biblioteca.",
            );
        }
        Ok(false) => {}
        Err(error) => return report_of(&source_text, "failed", &error),
    }

    let original_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "archivo".into());
    let destination_name = match allocate_name(db, managed, &original_name, None) {
        Ok(name) => name,
        Err(error) => return report_of(&source_text, "failed", &error),
    };
    let opened = match win::open_source(source) {
        Ok(opened) => opened,
        Err(error) => return report_of(&source_text, "failed", &error),
    };
    let free = hooks
        .free_bytes
        .map(Ok)
        .unwrap_or_else(|| win::free_bytes(managed));
    let free = match free {
        Ok(free) => free,
        Err(error) => return report_of(&source_text, "failed", &error),
    };
    if free < opened.facts.size {
        return report_of(
            &source_text,
            "failed",
            "No hay espacio suficiente. No se ha empezado la copia.",
        );
    }

    let temp_relative = temp_relative();
    let temp_path = match resolve_inside(managed, &temp_relative) {
        Ok(path) => path,
        Err(error) => return report_of(&source_text, "failed", &error),
    };
    let kind = match mode {
        Mode::Copy => "copy",
        Mode::Move => "move",
    };
    // El renombrado directo no puede guardar la identidad en el diario actual.
    let method = "copy";
    let id = match journal::insert_planned(
        db,
        kind,
        method,
        library,
        &source_text,
        opened.facts.size as i64,
        &opened.facts.modified,
        Some(&temp_relative),
        &destination_name,
    ) {
        Ok(id) => id,
        Err(error) => return report_of(&source_text, "failed", &error),
    };
    if let Err(error) = journal::set_stage(db, id, "transferring", None) {
        return report_of(&source_text, "failed", &error);
    }

    let chunk = hooks.chunk_bytes.unwrap_or(win::COPY_CHUNK);
    let interrupt = hooks.interrupt_after_bytes;
    let copy_result =
        win::copy_handle_to_new_file(&opened, &temp_path, chunk, interrupt, cancel, || {
            if let Some(pause) = hooks.pause.take() {
                pause.started.wait();
                pause.release.wait();
            }
        });
    if let Err(error) = copy_result {
        let _ = discard_temp(managed, &temp_relative);
        let message = if error == "cancelada" {
            "Importación cancelada.".to_string()
        } else if error == "interrumpida" || error.contains("cambiado") {
            "La copia no ha terminado. El original se conserva.".to_string()
        } else {
            error
        };
        let _ = journal::set_stage(db, id, "failed", Some(&message));
        return report_of(&source_text, "failed", &message);
    }

    if let Err(error) = journal::set_stage(db, id, "verifying", None) {
        return report_of(&source_text, "failed", &error);
    }
    let temp_len = fs::metadata(&temp_path).map(|meta| meta.len()).unwrap_or(0);
    if temp_len != opened.facts.size || storage::is_reparse_point(&temp_path) {
        let _ = discard_temp(managed, &temp_relative);
        let message = "El temporal no coincide con el origen. El original se conserva.";
        let _ = journal::set_stage(db, id, "failed", Some(message));
        return report_of(&source_text, "failed", message);
    }

    let mut destination_name = destination_name;
    let published = publish_without_replace(
        db,
        managed,
        id,
        &temp_path,
        &original_name,
        &mut destination_name,
        hooks,
    );
    let destination = match published {
        Ok(path) => path,
        Err(error) => {
            let _ = journal::set_stage(db, id, "needs_review", Some(&error));
            return report_of(&source_text, "needs_review", &error);
        }
    };
    if let Err(error) = journal::set_stage(db, id, "ready_for_db", None) {
        return report_of(&source_text, "needs_review", &error);
    }
    if hooks.fail_commit {
        return report_of(
            &source_text,
            "ready_for_db",
            "El alta no se ha confirmado. Se reintentará al abrir la aplicación. El original se conserva.",
        );
    }

    let title = title_of(&destination_name);
    let container = container_of(&destination_name);
    let destination_facts = match win::destination_facts(&destination) {
        Ok(facts) if facts.size == opened.facts.size => facts,
        Ok(_) => {
            let message = "El destino publicado no tiene el tamaño esperado. No se borra.";
            let _ = journal::set_stage(db, id, "needs_review", Some(message));
            return report_of(&source_text, "needs_review", message);
        }
        Err(error) => {
            let _ = journal::set_stage(db, id, "needs_review", Some(&error));
            return report_of(&source_text, "needs_review", &error);
        }
    };
    let destination_key = match win::path_key(&destination) {
        Ok(key) => key,
        Err(error) => return report_of(&source_text, "needs_review", &error),
    };
    let file_id = win::file_id_of(&destination_facts);
    if let Err(error) = journal::commit_card(
        db,
        id,
        library,
        &title,
        &destination_name,
        &destination_key,
        destination_facts.size as i64,
        &destination_facts.modified,
        &container,
        file_id.as_deref(),
    ) {
        let message =
            format!("El alta no se ha confirmado. Se reintentará al abrir la aplicación. {error}");
        return report_of(&source_text, "ready_for_db", &message);
    }

    if mode == Mode::Copy {
        let _ = journal::set_stage(db, id, "completed", None);
        return report_of(
            &source_text,
            "completed",
            "La ficha se ha guardado. La lista de la biblioteca se mostrará más adelante.",
        );
    }

    if cancel.load(Ordering::SeqCst) {
        let message = "Cancelada después de registrar la ficha. El original se conserva.";
        let _ = journal::set_stage(db, id, "needs_review", Some(message));
        return report_of(&source_text, "needs_review", message);
    }
    if let Err(error) = journal::set_stage(db, id, "source_delete_pending", None) {
        return report_of(&source_text, "needs_review", &error);
    }
    let identity = Some((opened.facts.file_id, opened.facts.volume_serial));
    match verify_and_delete(
        &opened,
        opened.facts.size,
        &opened.facts.modified,
        identity,
        &destination,
    ) {
        Ok(hash) => {
            if let Ok(stored) = stored_id(db, id) {
                if let Some(stored) = stored {
                    let _ = journal::store_hash(db, stored, &hash);
                }
            }
            let _ = journal::set_stage(db, id, "completed", None);
            report_of(
                &source_text,
                "completed",
                "La ficha se ha guardado y el original comprobado se ha retirado.",
            )
        }
        Err(error) => {
            let _ = journal::set_stage(db, id, "needs_review", Some(&error));
            report_of(&source_text, "needs_review", &error)
        }
    }
}

fn publish_without_replace(
    db: &Database,
    managed: &Path,
    id: i64,
    temp_path: &Path,
    original_name: &str,
    destination_name: &mut String,
    hooks: &mut ImportHooks,
) -> Result<PathBuf, String> {
    let mut hooked = false;
    for _ in 0..50 {
        let destination = resolve_file_name(managed, destination_name)?;
        if !hooked {
            if let Some(hook) = hooks.before_publish.as_mut() {
                hook(&destination);
            }
            hooked = true;
        }
        match win::publish_exclusive(temp_path, &destination) {
            Ok(()) => return Ok(destination),
            Err(win::PublishError::Occupied) => {
                let next = allocate_name(db, managed, original_name, Some(id))?;
                journal::set_destination(db, id, &next)?;
                *destination_name = next;
            }
            Err(win::PublishError::Failed(error)) => return Err(error),
        }
    }
    Err("No se ha encontrado un nombre libre sin sobrescribir.".into())
}

fn verify_and_delete(
    source: &win::OpenedFile,
    expected_size: u64,
    expected_modified: &str,
    expected_identity: Option<([u8; 16], u64)>,
    destination: &Path,
) -> Result<String, String> {
    let facts = win::current_facts(source)?;
    if facts.size != expected_size || facts.modified != expected_modified {
        return Err("El original ya no coincide con el archivo comprobado. Se conserva.".into());
    }
    if let Some((file_id, volume)) = expected_identity {
        if facts.file_id != file_id || facts.volume_serial != volume {
            return Err("La identidad del original ha cambiado. Se conserva.".into());
        }
    }
    let source_hash = win::hash_handle(source)?;
    let destination_hash = win::hash_path(destination)?;
    if source_hash != destination_hash {
        return Err("La huella del original no coincide con la del destino. Se conserva.".into());
    }
    win::delete_open_file(source)?;
    Ok(source_hash)
}

fn recover_copy(
    db: &Database,
    managed: &Path,
    operation: &journal::Operation,
) -> Result<(), String> {
    let destination = match resolve_file_name(managed, &operation.destination_relative_path) {
        Ok(path) => path,
        Err(error) => {
            return journal::set_stage(db, operation.id, "needs_review", Some(&error));
        }
    };
    let temp = match operation.temp_relative_path.as_deref() {
        Some(relative) => match resolve_inside(managed, relative) {
            Ok(path) => Some((relative.to_string(), path)),
            Err(error) => {
                return journal::set_stage(db, operation.id, "needs_review", Some(&error));
            }
        },
        None => None,
    };
    if operation.media_item_id.is_some()
        || operation.stage == "db_committed"
        || operation.stage == "source_delete_pending"
    {
        if operation.kind == "copy" {
            return journal::set_stage(db, operation.id, "completed", None);
        }
        return finish_move(db, operation, &destination);
    }

    let dest_ok = file_matches(&destination, operation.source_size_bytes as u64);
    let temp_ok = temp
        .as_ref()
        .is_some_and(|(_, path)| file_matches(path, operation.source_size_bytes as u64));
    if dest_ok && temp_ok {
        return journal::set_stage(
            db,
            operation.id,
            "needs_review",
            Some("Hay un temporal y un destino. No se borra ninguno."),
        );
    }
    if dest_ok {
        return commit_once(db, operation, &destination);
    }
    if destination.exists()
        || temp
            .as_ref()
            .is_some_and(|(_, path)| path.exists() && !temp_ok)
    {
        if let Some((relative, path)) = &temp {
            if path.exists() && !temp_ok {
                let _ = discard_temp(managed, relative);
            }
        }
        if destination.exists() && !dest_ok {
            return journal::set_stage(
                db,
                operation.id,
                "needs_review",
                Some("El destino existe pero no coincide con el origen. No se borra."),
            );
        }
        return journal::set_stage(
            db,
            operation.id,
            "failed",
            Some("La copia no llegó a completarse. El original se conserva."),
        );
    }
    if temp_ok {
        let (_, temp_path) = temp.unwrap();
        if let Err(error) =
            win::publish_exclusive(&temp_path, &destination).map_err(|error| match error {
                win::PublishError::Occupied => {
                    "El nombre de destino apareció durante la recuperación. No se sobrescribe."
                        .into()
                }
                win::PublishError::Failed(message) => message,
            })
        {
            return journal::set_stage(db, operation.id, "needs_review", Some(&error));
        }
        return commit_once(db, operation, &destination);
    }
    journal::set_stage(
        db,
        operation.id,
        "failed",
        Some("La copia no llegó a completarse. El original se conserva."),
    )
}

fn commit_once(
    db: &Database,
    operation: &journal::Operation,
    destination: &Path,
) -> Result<(), String> {
    let facts = match win::destination_facts(destination) {
        Ok(facts) if facts.size == operation.source_size_bytes as u64 => facts,
        Ok(_) => {
            return journal::set_stage(
                db,
                operation.id,
                "needs_review",
                Some("El destino no tiene el tamaño registrado. No se borra."),
            );
        }
        Err(error) => return journal::set_stage(db, operation.id, "needs_review", Some(&error)),
    };
    let key = match win::path_key(destination) {
        Ok(key) => key,
        Err(error) => return journal::set_stage(db, operation.id, "needs_review", Some(&error)),
    };
    let file_id = win::file_id_of(&facts);
    let result = journal::commit_card(
        db,
        operation.id,
        &operation.library,
        &title_of(&operation.destination_relative_path),
        &operation.destination_relative_path,
        &key,
        operation.source_size_bytes,
        &facts.modified,
        &container_of(&operation.destination_relative_path),
        file_id.as_deref(),
    );
    if let Err(error) = result {
        return journal::set_stage(db, operation.id, "needs_review", Some(&error));
    }
    if operation.kind == "copy" {
        return journal::set_stage(db, operation.id, "completed", None);
    }
    let refreshed = journal::pending(db)?
        .into_iter()
        .find(|item| item.id == operation.id);
    match refreshed {
        Some(operation) => finish_move(db, &operation, destination),
        None => Ok(()),
    }
}

fn finish_move(
    db: &Database,
    operation: &journal::Operation,
    destination: &Path,
) -> Result<(), String> {
    if operation.media_item_id.is_none() {
        return journal::set_stage(
            db,
            operation.id,
            "needs_review",
            Some("El alta no está confirmada. El original se conserva."),
        );
    }
    let source = PathBuf::from(&operation.source_path);
    if !source.exists() {
        return journal::set_stage(
            db,
            operation.id,
            "needs_review",
            Some(
                "El original ya no está y no se puede demostrar cuál se comprobó. La copia se conserva.",
            ),
        );
    }
    let opened = match win::open_source(&source) {
        Ok(opened) => opened,
        Err(error) => return journal::set_stage(db, operation.id, "needs_review", Some(&error)),
    };
    let Some(modified) = operation.source_modified_at.as_deref() else {
        return journal::set_stage(
            db,
            operation.id,
            "needs_review",
            Some("Falta la fecha del original. Se conserva."),
        );
    };
    if operation.stage != "source_delete_pending" {
        journal::set_stage(db, operation.id, "source_delete_pending", None)?;
    }
    match verify_and_delete(
        &opened,
        operation.source_size_bytes as u64,
        modified,
        None,
        destination,
    ) {
        Ok(hash) => {
            if let Some(stored) = operation.stored_file_id {
                journal::store_hash(db, stored, &hash)?;
            }
            journal::set_stage(db, operation.id, "completed", None)
        }
        Err(error) => journal::set_stage(db, operation.id, "needs_review", Some(&error)),
    }
}

fn file_matches(path: &Path, size: u64) -> bool {
    !storage::is_reparse_point(path)
        && fs::metadata(path)
            .map(|meta| meta.is_file() && meta.len() == size)
            .unwrap_or(false)
}

fn stored_id(db: &Database, operation_id: i64) -> Result<Option<i64>, String> {
    db.with_connection(|connection| {
        connection.query_row(
            "SELECT stored_file_id FROM file_operations WHERE id = ?1",
            rusqlite::params![operation_id],
            |row| row.get(0),
        )
    })
    .map_err(|error| error.to_string())
}

fn expand_folder(folder: &Path, managed: &Path, app_data: &Path) -> Vec<Candidate> {
    if let Err(error) = validate_folder(folder, managed, app_data) {
        return vec![Candidate::Note(folder.display().to_string(), error)];
    }
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) => {
            return vec![Candidate::Note(
                folder.display().to_string(),
                format!("No se ha podido leer la carpeta: {error}"),
            )];
        }
    };
    let mut items = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let name = path.display().to_string();
        if storage::is_reparse_point(&path) {
            items.push(Candidate::Note(name, "Es un enlace y no se sigue.".into()));
            continue;
        }
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() => items.push(Candidate::Note(
                name,
                "Las subcarpetas no se importan.".into(),
            )),
            Ok(meta) if meta.is_file() => items.push(Candidate::File(path)),
            _ => items.push(Candidate::Note(
                name,
                "No es un archivo normal y no se importa.".into(),
            )),
        }
    }
    items
}

fn validate_folder(folder: &Path, managed: &Path, app_data: &Path) -> Result<(), String> {
    if !folder.is_absolute() {
        return Err("La carpeta tiene que ser una ruta absoluta.".into());
    }
    if storage::is_reparse_point(folder) {
        return Err("La carpeta es un enlace y no se abre.".into());
    }
    if !folder.is_dir() {
        return Err("La ruta elegida no es una carpeta.".into());
    }
    reject_reserved(folder, managed, app_data)
}

fn validate_source(source: &Path, managed: &Path, app_data: &Path) -> Result<(), String> {
    if !source.is_absolute() {
        return Err("El archivo tiene que ser una ruta absoluta.".into());
    }
    if storage::is_reparse_point(source) {
        return Err("El origen es un enlace y no se abre.".into());
    }
    if source.is_dir() {
        return Err("La selección de archivos no admite carpetas.".into());
    }
    if !source.is_file() {
        return Err("El origen no existe o no es un archivo.".into());
    }
    reject_reserved(source, managed, app_data)
}

fn reject_reserved(path: &Path, managed: &Path, app_data: &Path) -> Result<(), String> {
    if win::is_inside(managed, path)? {
        return Err(
            "Un archivo de la carpeta administrada no se vuelve a importar por esta vía.".into(),
        );
    }
    let incoming = storage::incoming_dir(managed);
    if win::is_inside(&incoming, path).unwrap_or(false) {
        return Err("Un temporal de importación no se puede importar.".into());
    }
    if win::is_inside(app_data, path)? {
        return Err("Un archivo de los datos de la aplicación no se importa.".into());
    }
    Ok(())
}

fn allocate_name(
    db: &Database,
    managed: &Path,
    original: &str,
    except_id: Option<i64>,
) -> Result<String, String> {
    let base = safe_file_name(original);
    for index in 0..100 {
        let candidate = if index == 0 {
            base.clone()
        } else {
            with_suffix(&base, index + 1)
        };
        if candidate.contains(['/', '\\']) {
            continue;
        }
        let destination = managed.join(&candidate);
        if destination.exists() {
            continue;
        }
        if journal::destination_reserved(db, &candidate, except_id)? {
            continue;
        }
        if journal::path_key_available(db, &win::path_key(&destination)?)? {
            continue;
        }
        return Ok(candidate);
    }
    Err("No hay un nombre libre en la carpeta administrada.".into())
}

fn safe_file_name(original: &str) -> String {
    let mut name = String::new();
    for character in original.chars() {
        if matches!(
            character,
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
        ) || (character as u32) < 32
        {
            name.push('_');
        } else {
            name.push(character);
        }
    }
    let name = name.trim_end_matches([' ', '.']).to_string();
    let name = if name.is_empty() {
        "archivo".to_string()
    } else {
        name
    };
    let stem = Path::new(&name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem.to_ascii_uppercase().as_str()) {
        format!("archivo {name}")
    } else {
        name
    }
}

fn with_suffix(name: &str, number: usize) -> String {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(extension) => format!("{stem} ({number}).{extension}"),
        None => format!("{stem} ({number})"),
    }
}

fn temp_relative() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let number = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ticks = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!(".media-center/incoming/{ticks}-{number}.partial")
}

fn resolve_inside(managed: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.starts_with(['/', '\\']) || relative.contains("..") {
        return Err("La ruta de la operación sale de la carpeta administrada.".into());
    }
    let mut path = managed.to_path_buf();
    for part in relative.split(['/', '\\']) {
        if part.is_empty() || part == "." || part == ".." {
            return Err("La ruta de la operación sale de la carpeta administrada.".into());
        }
        path.push(part);
    }
    if !win::is_inside(managed, &path)? {
        return Err("La ruta de la operación sale de la carpeta administrada.".into());
    }
    Ok(path)
}

fn resolve_file_name(managed: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || name.contains("..")
    {
        return Err("El nombre de destino no es válido.".into());
    }
    let path = managed.join(name);
    if !win::is_inside(managed, &path)? {
        return Err("El destino sale de la carpeta administrada.".into());
    }
    Ok(path)
}

fn ensure_incoming(managed: &Path) -> Result<(), String> {
    if !managed.is_dir() || storage::is_reparse_point(managed) {
        return Err("La carpeta administrada no está disponible.".into());
    }
    let marker = managed.join(".media-center");
    if marker.exists() && storage::is_reparse_point(&marker) {
        return Err("La carpeta interna es un enlace.".into());
    }
    let incoming = storage::incoming_dir(managed);
    if incoming.exists() && storage::is_reparse_point(&incoming) {
        return Err("La carpeta de entrada es un enlace.".into());
    }
    fs::create_dir_all(&incoming)
        .map_err(|error| format!("No se ha podido crear la entrada: {error}"))?;
    if storage::is_reparse_point(&incoming) {
        return Err("La carpeta de entrada es un enlace.".into());
    }
    Ok(())
}

fn discard_temp(managed: &Path, relative: &str) -> Result<(), String> {
    let path = resolve_inside(managed, relative)?;
    let incoming = storage::incoming_dir(managed);
    if !win::is_inside(&incoming, &path)? {
        return Err("El temporal no está en la carpeta de entrada.".into());
    }
    if path.is_file() && !storage::is_reparse_point(&path) {
        fs::remove_file(&path)
            .map_err(|error| format!("No se ha podido retirar el temporal: {error}"))?;
    }
    Ok(())
}

fn title_of(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    if stem.is_empty() {
        file_name.to_string()
    } else {
        stem
    }
}

fn container_of(file_name: &str) -> String {
    Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .take(32)
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|ext| !ext.is_empty())
        .unwrap_or_else(|| "bin".into())
}

fn report_of(source: &str, state: &str, message: &str) -> ItemReport {
    ItemReport {
        source: source.to_string(),
        state: state.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests;
