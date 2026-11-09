-- Migración 1, esquema inicial aprobado en DC-18. No modificar este archivo.

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
) STRICT;

CREATE TABLE stored_files (
    id INTEGER PRIMARY KEY,
    storage TEXT NOT NULL CHECK (storage IN ('managed', 'external')),
    path TEXT NOT NULL,
    path_key TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL,
    modified_at TEXT NOT NULL,
    content_hash TEXT,
    file_id TEXT,
    container TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('available', 'missing')),
    missing_since TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE media_items (
    id INTEGER PRIMARY KEY,
    library TEXT NOT NULL CHECK (library IN ('music', 'video', 'movie')),
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT NOT NULL,
    genre TEXT NOT NULL,
    year INTEGER,
    description TEXT NOT NULL,
    duration_ms INTEGER,
    favorite INTEGER NOT NULL CHECK (favorite IN (0, 1)),
    visual_preference TEXT CHECK (
        visual_preference IS NULL
        OR visual_preference IN ('cover', 'original_video', 'associated_video')
    ),
    resume_position_ms INTEGER,
    imported_snapshot TEXT NOT NULL CHECK (json_valid(imported_snapshot)),
    user_edited_fields TEXT NOT NULL CHECK (json_valid(user_edited_fields)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE item_files (
    id INTEGER PRIMARY KEY,
    media_item_id INTEGER NOT NULL REFERENCES media_items (id) ON DELETE CASCADE,
    stored_file_id INTEGER NOT NULL REFERENCES stored_files (id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN ('primary', 'associated_video')),
    created_at TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX item_files_one_primary
    ON item_files (media_item_id)
    WHERE role = 'primary';

CREATE UNIQUE INDEX item_files_one_associated_video
    ON item_files (media_item_id)
    WHERE role = 'associated_video';

CREATE TABLE media_images (
    id INTEGER PRIMARY KEY,
    media_item_id INTEGER NOT NULL REFERENCES media_items (id) ON DELETE CASCADE,
    source TEXT NOT NULL CHECK (source IN ('embedded', 'download', 'user')),
    relative_path TEXT NOT NULL,
    selected INTEGER NOT NULL CHECK (selected IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX media_images_one_selected
    ON media_images (media_item_id)
    WHERE selected = 1;

CREATE TABLE playlists (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    library TEXT CHECK (library IS NULL OR library IN ('music', 'video', 'movie')),
    source_url TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE playlist_items (
    id INTEGER PRIMARY KEY,
    playlist_id INTEGER NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
    media_item_id INTEGER NOT NULL REFERENCES media_items (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    UNIQUE (playlist_id, position)
) STRICT;

CREATE TABLE download_batches (
    id INTEGER PRIMARY KEY,
    source_url TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('single', 'playlist')),
    library TEXT NOT NULL CHECK (library IN ('music', 'video', 'movie')),
    mode TEXT NOT NULL CHECK (mode IN ('audio', 'audio_video')),
    format TEXT NOT NULL,
    quality TEXT NOT NULL,
    cancel_requested INTEGER NOT NULL CHECK (cancel_requested IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE download_jobs (
    id INTEGER PRIMARY KEY,
    batch_id INTEGER NOT NULL REFERENCES download_batches (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    source_url TEXT NOT NULL,
    source_key TEXT,
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'failed', 'cancelled')),
    error TEXT,
    attempt_count INTEGER NOT NULL,
    media_item_id INTEGER REFERENCES media_items (id) ON DELETE SET NULL,
    stored_file_id INTEGER REFERENCES stored_files (id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (batch_id, position)
) STRICT;

CREATE INDEX download_jobs_source_key ON download_jobs (source_key);

CREATE TABLE file_operations (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('copy', 'move', 'download')),
    method TEXT NOT NULL CHECK (method IN ('rename', 'copy')),
    library TEXT NOT NULL CHECK (library IN ('music', 'video', 'movie')),
    source_path TEXT NOT NULL,
    source_size_bytes INTEGER NOT NULL,
    source_modified_at TEXT,
    temp_relative_path TEXT,
    destination_relative_path TEXT NOT NULL,
    stage TEXT NOT NULL CHECK (
        stage IN (
            'planned',
            'transferring',
            'verifying',
            'ready_for_db',
            'db_committed',
            'source_delete_pending',
            'completed',
            'failed',
            'needs_review'
        )
    ),
    media_item_id INTEGER REFERENCES media_items (id) ON DELETE SET NULL,
    stored_file_id INTEGER REFERENCES stored_files (id) ON DELETE SET NULL,
    error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;
