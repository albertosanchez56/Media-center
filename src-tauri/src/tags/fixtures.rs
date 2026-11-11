use std::fs;
use std::path::Path;

use lofty::config::WriteOptions;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::tag::items::Timestamp;
use lofty::tag::{Tag, TagType};

pub const JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08,
    0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
    0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20,
    0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27,
    0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
    0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xC4, 0x00, 0x14,
    0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x00, 0xFF, 0xD9,
];

pub fn write_mp3(path: &Path) {
    fs::write(path, mpeg_frame()).unwrap();
    save_tag(path, TagType::Id3v2, Some(JPEG));
}

pub fn write_flac(path: &Path) {
    fs::write(path, flac_stream()).unwrap();
    save_tag(path, TagType::VorbisComments, Some(JPEG));
}

pub fn write_mp4(path: &Path) {
    fs::write(path, mp4_with_cover(JPEG)).unwrap();
}

pub fn write_mp3_with_webp_and_title(path: &Path) {
    fs::write(path, mpeg_frame()).unwrap();
    let mut tag = Tag::new(TagType::Id3v2);
    tag.set_title("Con webp".to_string());
    let mut webp = b"RIFF".to_vec();
    webp.extend_from_slice(&12u32.to_le_bytes());
    webp.extend_from_slice(b"WEBPVP8 ");
    let picture = Picture::unchecked(webp)
        .pic_type(PictureType::CoverFront)
        .mime_type(MimeType::from_str("image/webp"))
        .build();
    tag.push_picture(picture);
    tag.save_to_path(path, WriteOptions::new()).unwrap();
}

pub fn write_mp3_with_oversized_cover(path: &Path) {
    fs::write(path, mpeg_frame()).unwrap();
    let mut blob = vec![0xFF, 0xD8, 0xFF];
    blob.resize(super::COVER_LIMIT + 1, 0);
    let mut tag = Tag::new(TagType::Id3v2);
    tag.set_title("Grande".to_string());
    tag.set_artist("Artista".to_string());
    let picture = Picture::unchecked(blob)
        .pic_type(PictureType::CoverFront)
        .mime_type(MimeType::Jpeg)
        .build();
    tag.push_picture(picture);
    tag.save_to_path(path, WriteOptions::new()).unwrap();
}

fn save_tag(path: &Path, kind: TagType, cover: Option<&[u8]>) {
    let mut tag = Tag::new(kind);
    tag.set_title("Cancion".to_string());
    tag.set_artist("Artista".to_string());
    tag.set_album("Disco".to_string());
    tag.set_genre("Rock".to_string());
    tag.set_comment("Una nota".to_string());
    tag.set_date(Timestamp {
        year: 1999,
        ..Timestamp::default()
    });
    if let Some(bytes) = cover {
        let picture = Picture::unchecked(bytes.to_vec())
            .pic_type(PictureType::CoverFront)
            .mime_type(MimeType::Jpeg)
            .build();
        tag.push_picture(picture);
    }
    tag.save_to_path(path, WriteOptions::new()).unwrap();
}

fn mpeg_frame() -> Vec<u8> {
    // lofty only accepts a frame when the next header matches, so the fixture
    // carries two identical MPEG1 Layer III frames (128 kbps, 44100 Hz, 417 bytes).
    let mut frame = vec![0xFF, 0xFB, 0x90, 0x00];
    frame.resize(417, 0);
    let mut audio = frame.clone();
    audio.extend_from_slice(&frame);
    audio
}

fn flac_stream() -> Vec<u8> {
    let mut bits = 0u64;
    bits |= 44_100u64 << 44;
    bits |= 1u64 << 41;
    bits |= 15u64 << 36;
    bits |= 44_100u64;
    let mut body = vec![0x10, 0x00, 0x10, 0x00, 0, 0, 0, 0, 0, 0];
    body.extend_from_slice(&bits.to_be_bytes());
    body.resize(34, 0);
    let mut file = b"fLaC".to_vec();
    file.push(0x80);
    file.extend_from_slice(&34u32.to_be_bytes()[1..]);
    file.extend_from_slice(&body);
    file
}

fn mp4_with_cover(jpeg: &[u8]) -> Vec<u8> {
    let mut ilst = Vec::new();
    ilst.extend(text_atom(&[0xA9, b'n', b'a', b'm'], "Cancion"));
    ilst.extend(text_atom(&[0xA9, b'A', b'R', b'T'], "Artista"));
    ilst.extend(text_atom(&[0xA9, b'a', b'l', b'b'], "Disco"));
    ilst.extend(text_atom(&[0xA9, b'g', b'e', b'n'], "Rock"));
    ilst.extend(text_atom(&[0xA9, b'd', b'a', b'y'], "1999"));
    ilst.extend(text_atom(&[0xA9, b'c', b'm', b't'], "Una nota"));
    ilst.extend(atom(b"covr", &data_atom(13, jpeg)));
    let meta_body = {
        let mut body = vec![0, 0, 0, 0];
        body.extend(atom(b"ilst", &ilst));
        body
    };
    let udta = atom(b"udta", &atom(b"meta", &meta_body));
    let moov = atom(b"moov", &{
        let mut body = mvhd();
        body.extend(audio_trak());
        body.extend(udta);
        body
    });
    let mut ftyp_body = b"M4A ".to_vec();
    ftyp_body.extend_from_slice(&0u32.to_be_bytes());
    ftyp_body.extend_from_slice(b"M4A ");
    let mut file = atom(b"ftyp", &ftyp_body);
    file.extend(moov);
    file
}

fn audio_trak() -> Vec<u8> {
    let mut mdhd = vec![0, 0, 0, 0];
    mdhd.extend_from_slice(&0u32.to_be_bytes());
    mdhd.extend_from_slice(&0u32.to_be_bytes());
    mdhd.extend_from_slice(&1000u32.to_be_bytes());
    mdhd.extend_from_slice(&1000u32.to_be_bytes());
    let mut hdlr = vec![0, 0, 0, 0, 0, 0, 0, 0];
    hdlr.extend_from_slice(b"soun");
    let mut mdia = atom(b"mdhd", &mdhd);
    mdia.extend(atom(b"hdlr", &hdlr));
    atom(b"trak", &atom(b"mdia", &mdia))
}

fn mvhd() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&[0, 0, 0, 0]);
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&1000u32.to_be_bytes());
    body.extend_from_slice(&1000u32.to_be_bytes());
    body.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    body.extend_from_slice(&0x0100u16.to_be_bytes());
    body.extend_from_slice(&[0; 10]);
    for value in [0x0001_0000u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
        body.extend_from_slice(&value.to_be_bytes());
    }
    body.extend_from_slice(&[0; 24]);
    body.extend_from_slice(&2u32.to_be_bytes());
    atom(b"mvhd", &body)
}

fn text_atom(name: &[u8], text: &str) -> Vec<u8> {
    atom(name, &data_atom(1, text.as_bytes()))
}

fn data_atom(kind: u32, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&kind.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(payload);
    atom(b"data", &body)
}

fn atom(kind: &[u8], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&((8 + body.len()) as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    out
}
