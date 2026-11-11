//! Diario `file_operations` y alta de la ficha. Las transacciones son cortas:
//! no permanecen abiertas durante la copia.

use crate::db::Database;
use rusqlite::params;

pub const RENAMED_WITHOUT_IDENTITY: &str = "El diario no conserva la identidad del archivo ni la del volumen. No se crea la ficha solo porque el tamaño coincida y no se borra ningún archivo.";

#[derive(Debug, Clone)]
pub struct Operation {
    pub id: i64,
    pub kind: String,
    pub method: String,
    pub library: String,
    pub source_path: String,
    pub source_size_bytes: i64,
    pub source_modified_at: Option<String>,
    pub temp_relative_path: Option<String>,
    pub destination_relative_path: String,
    pub stage: String,
    pub media_item_id: Option<i64>,
    pub stored_file_id: Option<i64>,
}

pub fn insert_planned(
    db: &Database,
    kind: &str,
    method: &str,
    library: &str,
    source_path: &str,
    source_size: i64,
    source_modified: &str,
    temp_relative: Option<&str>,
    destination_relative: &str,
) -> Result<i64, String> {
    db.with_connection(|connection| {
        let tx = connection.transaction()?;
        let now = now_iso();
        tx.execute(
            "INSERT INTO file_operations (
                kind, method, library, source_path, source_size_bytes, source_modified_at,
                temp_relative_path, destination_relative_path, stage, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'planned', ?9, ?9)",
            params![
                kind,
                method,
                library,
                source_path,
                source_size,
                source_modified,
                temp_relative,
                destination_relative,
                now,
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    })
    .map_err(show)
}

pub fn set_stage(db: &Database, id: i64, stage: &str, error: Option<&str>) -> Result<(), String> {
    db.with_connection(|connection| {
        connection.execute(
            "UPDATE file_operations SET stage = ?1, error = ?2, updated_at = ?3 WHERE id = ?4",
            params![stage, error, now_iso(), id],
        )?;
        Ok(())
    })
    .map_err(show)
}

pub fn set_destination(db: &Database, id: i64, destination_relative: &str) -> Result<(), String> {
    db.with_connection(|connection| {
        connection.execute(
            "UPDATE file_operations
             SET destination_relative_path = ?1, updated_at = ?2
             WHERE id = ?3",
            params![destination_relative, now_iso(), id],
        )?;
        Ok(())
    })
    .map_err(show)
}

pub fn destination_reserved(
    db: &Database,
    destination_relative: &str,
    except_id: Option<i64>,
) -> Result<bool, String> {
    db.with_connection(|connection| {
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM file_operations
             WHERE destination_relative_path = ?1
               AND stage NOT IN ('failed', 'needs_review')
               AND id != ?2",
            params![destination_relative, except_id.unwrap_or(0)],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    })
    .map_err(show)
}

pub fn path_key_available(db: &Database, path_key: &str) -> Result<bool, String> {
    db.with_connection(|connection| {
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM stored_files WHERE path_key = ?1 AND status = 'available'",
            params![path_key],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    })
    .map_err(show)
}

pub fn source_already_imported(db: &Database, source_path: &str) -> Result<bool, String> {
    db.with_connection(|connection| {
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM file_operations AS operation
             JOIN stored_files AS stored ON stored.id = operation.stored_file_id
             WHERE operation.source_path = ?1
               AND operation.stage = 'completed'
               AND stored.status = 'available'",
            params![source_path],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    })
    .map_err(show)
}

pub struct NewCard {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<i64>,
    pub description: String,
    pub duration_ms: Option<i64>,
    pub snapshot: String,
    pub cover_relative: Option<String>,
}

pub fn card_id(db: &Database, operation_id: i64) -> Result<Option<i64>, String> {
    db.with_connection(|connection| {
        connection.query_row(
            "SELECT media_item_id FROM file_operations WHERE id = ?1",
            params![operation_id],
            |row| row.get(0),
        )
    })
    .map_err(show)
}

pub fn commit_card(
    db: &Database,
    operation_id: i64,
    library: &str,
    card: &NewCard,
    relative_path: &str,
    path_key: &str,
    size: i64,
    modified: &str,
    container: &str,
    file_id: Option<&str>,
) -> Result<(), String> {
    db.with_connection(|connection| {
        let tx = connection.transaction()?;
        let existing: Option<i64> = tx.query_row(
            "SELECT media_item_id FROM file_operations WHERE id = ?1",
            params![operation_id],
            |row| row.get(0),
        )?;
        if existing.is_some() {
            tx.execute(
                "UPDATE file_operations
                 SET stage = 'db_committed', error = NULL, updated_at = ?1
                 WHERE id = ?2",
                params![now_iso(), operation_id],
            )?;
            tx.commit()?;
            return Ok(());
        }
        let now = now_iso();
        tx.execute(
            "INSERT INTO stored_files (
                storage, path, path_key, size_bytes, modified_at, content_hash, file_id,
                container, status, created_at, updated_at
             ) VALUES ('managed', ?1, ?2, ?3, ?4, NULL, ?5, ?6, 'available', ?7, ?7)",
            params![
                relative_path,
                path_key,
                size,
                modified,
                file_id,
                container,
                now
            ],
        )?;
        let stored_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO media_items (
                library, title, artist, album, genre, year, description, duration_ms, favorite,
                imported_snapshot, user_edited_fields, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, '{}', ?10, ?10)",
            params![
                library,
                card.title,
                card.artist,
                card.album,
                card.genre,
                card.year,
                card.description,
                card.duration_ms,
                card.snapshot,
                now
            ],
        )?;
        let item_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO item_files (media_item_id, stored_file_id, role, created_at)
             VALUES (?1, ?2, 'primary', ?3)",
            params![item_id, stored_id, now],
        )?;
        if let Some(relative) = card.cover_relative.as_deref() {
            insert_embedded_tx(&tx, item_id, relative, &now)?;
        }
        tx.execute(
            "UPDATE file_operations
             SET stage = 'db_committed', media_item_id = ?1, stored_file_id = ?2,
                 error = NULL, updated_at = ?3
             WHERE id = ?4",
            params![item_id, stored_id, now, operation_id],
        )?;
        tx.commit()?;
        Ok(())
    })
    .map_err(show)
}

