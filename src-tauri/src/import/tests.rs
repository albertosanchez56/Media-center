use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::{self, Database};
use crate::storage::{self, Confirmation, ManagedFolderConfig};

use super::{ImportHooks, Mode, Pause, import_paths, recover};

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "mc-f32-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn child(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct Env {
    db: Database,
    managed: PathBuf,
    app_data: PathBuf,
    scratch: Scratch,
}

impl Env {
    fn new() -> Self {
        let scratch = Scratch::new();
        let managed = scratch.child("biblioteca");
        let app_data = scratch.child("appdata");
        fs::create_dir_all(&managed).unwrap();
        fs::create_dir_all(&app_data).unwrap();
        let db = db::initialize(&scratch.child("library.db")).unwrap();
        Self {
            db,
            managed,
            app_data,
            scratch,
        }
    }

    fn source(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.scratch.child(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, bytes).unwrap();
        path
    }
}

fn run(
    env: &Env,
    library: &str,
    mode: Mode,
    paths: &[PathBuf],
    folder: bool,
    cancel: &AtomicBool,
    hooks: &mut ImportHooks,
) -> Vec<super::ItemReport> {
    import_paths(
        &env.db,
        &env.managed,
        &env.app_data,
        library,
        mode,
        paths,
        folder,
        cancel,
        hooks,
        &mut |_| {},
    )
}

fn run_simple(env: &Env, mode: Mode, path: &Path) -> Vec<super::ItemReport> {
    let cancel = AtomicBool::new(false);
    run(
        env,
        "music",
        mode,
        &[path.to_path_buf()],
        false,
        &cancel,
        &mut ImportHooks::default(),
    )
}

fn stages(env: &Env) -> Vec<(String, String, String, String)> {
    env.db
        .with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT source_path, stage, method, kind
                 FROM file_operations ORDER BY id",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            rows.collect()
        })
        .unwrap()
}

fn media_count(env: &Env) -> i64 {
    env.db
        .with_connection(|connection| {
            connection.query_row("SELECT COUNT(*) FROM media_items", [], |row| row.get(0))
        })
        .unwrap()
}

fn libraries(env: &Env) -> Vec<(String, String)> {
    env.db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT library, title FROM media_items ORDER BY id")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect()
        })
        .unwrap()
}

fn hashes(env: &Env) -> Vec<Option<String>> {
    env.db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT content_hash FROM stored_files ORDER BY id")?;
            let rows = statement.query_map([], |row| row.get(0))?;
            rows.collect()
        })
        .unwrap()
}

fn partials(managed: &Path) -> Vec<PathBuf> {
    let incoming = storage::incoming_dir(managed);
    if !incoming.is_dir() {
        return Vec::new();
    }
    fs::read_dir(incoming)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect()
}

#[test]
fn copy_keeps_the_original_and_creates_one_card() {
    let env = Env::new();
    let source = env.source("clip.mp4", b"audio-mp4");
    let reports = run_simple(&env, Mode::Copy, &source);
    assert_eq!(reports[0].state, "completed");
    assert_eq!(fs::read(&source).unwrap(), b"audio-mp4");
    assert_eq!(
        fs::read(env.managed.join("clip.mp4")).unwrap(),
        b"audio-mp4"
    );
    assert_eq!(
        stages(&env),
        vec![(
            source.display().to_string(),
            "completed".into(),
            "copy".into(),
            "copy".into(),
        )]
    );
    assert_eq!(media_count(&env), 1);
    assert_eq!(libraries(&env), vec![("music".into(), "clip".into())]);
    assert_eq!(hashes(&env), vec![None]);
    assert!(partials(&env.managed).is_empty());
}

