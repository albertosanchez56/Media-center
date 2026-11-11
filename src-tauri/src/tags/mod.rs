//! Lectura de etiquetas con lofty 0.25.5. Solo se abre el archivo para leer.
//! No hay ninguna llamada que escriba, borre o reescriba el multimedia.
//!
//! lofty no ofrece un tope de tamaño del bloque de etiquetas. Sí limita la
//! búsqueda de bytes no reconocidos (`max_junk_bytes`, 1024). El audio y el
//! vídeo no se copian a un `Vec`: se recorren con el archivo abierto. La
//! portada sí entra en memoria, porque la biblioteca la entrega ya leída, y
//! aquí se descarta si supera 8 MiB o si su firma no es una imagen admitida.
//!
//! La firma reconocida es JPEG, PNG, GIF, BMP y TIFF. WebP no está en
//! `MimeType` ni en el reconocimiento por bytes de lofty 0.25.5, así que una
//! portada WebP no se guarda.

use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::time::SystemTime;

use lofty::config::{ParseOptions, ParsingMode};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::prelude::Accessor;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag};
use serde_json::{Value, json};

pub const COVER_LIMIT: usize = 8 * 1024 * 1024;
const TEXT_LIMIT: usize = 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverImage {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CardDraft {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<i64>,
    pub description: String,
    pub duration_ms: Option<i64>,
    pub snapshot: String,
    pub cover: Option<CoverImage>,
    pub tags_read: bool,
}

pub fn read_card(path: &Path, library: &str, fallback_title: &str) -> CardDraft {
    let before = file_stamp(path);
    let read = std::panic::catch_unwind(AssertUnwindSafe(|| read_values(path)))
        .ok()
        .flatten();
    let after = file_stamp(path);
    if before.is_none() || before != after {
        return provisional(library, fallback_title);
    }
    match read {
        Some(values) => draft_from_values(library, fallback_title, values),
        None => provisional(library, fallback_title),
    }
}

pub fn apply_protected(
    current: &CardDraft,
    protected_json: &str,
    incoming: &CardDraft,
) -> CardDraft {
    let protected = protected_names(protected_json);
    let mut next = incoming.clone();
    if protected.contains("title") {
        next.title = current.title.clone();
    }
    if protected.contains("artist") {
        next.artist = current.artist.clone();
    }
    if protected.contains("album") {
        next.album = current.album.clone();
    }
    if protected.contains("genre") {
        next.genre = current.genre.clone();
    }
    if protected.contains("year") {
        next.year = current.year;
    }
    if protected.contains("description") {
        next.description = current.description.clone();
    }
    if incoming.duration_ms.is_none() {
        next.duration_ms = current.duration_ms;
    }
    if incoming.cover.is_none() {
        next.cover = current.cover.clone();
    }
    next.snapshot = incoming.snapshot.clone();
    next
}

fn provisional(library: &str, fallback_title: &str) -> CardDraft {
    CardDraft {
        title: fallback_title.to_string(),
        artist: String::new(),
        album: String::new(),
        genre: String::new(),
        description: String::new(),
        year: None,
        duration_ms: None,
        snapshot: json!({
            "library": library,
            "title": fallback_title,
            "artist": "",
            "album": "",
            "genre": "",
            "description": "",
        })
        .to_string(),
        cover: None,
        tags_read: false,
    }
}

struct ReadValues {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    genre: Option<String>,
    year: Option<i64>,
    description: Option<String>,
    duration_ms: Option<i64>,
    cover: Option<CoverImage>,
}

fn draft_from_values(library: &str, fallback_title: &str, values: ReadValues) -> CardDraft {
    let snapshot = json!({
        "library": library,
        "title": values.title.clone().unwrap_or_default(),
        "artist": values.artist.clone().unwrap_or_default(),
        "album": values.album.clone().unwrap_or_default(),
        "genre": values.genre.clone().unwrap_or_default(),
        "year": values.year,
        "description": values.description.clone().unwrap_or_default(),
    })
    .to_string();
    CardDraft {
        title: values.title.unwrap_or_else(|| fallback_title.to_string()),
        artist: values.artist.unwrap_or_default(),
        album: values.album.unwrap_or_default(),
        genre: values.genre.unwrap_or_default(),
        year: values.year,
        description: values.description.unwrap_or_default(),
        duration_ms: values.duration_ms,
        snapshot,
        cover: values.cover,
        tags_read: true,
    }
}

fn read_values(path: &Path) -> Option<ReadValues> {
    let options = ParseOptions::new()
        .parsing_mode(ParsingMode::BestAttempt)
        .max_junk_bytes(ParseOptions::DEFAULT_MAX_JUNK_BYTES);
    let tagged = open_tagged(path, options).ok()?;
    let mut values = ReadValues {
        title: None,
        artist: None,
        album: None,
        genre: None,
        year: None,
        description: None,
        duration_ms: duration_of(tagged.properties().duration()),
        cover: None,
    };
    let mut tags: Vec<&Tag> = Vec::new();
    if let Some(primary) = tagged.primary_tag() {
        tags.push(primary);
    }
    for tag in tagged.tags() {
        if !tags.iter().any(|current| std::ptr::eq(*current, tag)) {
            tags.push(tag);
        }
    }
    for tag in tags {
        if values.title.is_none() {
            values.title = owned_text(tag.title());
        }
        if values.artist.is_none() {
            values.artist =
                owned_text(tag.artist()).or_else(|| field_text(tag, ItemKey::AlbumArtist));
        }
        if values.album.is_none() {
            values.album = owned_text(tag.album());
        }
        if values.genre.is_none() {
            values.genre = joined_text(tag, ItemKey::Genre).or_else(|| owned_text(tag.genre()));
        }
        if values.year.is_none() {
            values.year = tag.date().and_then(|date| valid_year(i64::from(date.year)));
        }
        if values.description.is_none() {
            values.description =
                owned_text(tag.comment()).or_else(|| field_text(tag, ItemKey::Description));
        }
        if values.cover.is_none() {
            values.cover = choose_cover(tag.pictures());
        }
    }
    Some(values)
}

fn open_tagged(path: &Path, options: ParseOptions) -> Result<lofty::file::TaggedFile, String> {
    let opened = Probe::open(path)
        .map_err(|error| error.to_string())?
        .options(options);
    match opened.guess_file_type() {
        Ok(guessed) => guessed.read().map_err(|error| error.to_string()),
        Err(_) => Probe::open(path)
            .map_err(|error| error.to_string())?
            .options(options)
            .read()
            .map_err(|error| error.to_string()),
    }
}

fn duration_of(duration: std::time::Duration) -> Option<i64> {
    let millis = duration.as_millis();
    if millis == 0 || millis > i64::MAX as u128 {
        None
    } else {
        Some(millis as i64)
    }
}

fn owned_text(value: Option<impl AsRef<str>>) -> Option<String> {
    value.and_then(|value| limited_text(value.as_ref()))
}

fn field_text(tag: &Tag, key: ItemKey) -> Option<String> {
    tag.get_string(key).and_then(limited_text)
}

fn joined_text(tag: &Tag, key: ItemKey) -> Option<String> {
    let parts: Vec<String> = tag.get_strings(key).filter_map(limited_text).collect();
    if parts.is_empty() {
        None
    } else {
        limited_text(&parts.join(", "))
    }
}

fn limited_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > TEXT_LIMIT {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn valid_year(year: i64) -> Option<i64> {
    if (1000..=9999).contains(&year) {
        Some(year)
    } else {
        None
    }
}

fn choose_cover(pictures: &[lofty::picture::Picture]) -> Option<CoverImage> {
    let mut best: Option<(u8, CoverImage)> = None;
    for picture in pictures {
        let data = picture.data();
        if data.len() > COVER_LIMIT {
            continue;
        }
        let Some(extension) = image_extension(data) else {
            continue;
        };
        let rank = cover_rank(picture.pic_type());
        let replace = best.as_ref().is_none_or(|(current, _)| rank < *current);
        if replace {
            best = Some((
                rank,
                CoverImage {
                    bytes: data.to_vec(),
                    extension,
                },
            ));
        }
    }
    best.map(|(_, cover)| cover)
}

fn cover_rank(kind: PictureType) -> u8 {
    match kind {
        PictureType::CoverFront => 0,
        PictureType::Other => 1,
        PictureType::Media => 2,
        PictureType::Leaflet => 3,
        PictureType::CoverBack => 4,
        PictureType::Icon | PictureType::OtherIcon => 9,
        _ => 5,
    }
}

pub(crate) fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 8 || bytes.len() > COVER_LIMIT {
        return None;
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return None;
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("png");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("gif");
    }
    if bytes.starts_with(b"BM") && bytes.len() >= 14 {
        return Some("bmp");
    }
    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        return Some("tif");
    }
    None
}

