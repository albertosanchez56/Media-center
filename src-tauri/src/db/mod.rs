//! Base de datos SQLite. Angular no ejecuta SQL: solo este módulo abre `library.db`.

mod migrations;
mod settings;

use rusqlite::{Connection, ErrorCode};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const BUSY_TIMEOUT_MS: i64 = 5000;
const SYNCHRONOUS_FULL: i64 = 2;

#[derive(Clone, Copy)]
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationOutcome {
    Created,
    Updated,
    AlreadyCurrent,
}

#[derive(Debug)]
pub enum DatabaseError {
    Open {
        path: PathBuf,
        message: String,
    },
    Migration {
        version: i64,
        message: String,
    },
    NewerSchema {
        path: PathBuf,
        found: i64,
        known: i64,
    },
    Unexpected {
        path: PathBuf,
        message: String,
    },
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open { path, message } => write!(
                formatter,
                "No se ha podido abrir la base de datos en {}: {message}",
                path.display()
            ),
            Self::Migration { version, message } => write!(
                formatter,
                "La migración {version} ha fallado y se ha deshecho: {message}"
            ),
            Self::NewerSchema { path, found, known } => write!(
                formatter,
                "La base en {} contiene la migración {found}, posterior a la versión {known} que conoce esta aplicación. No se modifica.",
                path.display()
            ),
            Self::Unexpected { path, message } => {
                write!(formatter, "{message} ({})", path.display())
            }
        }
    }
}

impl std::error::Error for DatabaseError {}

pub struct Database {
    path: PathBuf,
    connection: Mutex<Connection>,
}

impl Database {
    pub fn check_ready(&self) -> Result<(), DatabaseError> {
        let connection = self
            .connection
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        verify_pragmas(&self.path, &connection)?;
        let version: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error(&self.path, error))?;
        if version < 1 {
            return Err(DatabaseError::Unexpected {
                path: self.path.clone(),
                message: "La base no contiene la migración inicial y no se usa.".into(),
            });
        }
        Ok(())
    }

    pub fn with_connection<T>(
        &self,
        body: impl FnOnce(&mut Connection) -> Result<T, rusqlite::Error>,
    ) -> Result<T, DatabaseError> {
        let mut connection = self
            .connection
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        body(&mut connection).map_err(|error| sqlite_error(&self.path, error))
    }

    #[cfg(test)]
    pub fn try_with_connection<T>(
        &self,
        body: impl FnOnce(&mut Connection) -> Result<T, rusqlite::Error>,
    ) -> Result<Option<T>, DatabaseError> {
        let mut connection = match self.connection.try_lock() {
            Ok(connection) => connection,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
            Err(std::sync::TryLockError::Poisoned(poison)) => poison.into_inner(),
        };
        body(&mut connection)
            .map(Some)
            .map_err(|error| sqlite_error(&self.path, error))
    }
}

pub fn initialize(path: &Path) -> Result<Database, DatabaseError> {
    let (connection, _) = open_and_migrate(path, &migrations::PRODUCT)?;
    Ok(Database {
        path: path.to_path_buf(),
        connection: Mutex::new(connection),
    })
}

fn open_and_migrate(
    path: &Path,
    migrations: &[Migration],
) -> Result<(Connection, MigrationOutcome), DatabaseError> {
    validate_migrations(path, migrations)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.is_dir()
    {
        return Err(DatabaseError::Open {
            path: path.to_path_buf(),
            message: "El directorio de la base de datos no existe.".into(),
        });
    }
    if path.is_dir() {
        return Err(DatabaseError::Unexpected {
            path: path.to_path_buf(),
            message: "La ruta de la base de datos es un directorio y no se modifica.".into(),
        });
    }

    let mut connection = Connection::open(path).map_err(|error| sqlite_error(path, error))?;
    let applied = match applied_versions(path, &connection) {
        Ok(applied) => applied,
        Err(error) => {
            drop(connection);
            return Err(error);
        }
    };
    let pending_from = match pending_start(path, &applied, migrations) {
        Ok(pending_from) => pending_from,
        Err(error) => {
            drop(connection);
            return Err(error);
        }
    };

    if let Err(error) = configure(path, &connection) {
        drop(connection);
        return Err(error);
    }
    if pending_from == migrations.len() {
        return Ok((connection, MigrationOutcome::AlreadyCurrent));
    }

    let outcome = if applied.is_empty() {
        MigrationOutcome::Created
    } else {
        MigrationOutcome::Updated
    };
    for migration in &migrations[pending_from..] {
        if let Err(error) = apply_migration(path, &mut connection, migration) {
            drop(connection);
            return Err(error);
        }
    }
    Ok((connection, outcome))
}

