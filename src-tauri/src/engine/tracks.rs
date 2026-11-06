use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub id: Option<i64>,
    pub kind: String,
    pub selected: bool,
    pub external: bool,
    pub external_filename: Option<String>,
    pub albumart: bool,
    pub title: Option<String>,
    pub language: Option<String>,
    pub codec: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackReport {
    pub tracks: Vec<TrackInfo>,
    /// Hay una sola pista de audio activa y pertenece al archivo principal.
    pub audio_only_from_main_file: bool,
    /// Archivo del que procede la pista de vídeo activa, si es externa.
    pub selected_external_video: Option<String>,
    /// Estado del audio y de la reproducción en el momento de la lectura.
    pub playback: Vec<PropertyReading>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PropertyReading {
    pub name: &'static str,
    pub value: Option<String>,
}

/// Propiedades que muestran si el audio suena tras un salto (`audio-pts` falta mientras espera)
/// y qué subtítulo se está dibujando (`sub-text`).
const PLAYBACK_PROPERTIES: [&str; 14] = [
    "aid",
    "vid",
    "sid",
    "sub-text",
    "fullscreen",
    "mute",
    "volume",
    "current-ao",
    "audio-pts",
    "time-pos",
    "core-idle",
    "seeking",
    "hr-seek",
    "vo-configured",
];

/// Lee `track-list` con un lector de propiedades en formato texto, común a los dos motores.
pub fn read_tracks(
    get: impl Fn(&str) -> Result<Option<String>, String>,
) -> Result<TrackReport, String> {
    let count = get("track-list/count")?
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut tracks = Vec::with_capacity(count);
    for index in 0..count {
        let field = |name: &str| get(&format!("track-list/{index}/{name}"));
        tracks.push(TrackInfo {
            id: field("id")?.and_then(|value| value.parse().ok()),
            kind: field("type")?.unwrap_or_default(),
            selected: is_yes(field("selected")?),
            external: is_yes(field("external")?),
            external_filename: field("external-filename")?,
            albumart: is_yes(field("albumart")?),
            title: field("title")?,
            language: field("lang")?,
            codec: field("codec")?,
        });
    }
    let playback = PLAYBACK_PROPERTIES
        .iter()
        .map(|&name| {
            Ok(PropertyReading {
                name,
                value: get(name)?,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(summarize(tracks, playback))
}

fn summarize(tracks: Vec<TrackInfo>, playback: Vec<PropertyReading>) -> TrackReport {
    let selected_audio: Vec<&TrackInfo> = tracks
        .iter()
        .filter(|track| track.kind == "audio" && track.selected)
        .collect();
    let audio_only_from_main_file = selected_audio.len() == 1 && !selected_audio[0].external;
    let selected_external_video = tracks
        .iter()
        .find(|track| track.kind == "video" && track.selected && track.external)
        .and_then(|track| track.external_filename.clone());
    TrackReport {
        tracks,
        audio_only_from_main_file,
        selected_external_video,
        playback,
    }
}

fn is_yes(value: Option<String>) -> bool {
    matches!(value.as_deref(), Some("yes" | "true"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn getter(values: &[(&str, &str)]) -> impl Fn(&str) -> Result<Option<String>, String> + use<> {
        let map: HashMap<String, String> = values
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        move |name| Ok(map.get(name).cloned())
    }

    #[test]
    fn song_with_associated_video_uses_only_the_song_audio() {
        let report = read_tracks(getter(&[
            ("track-list/count", "3"),
            ("track-list/0/id", "1"),
            ("track-list/0/type", "audio"),
            ("track-list/0/selected", "yes"),
            ("track-list/0/external", "no"),
            ("track-list/1/id", "1"),
            ("track-list/1/type", "video"),
            ("track-list/1/selected", "no"),
            ("track-list/1/albumart", "yes"),
            ("track-list/2/id", "2"),
            ("track-list/2/type", "video"),
            ("track-list/2/selected", "yes"),
            ("track-list/2/external", "yes"),
            ("track-list/2/external-filename", "C:/videos/clip.mp4"),
        ]))
        .unwrap();

        assert_eq!(report.tracks.len(), 3);
        assert!(report.audio_only_from_main_file);
        assert_eq!(
            report.selected_external_video.as_deref(),
            Some("C:/videos/clip.mp4")
        );
        assert!(report.tracks[1].albumart);
    }

    #[test]
    fn reads_the_language_of_each_track() {
        let report = read_tracks(getter(&[
            ("track-list/count", "2"),
            ("track-list/0/type", "audio"),
            ("track-list/0/lang", "spa"),
            ("track-list/1/type", "sub"),
            ("track-list/1/lang", "jpn"),
            ("track-list/1/selected", "yes"),
        ]))
        .unwrap();
        assert_eq!(report.tracks[0].language.as_deref(), Some("spa"));
        assert_eq!(report.tracks[1].language.as_deref(), Some("jpn"));
        assert!(report.tracks[1].selected);
    }

    #[test]
    fn external_audio_is_detected() {
        let report = read_tracks(getter(&[
            ("track-list/count", "1"),
            ("track-list/0/type", "audio"),
            ("track-list/0/selected", "yes"),
            ("track-list/0/external", "yes"),
        ]))
        .unwrap();
        assert!(!report.audio_only_from_main_file);
    }

    #[test]
    fn empty_player_has_no_tracks() {
        let report = read_tracks(getter(&[])).unwrap();
        assert!(report.tracks.is_empty());
        assert!(!report.audio_only_from_main_file);
    }

    #[test]
    fn playback_state_reports_missing_audio_position() {
        let report = read_tracks(getter(&[("aid", "1"), ("mute", "no")])).unwrap();
        let value = |name: &str| {
            report
                .playback
                .iter()
                .find(|reading| reading.name == name)
                .and_then(|reading| reading.value.clone())
        };
        assert_eq!(value("aid").as_deref(), Some("1"));
        assert_eq!(value("mute").as_deref(), Some("no"));
        assert_eq!(value("audio-pts"), None);
    }
}