#[test]
fn move_uses_copy_then_deletes_the_verified_original() {
    let env = Env::new();
    let source = env.source("pieza.mp4", b"video-real");
    let reports = run_simple(&env, Mode::Move, &source);
    assert_eq!(reports[0].state, "completed");
    assert!(!source.exists());
    assert_eq!(
        fs::read(env.managed.join("pieza.mp4")).unwrap(),
        b"video-real"
    );
    let rows = stages(&env);
    assert_eq!(rows[0].1, "completed");
    assert_eq!(rows[0].2, "copy");
    assert_eq!(rows[0].3, "move");
    let hash = hashes(&env)[0].clone().unwrap();
    assert!(hash.starts_with("sha256:"));
    assert_ne!(
        hash,
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn an_existing_destination_is_not_overwritten() {
    let env = Env::new();
    fs::write(env.managed.join("clip.mp4"), b"ocupado").unwrap();
    let source = env.source("clip.mp4", b"nuevo");
    let reports = run_simple(&env, Mode::Copy, &source);
    assert_eq!(reports[0].state, "completed");
    assert_eq!(fs::read(env.managed.join("clip.mp4")).unwrap(), b"ocupado");
    assert_eq!(
        fs::read(env.managed.join("clip (2).mp4")).unwrap(),
        b"nuevo"
    );
}

#[test]
fn a_destination_created_during_publish_is_not_overwritten() {
    let env = Env::new();
    let source = env.source("clip.mp4", b"nuevo");
    let managed = env.managed.clone();
    let mut hooks = ImportHooks {
        before_publish: Some(Box::new(move |destination| {
            fs::write(destination, b"aparecio-despues").unwrap();
            assert_eq!(destination, managed.join("clip.mp4"));
        })),
        ..ImportHooks::default()
    };
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "music",
        Mode::Copy,
        &[source],
        false,
        &cancel,
        &mut hooks,
    );
    assert_eq!(reports[0].state, "completed", "{}", reports[0].message);
    assert_eq!(
        fs::read(env.managed.join("clip.mp4")).unwrap(),
        b"aparecio-despues"
    );
    assert_eq!(
        fs::read(env.managed.join("clip (2).mp4")).unwrap(),
        b"nuevo"
    );
}

#[test]
fn spaces_and_unicode_names_are_preserved() {
    let env = Env::new();
    let spaced = env.source("mi archivo.mp4", b"espacios");
    let unicode = env.source("canción ñ.mp4", b"tilde");
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "video",
        Mode::Copy,
        &[spaced, unicode],
        false,
        &cancel,
        &mut ImportHooks::default(),
    );
    assert!(reports.iter().all(|item| item.state == "completed"));
    assert_eq!(
        fs::read(env.managed.join("mi archivo.mp4")).unwrap(),
        b"espacios"
    );
    assert_eq!(
        fs::read(env.managed.join("canción ñ.mp4")).unwrap(),
        b"tilde"
    );
    assert_eq!(
        libraries(&env),
        vec![
            ("video".into(), "mi archivo".into()),
            ("video".into(), "canción ñ".into()),
        ]
    );
}

#[test]
fn the_same_mp4_name_can_belong_to_any_library() {
    let env = Env::new();
    for library in ["music", "video", "movie"] {
        let source = env.source(&format!("{library}-clip.mp4"), library.as_bytes());
        let cancel = AtomicBool::new(false);
        let reports = run(
            &env,
            library,
            Mode::Copy,
            &[source.clone()],
            false,
            &cancel,
            &mut ImportHooks::default(),
        );
        assert_eq!(reports[0].state, "completed");
        assert_eq!(fs::read(&source).unwrap(), library.as_bytes());
    }
    let names: Vec<String> = libraries(&env)
        .into_iter()
        .map(|(library, _)| library)
        .collect();
    assert_eq!(names, vec!["music", "video", "movie"]);
}

#[test]
fn a_changed_original_is_not_deleted_when_the_hash_differs() {
    let env = Env::new();
    let source = env.source("original.bin", b"aaaa");
    let destination = env.managed.join("destino.bin");
    fs::write(&destination, b"bbbb").unwrap();
    let opened = super::win::open_source(&source).unwrap();
    let facts = opened.facts.clone();
    let error = super::verify_and_delete(
        &opened,
        facts.size,
        &facts.modified,
        Some((facts.file_id, facts.volume_serial)),
        &destination,
    );
    assert!(error.is_err(), "{error:?}");
    assert_eq!(fs::read(&source).unwrap(), b"aaaa");
    assert_eq!(fs::read(&destination).unwrap(), b"bbbb");
}

#[test]
fn replacing_the_original_before_delete_keeps_the_replacement() {
    let env = Env::new();
    let source = env.source("original.bin", b"original-bytes");
    let opened = super::win::open_source(&source).unwrap();
    let expected_size = opened.facts.size;
    let expected_modified = opened.facts.modified.clone();
    drop(opened);
    fs::write(&source, b"reemplazo-distinto").unwrap();
    let reopened = super::win::open_source(&source).unwrap();
    let destination = env.managed.join("destino.bin");
    fs::write(&destination, b"original-bytes").unwrap();
    let error = super::verify_and_delete(
        &reopened,
        expected_size,
        &expected_modified,
        None,
        &destination,
    );
    assert!(error.is_err(), "{error:?}");
    assert_eq!(fs::read(&source).unwrap(), b"reemplazo-distinto");
}

#[test]
fn an_interrupted_copy_keeps_the_original_and_removes_the_temp() {
    let env = Env::new();
    let source = env.source("largo.bin", b"0123456789abcdef");
    let mut hooks = ImportHooks {
        chunk_bytes: Some(4),
        interrupt_after_bytes: Some(4),
        ..ImportHooks::default()
    };
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "music",
        Mode::Move,
        &[source.clone()],
        false,
        &cancel,
        &mut hooks,
    );
    assert_eq!(reports[0].state, "failed");
    assert_eq!(fs::read(&source).unwrap(), b"0123456789abcdef");
    assert!(partials(&env.managed).is_empty());
    assert_eq!(media_count(&env), 0);
    assert_eq!(stages(&env)[0].1, "failed");
    recover(&env.db, &env.managed, &env.app_data.join("images")).unwrap();
    assert_eq!(fs::read(&source).unwrap(), b"0123456789abcdef");
    assert_eq!(media_count(&env), 0);
    assert_eq!(stages(&env)[0].1, "failed");
}