#[cfg(test)]
pub fn insert_embedded(db: &Database, item_id: i64, relative: &str) -> Result<bool, String> {
    db.with_connection(|connection| {
        let tx = connection.transaction()?;
        let inserted = insert_embedded_tx(&tx, item_id, relative, &now_iso())?;
        tx.commit()?;
        Ok(inserted)
    })
    .map_err(show)
}

fn insert_embedded_tx(
    tx: &rusqlite::Transaction<'_>,
    item_id: i64,
    relative: &str,
    now: &str,
) -> rusqlite::Result<bool> {
    let embedded: i64 = tx.query_row(
        "SELECT COUNT(*) FROM media_images WHERE media_item_id = ?1 AND source = 'embedded'",
        params![item_id],
        |row| row.get(0),
    )?;
    if embedded > 0 {
        return Ok(false);
    }
    let used: i64 = tx.query_row(
        "SELECT COUNT(*) FROM media_images WHERE relative_path = ?1",
        params![relative],
        |row| row.get(0),
    )?;
    if used > 0 {
        return Ok(false);
    }
    let selected_rows: i64 = tx.query_row(
        "SELECT COUNT(*) FROM media_images WHERE media_item_id = ?1 AND selected = 1",
        params![item_id],
        |row| row.get(0),
    )?;
    let selected = if selected_rows == 0 { 1 } else { 0 };
    tx.execute(
        "INSERT INTO media_images (
            media_item_id, source, relative_path, selected, created_at, updated_at
         ) VALUES (?1, 'embedded', ?2, ?3, ?4, ?4)",
        params![item_id, relative, selected, now],
    )?;
    Ok(true)
}

#[cfg(test)]
pub fn save_card_text(db: &Database, item_id: i64, card: &NewCard) -> Result<(), String> {
    db.with_connection(|connection| {
        connection.execute(
            "UPDATE media_items
             SET title = ?1, artist = ?2, album = ?3, genre = ?4, year = ?5,
                 description = ?6, duration_ms = ?7, imported_snapshot = ?8, updated_at = ?9
             WHERE id = ?10",
            params![
                card.title,
                card.artist,
                card.album,
                card.genre,
                card.year,
                card.description,
                card.duration_ms,
                card.snapshot,
                now_iso(),
                item_id
            ],
        )?;
        Ok(())
    })
    .map_err(show)
}

pub fn store_hash(db: &Database, stored_file_id: i64, hash: &str) -> Result<(), String> {
    db.with_connection(|connection| {
        connection.execute(
            "UPDATE stored_files SET content_hash = ?1, updated_at = ?2 WHERE id = ?3",
            params![hash, now_iso(), stored_file_id],
        )?;
        Ok(())
    })
    .map_err(show)
}

pub fn pending(db: &Database) -> Result<Vec<Operation>, String> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, kind, method, library, source_path, source_size_bytes,
                    source_modified_at, temp_relative_path, destination_relative_path,
                    stage, media_item_id, stored_file_id
             FROM file_operations
             WHERE stage NOT IN ('completed', 'failed', 'needs_review')
             ORDER BY id",
        )?;
        let rows = statement.query_map([], read_operation)?;
        rows.collect::<Result<Vec<_>, _>>()
    })
    .map_err(show)
}

fn read_operation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Operation> {
    Ok(Operation {
        id: row.get(0)?,
        kind: row.get(1)?,
        method: row.get(2)?,
        library: row.get(3)?,
        source_path: row.get(4)?,
        source_size_bytes: row.get(5)?,
        source_modified_at: row.get(6)?,
        temp_relative_path: row.get(7)?,
        destination_relative_path: row.get(8)?,
        stage: row.get(9)?,
        media_item_id: row.get(10)?,
        stored_file_id: row.get(11)?,
    })
}

fn now_iso() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    crate::import::win::format_unix_millis(millis as u64)
}

fn show(error: impl std::fmt::Display) -> String {
    error.to_string()
}