fn validate_migrations(path: &Path, migrations: &[Migration]) -> Result<(), DatabaseError> {
    if migrations.is_empty() {
        return Err(DatabaseError::Unexpected {
            path: path.to_path_buf(),
            message: "No hay migraciones conocidas. La base no se abre.".into(),
        });
    }
    for (index, migration) in migrations.iter().enumerate() {
        let expected = index as i64 + 1;
        if migration.version != expected
            || migration.name.trim().is_empty()
            || migration.sql.trim().is_empty()
        {
            return Err(DatabaseError::Unexpected {
                path: path.to_path_buf(),
                message: format!(
                    "Las migraciones conocidas deben numerarse desde 1, sin huecos ni SQL vacío. Se esperaba la versión {expected}."
                ),
            });
        }
    }
    Ok(())
}

fn configure(path: &Path, connection: &Connection) -> Result<(), DatabaseError> {
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| sqlite_error(path, error))?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| sqlite_error(path, error))?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| sqlite_error(path, error))?;
    connection
        .busy_timeout(BUSY_TIMEOUT)
        .map_err(|error| sqlite_error(path, error))?;
    verify_pragmas(path, connection)
}

fn verify_pragmas(path: &Path, connection: &Connection) -> Result<(), DatabaseError> {
    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|error| sqlite_error(path, error))?;
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(|error| sqlite_error(path, error))?;
    let synchronous: i64 = connection
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .map_err(|error| sqlite_error(path, error))?;
    let busy_timeout: i64 = connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .map_err(|error| sqlite_error(path, error))?;
    if foreign_keys != 1
        || !journal_mode.eq_ignore_ascii_case("wal")
        || synchronous != SYNCHRONOUS_FULL
        || busy_timeout != BUSY_TIMEOUT_MS
    {
        return Err(DatabaseError::Unexpected {
            path: path.to_path_buf(),
            message: format!(
                "La conexión no tiene la configuración exigida: foreign_keys={foreign_keys}, journal_mode={journal_mode}, synchronous={synchronous}, busy_timeout={busy_timeout}."
            ),
        });
    }
    Ok(())
}

fn applied_versions(path: &Path, connection: &Connection) -> Result<Vec<i64>, DatabaseError> {
    let names = table_names(path, connection)?;
    let has_registry = names.iter().any(|name| name == "schema_migrations");
    if !has_registry {
        if names.is_empty() {
            return Ok(Vec::new());
        }
        return Err(DatabaseError::Unexpected {
            path: path.to_path_buf(),
            message: "La base ya contiene tablas que esta aplicación no reconoce y no se modifica."
                .into(),
        });
    }
    let mut statement = connection
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .map_err(|error| sqlite_error(path, error))?;
    let versions = statement
        .query_map([], |row| row.get(0))
        .map_err(|error| sqlite_error(path, error))?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|error| sqlite_error(path, error))?;
    Ok(versions)
}

fn table_names(path: &Path, connection: &Connection) -> Result<Vec<String>, DatabaseError> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|error| sqlite_error(path, error))?;
    let names = statement
        .query_map([], |row| row.get(0))
        .map_err(|error| sqlite_error(path, error))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|error| sqlite_error(path, error))?;
    Ok(names)
}

fn pending_start(
    path: &Path,
    applied: &[i64],
    migrations: &[Migration],
) -> Result<usize, DatabaseError> {
    let known_max = migrations[migrations.len() - 1].version;
    if let Some(found) = applied.iter().copied().find(|version| *version > known_max) {
        return Err(DatabaseError::NewerSchema {
            path: path.to_path_buf(),
            found,
            known: known_max,
        });
    }
    let known: Vec<i64> = migrations
        .iter()
        .map(|migration| migration.version)
        .collect();
    if applied.len() > known.len() || applied != &known[..applied.len()] {
        return Err(DatabaseError::Unexpected {
            path: path.to_path_buf(),
            message: "El historial de migraciones no coincide con esta aplicación. No se modifica."
                .into(),
        });
    }
    Ok(applied.len())
}