#[test]
fn sqlite_failure_after_publish_keeps_both_files_until_recovery() {
    let env = Env::new();
    let source = env.source("tema.bin", b"contenido");
    let mut hooks = ImportHooks {
        fail_commit: true,
        ..ImportHooks::default()
    };
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "movie",
        Mode::Copy,
        &[source.clone()],
        false,
        &cancel,
        &mut hooks,
    );
    assert_eq!(reports[0].state, "ready_for_db");
    assert_eq!(fs::read(&source).unwrap(), b"contenido");
    assert_eq!(
        fs::read(env.managed.join("tema.bin")).unwrap(),
        b"contenido"
    );
    assert_eq!(media_count(&env), 0);
    assert_eq!(stages(&env)[0].1, "ready_for_db");
    recover(&env.db, &env.managed, &env.app_data.join("images")).unwrap();
    assert_eq!(media_count(&env), 1);
    assert_eq!(fs::read(&source).unwrap(), b"contenido");
    assert_eq!(stages(&env)[0].1, "completed");
    recover(&env.db, &env.managed, &env.app_data.join("images")).unwrap();
    assert_eq!(media_count(&env), 1);
}

#[test]
fn a_move_whose_destination_changes_before_recovery_keeps_the_original() {
    let env = Env::new();
    let source = env.source("tema.bin", b"contenido");
    let mut hooks = ImportHooks {
        fail_commit: true,
        ..ImportHooks::default()
    };
    let cancel = AtomicBool::new(false);
    run(
        &env,
        "music",
        Mode::Move,
        &[source.clone()],
        false,
        &cancel,
        &mut hooks,
    )
    .into_iter()
    .for_each(|item| assert_eq!(item.state, "ready_for_db"));
    fs::write(env.managed.join("tema.bin"), b"corrupto!").unwrap();
    recover(&env.db, &env.managed, &env.app_data.join("images")).unwrap();
    assert_eq!(fs::read(&source).unwrap(), b"contenido");
    assert_eq!(stages(&env)[0].1, "needs_review");
    recover(&env.db, &env.managed, &env.app_data.join("images")).unwrap();
    assert_eq!(fs::read(&source).unwrap(), b"contenido");
    assert_eq!(stages(&env)[0].1, "needs_review");
}

#[test]
fn rename_recovery_does_not_trust_size_and_deletes_nothing() {
    let env = Env::new();
    let source = env.source("origen.bin", b"tamano-ok");
    let destination_name = "renombrado.bin";
    fs::write(env.managed.join(destination_name), b"tamano-ok").unwrap();
    fs::remove_file(&source).unwrap();
    let id = super::journal::insert_planned(
        &env.db,
        "move",
        "rename",
        "music",
        &source.display().to_string(),
        b"tamano-ok".len() as i64,
        "2020-01-01T00:00:00.0000000Z",
        None,
        destination_name,
    )
    .unwrap();
    super::journal::set_stage(&env.db, id, "ready_for_db", None).unwrap();
    recover(&env.db, &env.managed, &env.app_data.join("images")).unwrap();
    assert_eq!(
        fs::read(env.managed.join(destination_name)).unwrap(),
        b"tamano-ok"
    );
    assert!(!source.exists());
    assert_eq!(media_count(&env), 0);
    assert_eq!(stages(&env)[0].1, "needs_review");
    assert!(stages(&env)[0].2 == "rename");
    recover(&env.db, &env.managed, &env.app_data.join("images")).unwrap();
    assert_eq!(media_count(&env), 0);
    assert_eq!(
        fs::read(env.managed.join(destination_name)).unwrap(),
        b"tamano-ok"
    );
}

#[test]
fn insufficient_space_does_not_start_or_leave_a_temp() {
    let env = Env::new();
    let source = env.source("grande.bin", b"no-cabe");
    let mut hooks = ImportHooks {
        free_bytes: Some(0),
        ..ImportHooks::default()
    };
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "music",
        Mode::Move,
        &[source.clone()],
        false,
        &cancel,
        &mut hooks,
    );
    assert_eq!(reports[0].state, "failed");
    assert!(reports[0].message.contains("espacio"));
    assert_eq!(fs::read(&source).unwrap(), b"no-cabe");
    assert!(stages(&env).is_empty());
    assert!(partials(&env.managed).is_empty());
}

