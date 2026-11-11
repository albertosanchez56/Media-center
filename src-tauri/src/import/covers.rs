//! Copia de una portada ya comprobada. La creación es exclusiva: un archivo
//! existente no se sustituye. Solo se borra una imagen creada por el intento
//! en curso si la transacción no llegó a referenciarla.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;

use crate::db::Database;
use crate::import::win;
use crate::storage::is_reparse_point;

pub struct StoredCover {
    pub relative: String,
    pub created: bool,
}

pub fn store(
    db: &Database,
    images: &Path,
    operation_id: i64,
    bytes: &[u8],
    extension: &str,
) -> Result<Option<StoredCover>, String> {
    fs::create_dir_all(images)
        .map_err(|error| format!("No se ha podido preparar la carpeta de imágenes: {error}"))?;
    if extension.is_empty()
        || !extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return Ok(None);
    }
    for suffix in 0..8 {
        let relative = if suffix == 0 {
            format!("embedded/{operation_id}.{extension}")
        } else {
            format!("embedded/{operation_id}-{suffix}.{extension}")
        };
        match place(db, images, &relative, bytes)? {
            Place::Ready(stored) => return Ok(Some(stored)),
            Place::Occupied => continue,
        }
    }
    Ok(None)
}

pub fn delete_if_unreferenced(db: &Database, images: &Path, relative: &str) -> Result<(), String> {
    if image_source(db, relative)?.is_some() {
        return Ok(());
    }
    let path = resolve_image(images, relative)?;
    if path.is_file() && !is_reparse_point(&path) {
        fs::remove_file(&path).map_err(|error| {
            format!("No se ha podido retirar la imagen sin referencia: {error}")
        })?;
    }
    Ok(())
}

enum Place {
    Ready(StoredCover),
    Occupied,
}

fn place(db: &Database, images: &Path, relative: &str, bytes: &[u8]) -> Result<Place, String> {
    let path = resolve_image(images, relative)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("No se ha podido preparar la carpeta de la portada: {error}")
        })?;
    }
    if is_reparse_point(&path) {
        return Ok(Place::Occupied);
    }
    if path.exists() {
        return match image_source(db, relative)? {
            Some(source) if source == "user" => Ok(Place::Occupied),
            Some(_) if same_bytes(&path, bytes) => Ok(Place::Ready(StoredCover {
                relative: relative.to_string(),
                created: false,
            })),
            Some(_) => Ok(Place::Occupied),
            None if same_bytes(&path, bytes) => Ok(Place::Ready(StoredCover {
                relative: relative.to_string(),
                created: false,
            })),
            None => {
                fs::remove_file(&path).map_err(|error| {
                    format!("No se ha podido retirar la imagen sin referencia: {error}")
                })?;
                write_new(&path, bytes)
                    .map_err(|error| format!("No se ha podido guardar la portada: {error}"))?;
                Ok(Place::Ready(StoredCover {
                    relative: relative.to_string(),
                    created: true,
                }))
            }
        };
    }
    match write_new(&path, bytes) {
        Ok(()) => Ok(Place::Ready(StoredCover {
            relative: relative.to_string(),
            created: true,
        })),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if image_source(db, relative)?.as_deref() == Some("user") {
                return Ok(Place::Occupied);
            }
            if same_bytes(&path, bytes) {
                Ok(Place::Ready(StoredCover {
                    relative: relative.to_string(),
                    created: false,
                }))
            } else {
                Ok(Place::Occupied)
            }
        }
        Err(error) => Err(format!("No se ha podido guardar la portada: {error}")),
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn same_bytes(path: &Path, bytes: &[u8]) -> bool {
    match fs::metadata(path) {
        Ok(metadata) if metadata.len() == bytes.len() as u64 && metadata.is_file() => {
            fs::read(path).ok().is_some_and(|current| current == bytes)
        }
        _ => false,
    }
}

fn image_source(db: &Database, relative: &str) -> Result<Option<String>, String> {
    db.with_connection(|connection| {
        connection
            .query_row(
                "SELECT source FROM media_images WHERE relative_path = ?1 LIMIT 1",
                [relative],
                |row| row.get(0),
            )
            .optional()
    })
    .map_err(|error| error.to_string())
}

fn resolve_image(images: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.starts_with(['/', '\\']) || relative.contains("..") {
        return Err("La ruta de la imagen sale de la carpeta de imágenes.".into());
    }
    let mut path = images.to_path_buf();
    for part in relative.split(['/', '\\']) {
        if part.is_empty() || part == "." || part == ".." {
            return Err("La ruta de la imagen sale de la carpeta de imágenes.".into());
        }
        path.push(part);
    }
    if !win::is_inside(images, &path)? {
        return Err("La ruta de la imagen sale de la carpeta de imágenes.".into());
    }
    Ok(path)
}
