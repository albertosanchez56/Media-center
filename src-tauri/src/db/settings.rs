//! Volumen guardado en `app_settings`. No escribe `storage.json` ni se lee al arrancar.

use super::{Database, DatabaseError};
use std::path::Path;

const VOLUME_KEY: &str = "volume";
const DEFAULT_VOLUME: i64 = 100;
const MIN_VOLUME: i64 = 0;
const MAX_VOLUME: i64 = 100;

impl Database {
    /// El reproductor leerá este valor cuando aplique el audio. F2.3 no lo hace al arrancar.
    #[allow(dead_code)]
    pub fn volume(&self) -> Result<i64, DatabaseError> {
        let connection = self
            .connection
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match connection.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [VOLUME_KEY],
            |row| row.get::<_, String>(0),
        ) {
            Ok(value) => Ok(accepted_volume(&self.path, &value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(DEFAULT_VOLUME),
            Err(error) => Err(DatabaseError::Unexpected {
                path: self.path.clone(),
                message: format!("No se ha podido leer el ajuste de volumen: {error}"),
            }),
        }
    }

    /// El reproductor guardará el volumen cuando la persona lo cambie.
    #[allow(dead_code)]
    pub fn set_volume(&self, volume: i64) -> Result<(), DatabaseError> {
        if !(MIN_VOLUME..=MAX_VOLUME).contains(&volume) {
            return Err(DatabaseError::Unexpected {
                path: self.path.clone(),
                message: "El volumen tiene que ser un entero entre 0 y 100.".into(),
            });
        }
        let connection = self
            .connection
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        connection
            .execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![VOLUME_KEY, volume.to_string()],
            )
            .map_err(|error| DatabaseError::Unexpected {
                path: self.path.clone(),
                message: format!("No se ha podido guardar el ajuste de volumen: {error}"),
            })?;
        Ok(())
    }
}

fn accepted_volume(path: &Path, value: &str) -> i64 {
    match value.parse::<i64>() {
        Ok(volume) if (MIN_VOLUME..=MAX_VOLUME).contains(&volume) => volume,
        _ => {
            eprintln!(
                "El ajuste volume en {} no es válido ({value:?}) y no se modifica. Se usa {DEFAULT_VOLUME}.",
                path.display()
            );
            DEFAULT_VOLUME
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::initialize;
    use std::fs;
    use std::path::PathBuf;
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
                .join(format!("mc-f23-{}-{nanos}-{unique}", std::process::id()));
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

    fn stored_volume(database: &Database) -> Option<String> {
        let connection = database
            .connection
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match connection.query_row(
            "SELECT value FROM app_settings WHERE key = 'volume'",
            [],
            |row| row.get(0),
        ) {
            Ok(value) => Some(value),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(error) => panic!("no se ha podido leer la fila de volumen: {error}"),
        }
    }

    #[test]
    fn a_missing_volume_uses_the_default_without_writing_it() {
        let scratch = Scratch::new();
        let database = initialize(&scratch.database()).unwrap();
        assert_eq!(database.volume().unwrap(), DEFAULT_VOLUME);
        assert_eq!(stored_volume(&database), None);
        assert!(!scratch.path.join("storage.json").exists());
    }

    #[test]
    fn a_saved_volume_is_still_there_after_closing_and_opening() {
        let scratch = Scratch::new();
        let path = scratch.database();
        {
            let database = initialize(&path).unwrap();
            database.set_volume(40).unwrap();
        }
        let database = initialize(&path).unwrap();
        assert_eq!(database.volume().unwrap(), 40);
        assert_eq!(stored_volume(&database).as_deref(), Some("40"));
        assert!(!scratch.path.join("storage.json").exists());
    }

    #[test]
    fn the_edges_are_saved_and_a_value_outside_the_range_is_rejected() {
        let scratch = Scratch::new();
        let database = initialize(&scratch.database()).unwrap();
        database.set_volume(40).unwrap();
        assert!(database.set_volume(101).is_err());
        assert!(database.set_volume(-1).is_err());
        assert_eq!(database.volume().unwrap(), 40);
        database.set_volume(0).unwrap();
        assert_eq!(database.volume().unwrap(), 0);
        database.set_volume(100).unwrap();
        assert_eq!(database.volume().unwrap(), 100);
        assert_eq!(stored_volume(&database).as_deref(), Some("100"));
    }

    #[test]
    fn an_invalid_stored_volume_uses_the_default_and_keeps_the_row() {
        let scratch = Scratch::new();
        let database = initialize(&scratch.database()).unwrap();
        {
            let connection = database
                .connection
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            connection
                .execute(
                    "INSERT INTO app_settings (key, value) VALUES ('volume', 'alto')",
                    [],
                )
                .unwrap();
        }
        assert_eq!(database.volume().unwrap(), DEFAULT_VOLUME);
        assert_eq!(stored_volume(&database).as_deref(), Some("alto"));
        assert!(!scratch.path.join("storage.json").exists());
    }

    #[test]
    fn a_sqlite_failure_is_not_replaced_by_the_default() {
        let scratch = Scratch::new();
        let database = initialize(&scratch.database()).unwrap();
        database.set_volume(25).unwrap();
        {
            let connection = database
                .connection
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            connection
                .execute("ALTER TABLE app_settings RENAME TO app_settings_away", [])
                .unwrap();
        }
        let read_error = database.volume().unwrap_err();
        assert!(
            read_error.to_string().contains("No se ha podido leer"),
            "{read_error}"
        );
        let write_error = database.set_volume(30).unwrap_err();
        assert!(
            write_error.to_string().contains("No se ha podido guardar"),
            "{write_error}"
        );
        {
            let connection = database
                .connection
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            connection
                .execute("ALTER TABLE app_settings_away RENAME TO app_settings", [])
                .unwrap();
        }
        assert_eq!(stored_volume(&database).as_deref(), Some("25"));
        assert_eq!(database.volume().unwrap(), 25);
    }
}