#[test]
fn cancel_during_copy_does_not_import_the_remaining_files() {
    let scratch = Scratch::new();
    let managed = scratch.child("biblioteca");
    let app_data = scratch.child("appdata");
    fs::create_dir_all(&managed).unwrap();
    fs::create_dir_all(&app_data).unwrap();
    let first = scratch.child("uno.bin");
    let second = scratch.child("dos.bin");
    fs::write(&first, b"0123456789abcdef").unwrap();
    fs::write(&second, b"segundo").unwrap();
    let db = Arc::new(db::initialize(&scratch.child("library.db")).unwrap());
    let pause = Arc::new(Pause {
        started: Barrier::new(2),
        release: Barrier::new(2),
    });
    let cancel = Arc::new(AtomicBool::new(false));
    let pause_worker = Arc::clone(&pause);
    let cancel_worker = Arc::clone(&cancel);
    let db_worker = Arc::clone(&db);
    let managed_worker = managed.clone();
    let app_worker = app_data.clone();
    let paths = vec![first.clone(), second.clone()];
    let worker = std::thread::spawn(move || {
        let mut hooks = ImportHooks {
            chunk_bytes: Some(4),
            pause: Some(pause_worker),
            ..ImportHooks::default()
        };
        import_paths(
            &db_worker,
            &managed_worker,
            &app_worker,
            "music",
            Mode::Copy,
            &paths,
            false,
            &cancel_worker,
            &mut hooks,
            &mut |_| {},
        )
    });
    pause.started.wait();
    cancel.store(true, Ordering::SeqCst);
    pause.release.wait();
    let reports = worker.join().unwrap();
    assert_eq!(reports[0].state, "failed");
    assert_eq!(reports[1].state, "failed");
    assert!(reports[1].message.to_lowercase().contains("cancelada"));
    assert_eq!(fs::read(&first).unwrap(), b"0123456789abcdef");
    assert_eq!(fs::read(&second).unwrap(), b"segundo");
    assert!(partials(&managed).is_empty());
    let rows = db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT source_path, stage FROM file_operations ORDER BY id")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].0.contains("uno.bin"));
    assert_eq!(rows[0].1, "failed");
}

#[test]
fn the_database_stays_readable_during_the_copy() {
    let scratch = Scratch::new();
    let managed = scratch.child("biblioteca");
    let app_data = scratch.child("appdata");
    fs::create_dir_all(&managed).unwrap();
    fs::create_dir_all(&app_data).unwrap();
    let source = scratch.child("lento.bin");
    fs::write(&source, b"0123456789abcdef").unwrap();
    let db = Arc::new(db::initialize(&scratch.child("library.db")).unwrap());
    let pause = Arc::new(Pause {
        started: Barrier::new(2),
        release: Barrier::new(2),
    });
    let pause_worker = Arc::clone(&pause);
    let db_worker = Arc::clone(&db);
    let managed_worker = managed.clone();
    let app_worker = app_data.clone();
    let worker = std::thread::spawn(move || {
        let mut hooks = ImportHooks {
            chunk_bytes: Some(4),
            pause: Some(pause_worker),
            ..ImportHooks::default()
        };
        import_paths(
            &db_worker,
            &managed_worker,
            &app_worker,
            "music",
            Mode::Copy,
            &[source],
            false,
            &AtomicBool::new(false),
            &mut hooks,
            &mut |_| {},
        )
    });
    pause.started.wait();
    let stage = db
        .try_with_connection(|connection| {
            connection.query_row("SELECT stage FROM file_operations", [], |row| {
                row.get::<_, String>(0)
            })
        })
        .unwrap();
    assert_eq!(stage.as_deref(), Some("transferring"));
    pause.release.wait();
    let reports = worker.join().unwrap();
    assert_eq!(reports[0].state, "completed");
}

#[test]
fn a_folder_imports_only_its_first_level_and_skips_subfolders() {
    let env = Env::new();
    let folder = env.scratch.child("carpeta");
    fs::create_dir_all(folder.join("anidada")).unwrap();
    fs::write(folder.join("visible.mp4"), b"visible").unwrap();
    fs::write(folder.join("anidada").join("oculto.mp4"), b"oculto").unwrap();
    let outside = env.source("fuera.mp4", b"fuera");
    let link = folder.join("enlace.mp4");
    let link_created = std::os::windows::fs::symlink_file(&outside, &link);
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "music",
        Mode::Copy,
        &[folder],
        true,
        &cancel,
        &mut ImportHooks::default(),
    );
    assert!(
        reports
            .iter()
            .any(|item| item.state == "completed" && item.source.ends_with("visible.mp4"))
    );
    assert!(
        reports
            .iter()
            .any(|item| item.message.contains("subcarpetas"))
    );
    assert!(!env.managed.join("oculto.mp4").exists());
    assert_eq!(
        fs::read(env.managed.join("visible.mp4")).unwrap(),
        b"visible"
    );
    if link_created.is_ok() {
        assert!(
            reports
                .iter()
                .any(|item| item.source.ends_with("enlace.mp4") && item.state == "skipped")
        );
        assert!(!env.managed.join("enlace.mp4").exists());
        assert_eq!(fs::read(&outside).unwrap(), b"fuera");
    }
}