fn file_stamp(path: &Path) -> Option<(u64, SystemTime)> {
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.len(), metadata.modified().ok()?))
}

pub(crate) fn protected_names(json_text: &str) -> std::collections::BTreeSet<String> {
    let Ok(value) = serde_json::from_str::<Value>(json_text) else {
        return std::collections::BTreeSet::new();
    };
    let mut names = std::collections::BTreeSet::new();
    match value {
        Value::Array(items) => {
            for item in items {
                if let Some(name) = item.as_str() {
                    remember(&mut names, name);
                }
            }
        }
        Value::Object(map) => {
            for name in map.keys() {
                remember(&mut names, name);
            }
        }
        _ => {}
    }
    names
}

fn remember(names: &mut std::collections::BTreeSet<String>, name: &str) {
    if matches!(
        name,
        "title" | "artist" | "album" | "genre" | "year" | "description"
    ) {
        names.insert(name.to_string());
    }
}

#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webp_and_video_bytes_are_not_covers() {
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&24u32.to_le_bytes());
        webp.extend_from_slice(b"WEBPVP8 ");
        assert!(image_extension(&webp).is_none());
        assert!(image_extension(b"\0\0\0\x18ftypisom").is_none());
        assert_eq!(
            image_extension(&[0xFF, 0xD8, 0xFF, 0, 0, 0, 0, 0]),
            Some("jpg")
        );
    }

    #[test]
    fn a_protected_title_survives_another_read() {
        let current = CardDraft {
            title: "Personal".into(),
            artist: "Viejo".into(),
            album: String::new(),
            genre: String::new(),
            year: None,
            description: String::new(),
            duration_ms: Some(10),
            snapshot: "{}".into(),
            cover: None,
            tags_read: true,
        };
        let incoming = CardDraft {
            title: "Del archivo".into(),
            artist: "Nuevo".into(),
            album: "Disco".into(),
            genre: String::new(),
            year: Some(2001),
            description: String::new(),
            duration_ms: None,
            snapshot: "{\"title\":\"Del archivo\"}".into(),
            cover: None,
            tags_read: true,
        };
        let merged = apply_protected(&current, "[\"title\",\"duration_ms\"]", &incoming);
        assert_eq!(merged.title, "Personal");
        assert_eq!(merged.artist, "Nuevo");
        assert_eq!(merged.duration_ms, Some(10));
        assert_eq!(merged.snapshot, incoming.snapshot);
    }

    #[test]
    fn an_empty_object_protects_nothing() {
        let current = provisional("music", "clip");
        let incoming = draft_from_values(
            "music",
            "clip",
            ReadValues {
                title: Some("Cancion".into()),
                artist: Some("Artista".into()),
                album: None,
                genre: None,
                year: Some(1999),
                description: None,
                duration_ms: Some(26),
                cover: None,
            },
        );
        let merged = apply_protected(&current, "{}", &incoming);
        assert_eq!(merged.title, "Cancion");
        assert_eq!(merged.year, Some(1999));
    }
}