fn apply_migration(
    path: &Path,
    connection: &mut Connection,
    migration: &Migration,
) -> Result<(), DatabaseError> {
    let transaction = connection
        .transaction()
        .map_err(|error| sqlite_error(path, error))?;
    transaction
        .execute_batch(migration.sql)
        .map_err(|error| DatabaseError::Migration {
            version: migration.version,
            message: error.to_string(),
        })?;
    transaction
        .execute(
            "INSERT INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            rusqlite::params![migration.version, migration.name],
        )
        .map_err(|error| DatabaseError::Migration {
            version: migration.version,
            message: error.to_string(),
        })?;
    transaction
        .commit()
        .map_err(|error| DatabaseError::Migration {
            version: migration.version,
            message: error.to_string(),
        })?;
    Ok(())
}

fn sqlite_error(path: &Path, error: rusqlite::Error) -> DatabaseError {
    let not_a_database = match &error {
        rusqlite::Error::SqliteFailure(code, _) => code.code == ErrorCode::NotADatabase,
        _ => false,
    } || error.to_string().contains("not a database");
    if not_a_database {
        DatabaseError::Unexpected {
            path: path.to_path_buf(),
            message: "El archivo no es una base de SQLite y no se modifica.".into(),
        }
    } else {
        DatabaseError::Open {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    const WHEN: &str = "2026-11-09T18:00:00Z";
    const PROBE_SQL: &str = "CREATE TABLE migration_probe (id INTEGER PRIMARY KEY) STRICT;";
    const BROKEN_SQL: &str = "CREATE TABLE migration_probe (id INTEGER PRIMARY KEY) STRICT; INSERT INTO migration_probe (id) VALUES ('no-entero');";

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
                .join(format!("mc-f22-{}-{nanos}-{unique}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn database(&self) -> PathBuf {
            self.path.join("library.db")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn product() -> &'static [Migration] {
        &migrations::PRODUCT
    }

    fn assert_integrity(connection: &Connection) {
        let report: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(report, "ok");
        let violations: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(violations, 0);
    }

    fn assert_pragmas(connection: &Connection) {
        verify_pragmas(Path::new("temporal"), connection).unwrap();
    }

    fn is_constraint(error: rusqlite::Error) -> bool {
        matches!(
            error,
            rusqlite::Error::SqliteFailure(code, _)
                if code.code == ErrorCode::ConstraintViolation
        )
    }

    fn insert_file(connection: &Connection, path_key: &str, bytes: i64) -> i64 {
        connection
            .execute(
                "INSERT INTO stored_files (
                    storage, path, path_key, size_bytes, modified_at, container, status,
                    created_at, updated_at
                 ) VALUES ('managed', ?1, ?2, ?3, ?4, 'flac', 'available', ?4, ?4)",
                rusqlite::params![format!("audio/{path_key}.flac"), path_key, bytes, WHEN],
            )
            .unwrap();
        connection.last_insert_rowid()
    }

    fn insert_item(connection: &Connection, title: &str) -> i64 {
        connection
            .execute(
                "INSERT INTO media_items (
                    library, title, artist, album, genre, description, favorite,
                    imported_snapshot, user_edited_fields, created_at, updated_at
                 ) VALUES ('music', ?1, '', '', '', '', 0, '{}', '[]', ?2, ?2)",
                rusqlite::params![title, WHEN],
            )
            .unwrap();
        connection.last_insert_rowid()
    }

    fn insert_link(connection: &Connection, item: i64, file: i64, role: &str) {
        connection
            .execute(
                "INSERT INTO item_files (media_item_id, stored_file_id, role, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![item, file, role, WHEN],
            )
            .unwrap();
    }

    fn count(connection: &Connection, sql: &str) -> i64 {
        connection.query_row(sql, [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn new_database_creates_the_approved_schema() {
        let scratch = Scratch::new();
        let path = scratch.database();
        assert!(!path.exists());
        let database = initialize(&path).unwrap();
        database.check_ready().unwrap();
        let connection = database
            .connection
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_pragmas(&connection);
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM schema_migrations"),
            1
        );
        let name: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "initial");

        let mut names = table_names(&path, &connection).unwrap();
        names.sort();
        assert_eq!(
            names,
            vec![
                "app_settings",
                "download_batches",
                "download_jobs",
                "file_operations",
                "item_files",
                "media_images",
                "media_items",
                "playlist_items",
                "playlists",
                "schema_migrations",
                "stored_files",
            ]
        );
        let indexes: Vec<String> = connection
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'index' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            indexes,
            vec![
                "download_jobs_source_key",
                "item_files_one_associated_video",
                "item_files_one_primary",
                "media_images_one_selected",
            ]
        );
        let sql = product()[0].sql;
        assert!(!sql.contains("categories"));
        assert!(!sql.contains("collections"));
        assert!(!sql.contains("collection_items"));
        assert_integrity(&connection);
    }

    #[test]
    fn constraints_reject_values_outside_the_schema() {
        let scratch = Scratch::new();
        let (connection, outcome) = open_and_migrate(&scratch.database(), product()).unwrap();
        assert_eq!(outcome, MigrationOutcome::Created);
        let file = insert_file(&connection, "tema", 12);
        let item = insert_item(&connection, "Tema");
        insert_link(&connection, item, file, "primary");

        let invalid_library = connection.execute(
            "INSERT INTO media_items (
                library, title, artist, album, genre, description, favorite,
                imported_snapshot, user_edited_fields, created_at, updated_at
             ) VALUES ('podcast', 'X', '', '', '', '', 0, '{}', '[]', ?1, ?1)",
            [WHEN],
        );
        assert!(invalid_library.is_err_and(|error| is_constraint(error)));

        let invalid_json = connection.execute(
            "INSERT INTO media_items (
                library, title, artist, album, genre, description, favorite,
                imported_snapshot, user_edited_fields, created_at, updated_at
             ) VALUES ('music', 'X', '', '', '', '', 0, 'no-json', '[]', ?1, ?1)",
            [WHEN],
        );
        assert!(invalid_json.is_err_and(|error| is_constraint(error)));

        let second_primary = connection.execute(
            "INSERT INTO item_files (media_item_id, stored_file_id, role, created_at)
             VALUES (?1, ?2, 'primary', ?3)",
            rusqlite::params![item, file, WHEN],
        );
        assert!(second_primary.is_err_and(|error| is_constraint(error)));

        insert_link(&connection, item, file, "associated_video");
        let second_video = connection.execute(
            "INSERT INTO item_files (media_item_id, stored_file_id, role, created_at)
             VALUES (?1, ?2, 'associated_video', ?3)",
            rusqlite::params![item, file, WHEN],
        );
        assert!(second_video.is_err_and(|error| is_constraint(error)));

        let missing_item = connection.execute(
            "INSERT INTO item_files (media_item_id, stored_file_id, role, created_at)
             VALUES (999, ?1, 'primary', ?2)",
            rusqlite::params![file, WHEN],
        );
        assert!(missing_item.is_err_and(|error| is_constraint(error)));

        let duplicate_key = connection.execute(
            "INSERT INTO stored_files (
                storage, path, path_key, size_bytes, modified_at, container, status,
                created_at, updated_at
             ) VALUES ('external', 'D:/otro.flac', 'tema', 12, ?1, 'flac', 'available', ?1, ?1)",
            [WHEN],
        );
        assert!(duplicate_key.is_err_and(|error| is_constraint(error)));

        connection
            .execute(
                "INSERT INTO download_batches (
                    source_url, kind, library, mode, format, quality, cancel_requested,
                    created_at, updated_at
                 ) VALUES ('https://example.test/a', 'single', 'music', 'audio', 'flac', 'high', 0, ?1, ?1)",
                [WHEN],
            )
            .unwrap();
        let batch = connection.last_insert_rowid();
        for position in [0, 1] {
            connection
                .execute(
                    "INSERT INTO download_jobs (
                        batch_id, position, source_url, source_key, title, status, attempt_count,
                        created_at, updated_at
                     ) VALUES (?1, ?2, 'https://example.test/a', 'same-key', 'Tema', 'queued', 0, ?3, ?3)",
                    rusqlite::params![batch, position, WHEN],
                )
                .unwrap();
        }
        assert_integrity(&connection);
    }

    #[test]
    fn file_card_and_link_keep_their_own_rows() {
        let scratch = Scratch::new();
        let media = scratch.path.join("canción con espacio.flac");
        fs::write(&media, b"audio-original").unwrap();
        let (connection, _) = open_and_migrate(&scratch.database(), product()).unwrap();
        let file = insert_file(&connection, "cancion", 15);
        let first = insert_item(&connection, "Primera ficha");
        let second = insert_item(&connection, "Segunda ficha");
        insert_link(&connection, first, file, "primary");
        insert_link(&connection, second, file, "primary");
        connection
            .execute(
                "INSERT INTO playlists (name, library, created_at, updated_at)
                 VALUES ('Favoritas', 'music', ?1, ?1)",
                [WHEN],
            )
            .unwrap();
        let playlist = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO playlist_items (playlist_id, media_item_id, position)
                 VALUES (?1, ?2, 0)",
                [playlist, first],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO media_images (
                    media_item_id, source, relative_path, selected, created_at, updated_at
                 ) VALUES (?1, 'embedded', 'covers/a.png', 1, ?2, ?2)",
                rusqlite::params![first, WHEN],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO download_batches (
                    source_url, kind, library, mode, format, quality, cancel_requested,
                    created_at, updated_at
                 ) VALUES ('https://example.test/b', 'single', 'music', 'audio', 'flac', 'high', 0, ?1, ?1)",
                [WHEN],
            )
            .unwrap();
        let batch = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO download_jobs (
                    batch_id, position, source_url, title, status, attempt_count,
                    media_item_id, stored_file_id, created_at, updated_at
                 ) VALUES (?1, 0, 'https://example.test/b', 'Primera', 'completed', 1, ?2, ?3, ?4, ?4)",
                rusqlite::params![batch, first, file, WHEN],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO file_operations (
                    kind, method, library, source_path, source_size_bytes,
                    destination_relative_path, stage, media_item_id, stored_file_id,
                    created_at, updated_at
                 ) VALUES ('copy', 'copy', 'music', ?1, 15, 'audio/cancion.flac', 'completed', ?2, ?3, ?4, ?4)",
                rusqlite::params![media.to_string_lossy(), first, file, WHEN],
            )
            .unwrap();
        assert_integrity(&connection);

        let delete_file = connection.execute("DELETE FROM stored_files WHERE id = ?1", [file]);
        assert!(delete_file.is_err_and(|error| is_constraint(error)));
        assert_eq!(fs::read(&media).unwrap(), b"audio-original");

        connection
            .execute("DELETE FROM media_items WHERE id = ?1", [first])
            .unwrap();
        let remaining_links: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM item_files WHERE media_item_id = ?1",
                [first],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining_links, 0);
        assert_eq!(count(&connection, "SELECT COUNT(*) FROM playlist_items"), 0);
        assert_eq!(count(&connection, "SELECT COUNT(*) FROM media_images"), 0);
        assert_eq!(count(&connection, "SELECT COUNT(*) FROM stored_files"), 1);
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM item_files"),
            1,
            "la segunda ficha sigue vinculada al mismo archivo"
        );
        let job_item: Option<i64> = connection
            .query_row("SELECT media_item_id FROM download_jobs", [], |row| {
                row.get(0)
            })
            .unwrap();
        let job_file: Option<i64> = connection
            .query_row("SELECT stored_file_id FROM download_jobs", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(job_item, None);
        assert_eq!(job_file, Some(file));
        let operation_item: Option<i64> = connection
            .query_row("SELECT media_item_id FROM file_operations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(operation_item, None);
        assert_eq!(fs::read(&media).unwrap(), b"audio-original");
        assert_integrity(&connection);

        connection
            .execute("DELETE FROM media_items WHERE id = ?1", [second])
            .unwrap();
        connection
            .execute("DELETE FROM stored_files WHERE id = ?1", [file])
            .unwrap();
        assert_eq!(count(&connection, "SELECT COUNT(*) FROM stored_files"), 0);
        assert_eq!(fs::read(&media).unwrap(), b"audio-original");
        assert_integrity(&connection);
    }

    #[test]
    fn playlists_keep_repeated_items_at_distinct_positions() {
        let scratch = Scratch::new();
        let (connection, _) = open_and_migrate(&scratch.database(), product()).unwrap();
        let file = insert_file(&connection, "tema", 4);
        let item = insert_item(&connection, "Tema");
        insert_link(&connection, item, file, "primary");
        connection
            .execute(
                "INSERT INTO playlists (name, description, library, created_at, updated_at)
                 VALUES ('Mezcla', NULL, NULL, ?1, ?1)",
                [WHEN],
            )
            .unwrap();
        let playlist = connection.last_insert_rowid();
        for position in [0, 1] {
            connection
                .execute(
                    "INSERT INTO playlist_items (playlist_id, media_item_id, position)
                     VALUES (?1, ?2, ?3)",
                    [playlist, item, position],
                )
                .unwrap();
        }
        let repeated_position = connection.execute(
            "INSERT INTO playlist_items (playlist_id, media_item_id, position)
             VALUES (?1, ?2, 0)",
            [playlist, item],
        );
        assert!(repeated_position.is_err_and(|error| is_constraint(error)));
        assert_eq!(count(&connection, "SELECT COUNT(*) FROM playlist_items"), 2);
        assert_integrity(&connection);
    }

    #[test]
    fn migrations_apply_in_order_and_a_second_run_does_not_repeat_them() {
        let scratch = Scratch::new();
        let path = scratch.database();
        let probe = [
            product()[0],
            Migration {
                version: 2,
                name: "probe",
                sql: PROBE_SQL,
            },
        ];
        let (connection, outcome) = open_and_migrate(&path, &probe).unwrap();
        assert_eq!(outcome, MigrationOutcome::Created);
        connection
            .execute(
                "INSERT INTO app_settings (key, value) VALUES ('volume', '40')",
                [],
            )
            .unwrap();
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM schema_migrations"),
            2
        );
        let names: Vec<String> = connection
            .prepare("SELECT name FROM schema_migrations ORDER BY version")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(names, vec!["initial", "probe"]);
        drop(connection);

        let (connection, outcome) = open_and_migrate(&path, &probe).unwrap();
        assert_eq!(outcome, MigrationOutcome::AlreadyCurrent);
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM schema_migrations"),
            2
        );
        let volume: String = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'volume'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(volume, "40");
        assert_eq!(product().len(), 1);
        assert!(!product()[0].sql.contains("migration_probe"));
        assert_integrity(&connection);
    }

    #[test]
    fn a_failed_migration_keeps_the_existing_database() {
        let scratch = Scratch::new();
        let path = scratch.database();
        let (connection, _) = open_and_migrate(&path, product()).unwrap();
        connection
            .execute(
                "INSERT INTO app_settings (key, value) VALUES ('volume', '40')",
                [],
            )
            .unwrap();
        drop(connection);

        let broken = [
            product()[0],
            Migration {
                version: 2,
                name: "broken",
                sql: BROKEN_SQL,
            },
        ];
        let error = open_and_migrate(&path, &broken).unwrap_err();
        assert!(matches!(error, DatabaseError::Migration { version: 2, .. }));
        assert!(path.is_file());

        let (connection, outcome) = open_and_migrate(&path, product()).unwrap();
        assert_eq!(outcome, MigrationOutcome::AlreadyCurrent);
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM schema_migrations"),
            1
        );
        let volume: String = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'volume'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(volume, "40");
        let names = table_names(&path, &connection).unwrap();
        assert!(!names.iter().any(|name| name == "migration_probe"));
        assert_integrity(&connection);
    }

    #[test]
    fn a_newer_migration_stops_without_changing_the_database() {
        let scratch = Scratch::new();
        let path = scratch.database();
        let (connection, _) = open_and_migrate(&path, product()).unwrap();
        connection
            .execute(
                "INSERT INTO app_settings (key, value) VALUES ('volume', '40')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, name, applied_at)
                 VALUES (99, 'future', '2026-11-09T18:00:00Z')",
                [],
            )
            .unwrap();
        drop(connection);

        let error = open_and_migrate(&path, product()).unwrap_err();
        assert!(matches!(
            error,
            DatabaseError::NewerSchema {
                found: 99,
                known: 1,
                ..
            }
        ));
        assert!(path.is_file());

        let connection =
            Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let volume: String = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'volume'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(volume, "40");
        let versions: Vec<i64> = connection
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(versions, vec![1, 99]);
    }

    #[test]
    fn data_persists_after_closing_and_opening() {
        let scratch = Scratch::new();
        let path = scratch.database();
        {
            let (connection, outcome) = open_and_migrate(&path, product()).unwrap();
            assert_eq!(outcome, MigrationOutcome::Created);
            let file = insert_file(&connection, "persistente", 8);
            let item = insert_item(&connection, "Persiste");
            insert_link(&connection, item, file, "primary");
            assert_integrity(&connection);
        }
        let (connection, outcome) = open_and_migrate(&path, product()).unwrap();
        assert_eq!(outcome, MigrationOutcome::AlreadyCurrent);
        let title: String = connection
            .query_row("SELECT title FROM media_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(title, "Persiste");
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM schema_migrations"),
            1
        );
        assert_integrity(&connection);
    }

    #[test]
    fn sql_never_deletes_the_physical_media_file() {
        let scratch = Scratch::new();
        let media = scratch.path.join("vídeo.mp4");
        fs::write(&media, b"video").unwrap();
        let (connection, _) = open_and_migrate(&scratch.database(), product()).unwrap();
        let file = insert_file(&connection, "video", 5);
        let item = insert_item(&connection, "Vídeo");
        insert_link(&connection, item, file, "primary");
        connection
            .execute("DELETE FROM media_items WHERE id = ?1", [item])
            .unwrap();
        connection
            .execute("DELETE FROM stored_files WHERE id = ?1", [file])
            .unwrap();
        assert_eq!(fs::read(&media).unwrap(), b"video");
        assert!(media.is_file());
    }

    #[test]
    fn development_database_path_is_not_the_production_database() {
        let local = PathBuf::from(r"C:\Users\example\AppData\Local");
        let development = crate::storage::InternalPaths::from_app_local_data(
            crate::storage::app_local_data(&local, crate::storage::DEVELOPMENT_IDENTIFIER).unwrap(),
        );
        let production = crate::storage::InternalPaths::from_app_local_data(
            crate::storage::app_local_data(&local, crate::storage::PRODUCTION_IDENTIFIER).unwrap(),
        );
        assert!(development.database.ends_with("library.db"));
        assert!(production.database.ends_with("library.db"));
        assert_ne!(development.database, production.database);
        assert!(!development.database.starts_with(&production.database));
        assert!(!production.database.starts_with(&development.database));
        assert!(
            !development.database.exists(),
            "la prueba no crea la base de desarrollo real"
        );
        assert!(
            !production.database.exists(),
            "la prueba no crea la base de producción"
        );
    }

    #[test]
    fn an_unrecognized_file_is_left_untouched() {
        let scratch = Scratch::new();
        let path = scratch.database();
        let original = b"esto no es sqlite".to_vec();
        fs::write(&path, &original).unwrap();
        let error = open_and_migrate(&path, product()).unwrap_err();
        assert!(matches!(error, DatabaseError::Unexpected { .. }));
        assert_eq!(fs::read(&path).unwrap(), original);

        let foreign = scratch.path.join("ajena.db");
        {
            let connection = Connection::open(&foreign).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE ajena (id INTEGER PRIMARY KEY); INSERT INTO ajena (id) VALUES (7);",
                )
                .unwrap();
        }
        let error = open_and_migrate(&foreign, product()).unwrap_err();
        assert!(matches!(error, DatabaseError::Unexpected { .. }));
        let connection =
            Connection::open_with_flags(&foreign, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let id: i64 = connection
            .query_row("SELECT id FROM ajena", [], |row| row.get(0))
            .unwrap();
        assert_eq!(id, 7);

        let directory = scratch.path.join("directorio.db");
        fs::create_dir(&directory).unwrap();
        assert!(open_and_migrate(&directory, product()).is_err());
        assert!(directory.is_dir());

        let missing_parent = scratch.path.join("ausente").join("library.db");
        assert!(open_and_migrate(&missing_parent, product()).is_err());
        assert!(!missing_parent.exists());
    }

    #[test]
    fn a_gap_in_the_known_migrations_does_not_create_a_database() {
        let scratch = Scratch::new();
        let path = scratch.database();
        let skipped = [
            product()[0],
            Migration {
                version: 3,
                name: "skipped",
                sql: "SELECT 1;",
            },
        ];
        assert!(open_and_migrate(&path, &skipped).is_err());
        assert!(!path.exists());
    }
}