#[test]
fn an_empty_folder_creates_no_operation() {
    let env = Env::new();
    let folder = env.scratch.child("vacia");
    fs::create_dir_all(&folder).unwrap();
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "music",
        Mode::Copy,
        &[folder],
        true,
        &cancel,
        &mut ImportHooks::default(),
    );
    assert!(reports[0].message.contains("no contiene"));
    assert!(stages(&env).is_empty());
}

#[test]
fn other_files_are_not_deleted() {
    let env = Env::new();
    let source = env.source("pieza.bin", b"mover");
    let neighbor = env.scratch.child("vecino.bin");
    fs::write(&neighbor, b"vecino").unwrap();
    fs::write(env.managed.join("ya-estaba.bin"), b"biblioteca").unwrap();
    let reports = run_simple(&env, Mode::Move, &source);
    assert_eq!(reports[0].state, "completed");
    assert_eq!(fs::read(&neighbor).unwrap(), b"vecino");
    assert_eq!(
        fs::read(env.managed.join("ya-estaba.bin")).unwrap(),
        b"biblioteca"
    );
    assert!(!source.exists());
}

#[test]
fn importing_the_same_source_again_does_not_copy_it() {
    let env = Env::new();
    let source = env.source("unico.bin", b"una-vez");
    assert_eq!(run_simple(&env, Mode::Copy, &source)[0].state, "completed");
    let again = run_simple(&env, Mode::Copy, &source);
    assert_eq!(again[0].state, "skipped");
    assert_eq!(media_count(&env), 1);
    assert_eq!(fs::read(&source).unwrap(), b"una-vez");
}

#[test]
fn confirming_the_managed_folder_uses_only_the_given_storage_file() {
    let scratch = Scratch::new();
    let storage_file = scratch.child("storage.json");
    let managed = scratch.child("biblioteca");
    storage::save_storage(
        &storage_file,
        &ManagedFolderConfig {
            path: managed.clone(),
            confirmation: Confirmation::Proposed,
        },
    )
    .unwrap();
    let confirmed = super::ensure_managed_folder(&storage_file, &[]).unwrap();
    assert_eq!(confirmed, managed);
    assert!(managed.is_dir());
    let loaded = storage::load_storage(&storage_file).unwrap();
    assert_eq!(loaded.confirmation, Confirmation::Confirmed);
    let _ = SystemTime::now().duration_since(UNIX_EPOCH);
}

fn images_of(env: &Env) -> std::path::PathBuf {
    env.app_data.join("images")
}

fn image_rows(env: &Env) -> Vec<(String, i64, String)> {
    env.db
        .with_connection(|connection| {
            let mut statement = connection
                .prepare("SELECT source, selected, relative_path FROM media_images ORDER BY id")?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            rows.collect()
        })
        .unwrap()
}

fn card_text(
    env: &Env,
) -> (
    String,
    String,
    String,
    String,
    Option<i64>,
    String,
    Option<i64>,
    String,
) {
    env.db
        .with_connection(|connection| {
            connection.query_row(
                "SELECT title, artist, album, genre, year, description, duration_ms, imported_snapshot
                 FROM media_items ORDER BY id LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
        })
        .unwrap()
}

#[test]
fn an_mp3_keeps_the_original_and_stores_the_real_cover() {
    let env = Env::new();
    let source = env.scratch.child("cancion.mp3");
    crate::tags::fixtures::write_mp3(&source);
    let before = fs::metadata(&source).unwrap();
    let reports = run_simple(&env, Mode::Copy, &source);
    assert_eq!(reports[0].state, "completed");
    assert_eq!(
        fs::read(&source).unwrap(),
        fs::read(env.managed.join("cancion.mp3")).unwrap()
    );
    assert_eq!(
        fs::metadata(&source).unwrap().modified().unwrap(),
        before.modified().unwrap()
    );
    let (title, artist, album, genre, year, description, duration, snapshot) = card_text(&env);
    assert_eq!(title, "Cancion");
    assert_eq!(artist, "Artista");
    assert_eq!(album, "Disco");
    assert_eq!(genre, "Rock");
    assert_eq!(year, Some(1999));
    assert_eq!(description, "Una nota");
    assert!(duration.is_some());
    assert!(snapshot.contains("\"title\":\"Cancion\""));
    assert_eq!(
        image_rows(&env),
        vec![("embedded".into(), 1, "embedded/1.jpg".into())]
    );
    assert_eq!(
        fs::read(images_of(&env).join("embedded").join("1.jpg")).unwrap(),
        crate::tags::fixtures::JPEG
    );
    recover(&env.db, &env.managed, &images_of(&env)).unwrap();
    assert_eq!(media_count(&env), 1);
    assert_eq!(image_rows(&env).len(), 1);
}

#[test]
fn flac_and_m4a_fill_the_chosen_library() {
    let env = Env::new();
    let flac = env.scratch.child("tema.flac");
    crate::tags::fixtures::write_flac(&flac);
    let before = fs::read(&flac).unwrap();
    assert_eq!(run_simple(&env, Mode::Copy, &flac)[0].state, "completed");
    assert_eq!(fs::read(&flac).unwrap(), before);
    assert_eq!(card_text(&env).0, "Cancion");
    assert_eq!(image_rows(&env).len(), 1);

    let m4a = env.scratch.child("video.m4a");
    crate::tags::fixtures::write_mp4(&m4a);
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "movie",
        Mode::Copy,
        &[m4a.clone()],
        false,
        &cancel,
        &mut ImportHooks::default(),
    );
    assert_eq!(reports[0].state, "completed");
    assert_eq!(
        fs::read(&m4a).unwrap(),
        fs::read(env.managed.join("video.m4a")).unwrap()
    );
    assert_eq!(libraries(&env)[1], ("movie".into(), "Cancion".into()));
    assert_eq!(image_rows(&env).len(), 2);
}

#[test]
fn mkv_and_webm_still_import_without_tags() {
    let env = Env::new();
    let mkv = env.source("peli.mkv", b"\x1a\x45\xdf\xa3no-es-matroska");
    let webm = env.source("clip.webm", b"\x1a\x45\xdf\xa3tampoco");
    let cancel = AtomicBool::new(false);
    let reports = run(
        &env,
        "video",
        Mode::Copy,
        &[mkv.clone(), webm.clone()],
        false,
        &cancel,
        &mut ImportHooks::default(),
    );
    assert!(reports.iter().all(|report| report.state == "completed"));
    assert_eq!(
        libraries(&env),
        vec![
            ("video".into(), "peli".into()),
            ("video".into(), "clip".into()),
        ]
    );
    assert!(image_rows(&env).is_empty());
    assert_eq!(fs::read(&mkv).unwrap(), b"\x1a\x45\xdf\xa3no-es-matroska");
    assert_eq!(fs::read(&webm).unwrap(), b"\x1a\x45\xdf\xa3tampoco");
}

#[test]
fn a_webp_cover_is_not_saved_and_an_oversized_one_is_dropped() {
    let env = Env::new();
    let webp = env.scratch.child("webp.mp3");
    crate::tags::fixtures::write_mp3_with_webp_and_title(&webp);
    assert_eq!(run_simple(&env, Mode::Copy, &webp)[0].state, "completed");
    assert_eq!(card_text(&env).0, "Con webp");
    assert!(image_rows(&env).is_empty());

    let huge = env.scratch.child("grande.mp3");
    crate::tags::fixtures::write_mp3_with_oversized_cover(&huge);
    assert_eq!(run_simple(&env, Mode::Copy, &huge)[0].state, "completed");
    let texts = env
        .db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT title, artist FROM media_items ORDER BY id")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()
        })
        .unwrap();
    assert_eq!(texts[1], ("Grande".into(), "Artista".into()));
    assert!(image_rows(&env).is_empty());
}

#[test]
fn a_large_audio_file_does_not_become_the_cover() {
    let env = Env::new();
    let source = env.scratch.child("largo.mp3");
    crate::tags::fixtures::write_mp3(&source);
    let mut file = fs::OpenOptions::new().append(true).open(&source).unwrap();
    use std::io::Write;
    file.write_all(&vec![0u8; 2 * 1024 * 1024]).unwrap();
    drop(file);
    let length = fs::metadata(&source).unwrap().len();
    assert!(length > 2 * 1024 * 1024);
    assert_eq!(run_simple(&env, Mode::Copy, &source)[0].state, "completed");
    let cover = fs::read(images_of(&env).join("embedded").join("1.jpg")).unwrap();
    assert_eq!(cover, crate::tags::fixtures::JPEG);
    assert!(cover.len() < 1024);
    assert_eq!(fs::metadata(&source).unwrap().len(), length);
}

#[test]
fn recovery_does_not_duplicate_the_card_or_the_cover() {
    let env = Env::new();
    let source = env.scratch.child("cancion.mp3");
    crate::tags::fixtures::write_mp3(&source);
    assert_eq!(run_simple(&env, Mode::Copy, &source)[0].state, "completed");
    env.db
        .with_connection(|connection| {
            connection.execute(
                "UPDATE file_operations SET stage = 'ready_for_db' WHERE id = 1",
                [],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .unwrap();
    recover(&env.db, &env.managed, &images_of(&env)).unwrap();
    assert_eq!(media_count(&env), 1);
    assert_eq!(image_rows(&env).len(), 1);
    assert_eq!(stages(&env)[0].1, "completed");
}

#[test]
fn a_user_image_is_not_replaced_and_stays_selected() {
    let env = Env::new();
    let source = env.source("clip.bin", b"sin-etiquetas");
    assert_eq!(run_simple(&env, Mode::Copy, &source)[0].state, "completed");
    let user = images_of(&env).join("user");
    fs::create_dir_all(&user).unwrap();
    fs::write(user.join("1.png"), b"portada-personal").unwrap();
    env.db
        .with_connection(|connection| {
            connection.execute(
                "INSERT INTO media_images (
                    media_item_id, source, relative_path, selected, created_at, updated_at
                 ) VALUES (1, 'user', 'user/1.png', 1, '2026-11-11T00:00:00.000Z', '2026-11-11T00:00:00.000Z')",
                [],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .unwrap();
    let jpeg = crate::tags::fixtures::JPEG;
    let stored = super::covers::store(&env.db, &images_of(&env), 1, jpeg, "jpg")
        .unwrap()
        .unwrap();
    assert!(super::journal::insert_embedded(&env.db, 1, &stored.relative).unwrap());
    assert!(!super::journal::insert_embedded(&env.db, 1, &stored.relative).unwrap());
    assert_eq!(fs::read(user.join("1.png")).unwrap(), b"portada-personal");
    let rows = image_rows(&env);
    assert_eq!(
        rows.iter()
            .filter(|row| row.0 == "user" && row.1 == 1)
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.0 == "embedded" && row.1 == 0)
            .count(),
        1
    );
}

#[test]
fn protected_fields_remain_when_the_card_is_written_again() {
    let env = Env::new();
    let source = env.scratch.child("cancion.mp3");
    crate::tags::fixtures::write_mp3(&source);
    assert_eq!(run_simple(&env, Mode::Copy, &source)[0].state, "completed");
    env.db
        .with_connection(|connection| {
            connection.execute(
                "UPDATE media_items
                 SET title = 'Personal', user_edited_fields = '[\"title\"]'
                 WHERE id = 1",
                [],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .unwrap();
    let current = crate::tags::CardDraft {
        title: "Personal".into(),
        artist: card_text(&env).1,
        album: String::new(),
        genre: String::new(),
        year: None,
        description: String::new(),
        duration_ms: card_text(&env).6,
        snapshot: String::new(),
        cover: None,
        tags_read: true,
    };
    let mut incoming = crate::tags::read_card(&env.managed.join("cancion.mp3"), "music", "cancion");
    incoming.artist = "Otro".into();
    let merged = crate::tags::apply_protected(&current, "[\"title\"]", &incoming);
    super::journal::save_card_text(
        &env.db,
        1,
        &super::journal::NewCard {
            title: merged.title,
            artist: merged.artist,
            album: merged.album,
            genre: merged.genre,
            year: merged.year,
            description: merged.description,
            duration_ms: merged.duration_ms,
            snapshot: merged.snapshot,
            cover_relative: None,
        },
    )
    .unwrap();
    let (title, artist, _, _, _, _, _, snapshot) = card_text(&env);
    assert_eq!(title, "Personal");
    assert_eq!(artist, "Otro");
    assert!(snapshot.contains("\"title\":\"Cancion\""));
    assert!(!snapshot.contains("Personal"));
}

fn handlers(bytes: &[u8]) -> Vec<[u8; 4]> {
    let mut found = Vec::new();
    let mut index = 0;
    while index + 8 <= bytes.len() {
        let size = u32::from_be_bytes(bytes[index..index + 4].try_into().unwrap()) as usize;
        if size < 8 || index + size > bytes.len() {
            break;
        }
        let kind = &bytes[index + 4..index + 8];
        if kind == b"hdlr" && size >= 20 {
            found.push(bytes[index + 16..index + 20].try_into().unwrap());
        }
        let nested = matches!(
            kind,
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"dinf" | b"udta"
        );
        if nested {
            found.extend(handlers(&bytes[index + 8..index + size]));
        }
        index += size;
    }
    found
}

fn stored_files(env: &Env) -> Vec<(String, String)> {
    env.db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT path, container FROM stored_files ORDER BY id")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect()
        })
        .unwrap()
}

fn link_count(env: &Env) -> (i64, i64) {
    env.db
        .with_connection(|connection| {
            connection.query_row(
                "SELECT COUNT(*), COUNT(DISTINCT stored_file_id) FROM item_files",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
        })
        .unwrap()
}

fn preferences(env: &Env) -> Vec<Option<String>> {
    env.db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT visual_preference FROM media_items ORDER BY id")?;
            let rows = statement.query_map([], |row| row.get(0))?;
            rows.collect()
        })
        .unwrap()
}

#[test]
fn write_mp4_is_audio_only_and_the_video_fixtures_are_not_renames() {
    let env = Env::new();
    let audio_path = env.scratch.child("audio.m4a");
    let video_path = env.scratch.child("video.mp4");
    let silent_path = env.scratch.child("mudo.mp4");
    crate::tags::fixtures::write_mp4(&audio_path);
    crate::tags::fixtures::write_av_mp4(&video_path);
    crate::tags::fixtures::write_video_only_mp4(&silent_path);
    let audio = fs::read(&audio_path).unwrap();
    let video = fs::read(&video_path).unwrap();
    let silent = fs::read(&silent_path).unwrap();
    assert_eq!(&audio[4..12], b"ftypM4A ");
    assert_eq!(handlers(&audio), vec![*b"soun"]);
    assert_eq!(&video[4..12], b"ftypmp42");
    assert_eq!(handlers(&video), vec![*b"vide", *b"soun"]);
    assert!(
        video
            .windows(crate::tags::fixtures::JPEG.len())
            .any(|window| window == crate::tags::fixtures::JPEG)
    );
    assert_ne!(video, audio);
    assert_eq!(&silent[4..12], b"ftypmp42");
    assert_eq!(handlers(&silent), vec![*b"vide"]);
    assert_ne!(silent, video);
}

#[test]
fn three_independent_video_mp4_sources_keep_the_chosen_library() {
    let env = Env::new();
    let jobs = [
        ("music", "musica.mp4"),
        ("video", "clip.mp4"),
        ("movie", "pelicula.mp4"),
    ];
    let mut sources = Vec::new();
    for (_, name) in jobs {
        let path = env.scratch.child(name);
        crate::tags::fixtures::write_av_mp4(&path);
        sources.push(path);
    }
    let stamps: Vec<_> = sources
        .iter()
        .map(|path| fs::metadata(path).unwrap().modified().unwrap())
        .collect();
    for (index, (library, _)) in jobs.iter().enumerate() {
        let cancel = AtomicBool::new(false);
        let reports = run(
            &env,
            library,
            Mode::Copy,
            &[sources[index].clone()],
            false,
            &cancel,
            &mut ImportHooks::default(),
        );
        assert_eq!(reports[0].state, "completed");
        assert_eq!(
            fs::metadata(&sources[index]).unwrap().modified().unwrap(),
            stamps[index]
        );
        assert_eq!(
            fs::read(&sources[index]).unwrap(),
            fs::read(env.managed.join(sources[index].file_name().unwrap())).unwrap()
        );
    }
    assert_eq!(
        libraries(&env),
        vec![
            ("music".into(), "Cancion".into()),
            ("video".into(), "Cancion".into()),
            ("movie".into(), "Cancion".into()),
        ]
    );
    assert_eq!(media_count(&env), 3);
    assert_eq!(link_count(&env), (3, 3));
    assert_eq!(
        stored_files(&env),
        vec![
            ("musica.mp4".into(), "mp4".into()),
            ("clip.mp4".into(), "mp4".into()),
            ("pelicula.mp4".into(), "mp4".into()),
        ]
    );
    assert_eq!(preferences(&env), vec![None, None, None]);
    assert_eq!(image_rows(&env).len(), 3);
    let snapshots: Vec<String> = env
        .db
        .with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT imported_snapshot FROM media_items ORDER BY id")?;
            let rows = statement.query_map([], |row| row.get(0))?;
            rows.collect()
        })
        .unwrap();
    assert!(snapshots[0].contains("\"library\":\"music\""));
    assert!(snapshots[0].contains("\"title\":\"Cancion\""));
    assert!(snapshots[1].contains("\"library\":\"video\""));
    assert!(snapshots[2].contains("\"library\":\"movie\""));
}

#[test]
fn a_video_mp4_without_audio_keeps_a_provisional_card() {
    let env = Env::new();
    let source = env.scratch.child("mudo.mp4");
    crate::tags::fixtures::write_video_only_mp4(&source);
    let before = fs::metadata(&source).unwrap();
    let read = crate::tags::read_card(&source, "music", "mudo");
    assert!(!read.tags_read);
    assert_eq!(read.title, "mudo");
    let reports = run_simple(&env, Mode::Copy, &source);
    assert_eq!(reports[0].state, "completed");
    assert_eq!(
        fs::metadata(&source).unwrap().modified().unwrap(),
        before.modified().unwrap()
    );
    assert_eq!(
        fs::read(&source).unwrap(),
        fs::read(env.managed.join("mudo.mp4")).unwrap()
    );
    assert_eq!(libraries(&env), vec![("music".into(), "mudo".into())]);
    assert_eq!(media_count(&env), 1);
    assert_eq!(link_count(&env), (1, 1));
    assert_eq!(stored_files(&env), vec![("mudo.mp4".into(), "mp4".into())]);
    assert_eq!(preferences(&env), vec![None]);
    assert!(image_rows(&env).is_empty());
    let snapshot = card_text(&env).7;
    assert!(snapshot.contains("\"library\":\"music\""));
    assert!(snapshot.contains("\"title\":\"mudo\""));
}

#[test]
#[ignore]
fn write_f34_samples_for_manual() {
    let dir = std::env::temp_dir().join("mc-f34-manual");
    fs::create_dir_all(&dir).unwrap();
    crate::tags::fixtures::write_av_mp4(&dir.join("musica.mp4"));
    crate::tags::fixtures::write_av_mp4(&dir.join("clip.mp4"));
    crate::tags::fixtures::write_av_mp4(&dir.join("pelicula.mp4"));
    crate::tags::fixtures::write_video_only_mp4(&dir.join("mudo.mp4"));
}
