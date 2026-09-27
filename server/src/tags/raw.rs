//! lofty でタグと音声の情報を読む。フォーマットの差はここで吸収する。

use std::path::Path;

use lofty::file::{AudioFile, FileType, TaggedFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey, Tag};

/// タグの値をそのまま持つ。空白だけの値は無いものとして扱う。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawTags {
    pub title: Option<String>,
    pub album: Option<String>,
    /// ARTIST。表示用の文字列。複数値のファイルもある
    pub artist: Vec<String>,
    /// ARTISTS。一人ずつの名前
    pub artists: Vec<String>,
    pub album_artist: Vec<String>,
    pub album_artists: Vec<String>,
    pub title_sort: Option<String>,
    pub album_sort: Option<String>,
    pub artist_sort: Option<String>,
    pub album_artist_sort: Option<String>,
    pub mb_artist_ids: Vec<String>,
    pub mb_album_artist_ids: Vec<String>,
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
    pub year: Option<i32>,
    pub genres: Vec<String>,
    pub compilation: bool,
}

/// file 表の列に対応する音声の情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProps {
    pub suffix: String,
    pub content_type: &'static str,
    pub duration_ms: u64,
    /// kbps
    pub bit_rate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bit_depth: Option<u8>,
    pub lossless: bool,
}

/// ファイルを読み、タグと音声の情報を返す。
pub fn read(path: &Path) -> Result<(RawTags, AudioProps), lofty::error::FileParseError> {
    let file = lofty::read_from_path(path)?;
    let tags = file
        // MP3 に ID3v1 しかないときは、primary（ID3v2）がないので次の候補を使う
        .primary_tag()
        .or_else(|| file.first_tag())
        .map(from_tag)
        .unwrap_or_default();
    Ok((tags, audio_props(path, &file)))
}

fn from_tag(tag: &Tag) -> RawTags {
    let one = |key| tag.get_string(key).and_then(non_empty);
    let many = |key| {
        tag.get_strings(key)
            .filter_map(non_empty)
            .collect::<Vec<_>>()
    };
    RawTags {
        title: one(ItemKey::TrackTitle),
        album: one(ItemKey::AlbumTitle),
        artist: many(ItemKey::TrackArtist),
        artists: many(ItemKey::TrackArtists),
        album_artist: many(ItemKey::AlbumArtist),
        album_artists: many(ItemKey::AlbumArtists),
        title_sort: one(ItemKey::TrackTitleSortOrder),
        album_sort: one(ItemKey::AlbumTitleSortOrder),
        artist_sort: one(ItemKey::TrackArtistSortOrder),
        album_artist_sort: one(ItemKey::AlbumArtistSortOrder),
        mb_artist_ids: many(ItemKey::MusicBrainzArtistId),
        mb_album_artist_ids: many(ItemKey::MusicBrainzReleaseArtistId),
        disc_number: tag.disk(),
        track_number: tag.track(),
        year: tag.date().map(|date| i32::from(date.year)),
        genres: many(ItemKey::Genre),
        compilation: tag
            .get_string(ItemKey::FlagCompilation)
            .is_some_and(|value| matches!(value.trim(), "1") || value.eq_ignore_ascii_case("true")),
    }
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn audio_props(path: &Path, file: &TaggedFile) -> AudioProps {
    let props = file.properties();
    let file_type = file.file_type();
    let lossless = match file_type {
        FileType::Flac | FileType::Wav | FileType::Aiff | FileType::Ape | FileType::WavPack => true,
        // MP4 は ALAC のときだけ lofty がビット深度を持つ
        FileType::Mp4 => props.bit_depth().is_some(),
        _ => false,
    };
    let suffix = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    AudioProps {
        content_type: content_type(file_type),
        suffix,
        duration_ms: u64::try_from(props.duration().as_millis()).unwrap_or(u64::MAX),
        bit_rate: props.audio_bitrate().or(props.overall_bitrate()),
        sample_rate: props.sample_rate(),
        channels: props.channels(),
        bit_depth: props.bit_depth(),
        lossless,
    }
}

fn content_type(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Aac => "audio/aac",
        FileType::Aiff => "audio/aiff",
        FileType::Ape => "audio/x-ape",
        FileType::Flac => "audio/flac",
        FileType::Mpeg => "audio/mpeg",
        FileType::Mp4 => "audio/mp4",
        FileType::Mpc => "audio/x-musepack",
        FileType::Opus => "audio/ogg; codecs=opus",
        FileType::Vorbis => "audio/ogg",
        FileType::Speex => "audio/ogg; codecs=speex",
        FileType::Wav => "audio/wav",
        FileType::WavPack => "audio/x-wavpack",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> (RawTags, AudioProps) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tags")
            .join(name);
        read(&path).unwrap()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    /// tests/fixtures/tags/generate.py で全項目を書いたファイルの値
    fn full() -> RawTags {
        RawTags {
            title: Some("テスト曲".into()),
            album: Some("テストアルバム".into()),
            artist: strings(&["歌手A feat. 歌手B"]),
            artists: strings(&["歌手A", "歌手B"]),
            album_artist: strings(&["歌手A"]),
            album_artists: strings(&["歌手A"]),
            title_sort: Some("てすときょく".into()),
            album_sort: Some("てすとあるばむ".into()),
            artist_sort: Some("Kashu A feat. Kashu B".into()),
            album_artist_sort: Some("かしゅA".into()),
            mb_artist_ids: strings(&[
                "00000000-0000-0000-0000-00000000000a",
                "00000000-0000-0000-0000-00000000000b",
            ]),
            mb_album_artist_ids: strings(&["00000000-0000-0000-0000-00000000000a"]),
            disc_number: Some(2),
            track_number: Some(3),
            year: Some(2015),
            genres: strings(&["Rock", "Pop"]),
            compilation: true,
        }
    }

    #[test]
    fn flac() {
        let (tags, props) = fixture("full.flac");
        assert_eq!(tags, full());
        assert_eq!(props.suffix, "flac");
        assert_eq!(props.content_type, "audio/flac");
        assert_eq!(props.duration_ms, 1000);
        assert_eq!(props.sample_rate, Some(8000));
        assert_eq!(props.channels, Some(1));
        assert_eq!(props.bit_depth, Some(16));
        assert!(props.lossless);
    }

    #[test]
    fn mp3_id3v24() {
        let (tags, props) = fixture("full.mp3");
        assert_eq!(tags, full());
        assert_eq!(props.content_type, "audio/mpeg");
        assert_eq!(props.bit_rate, Some(8));
        assert!(!props.lossless);
    }

    #[test]
    fn mp3_id3v23() {
        let (tags, _) = fixture("full-v23.mp3");
        // ID3v2.3 は複数値を持てず、書き込み側が "/" でつなぐ。名前に "/" を含むアーティストがいるので分割しない
        let expected = RawTags {
            artists: strings(&["歌手A/歌手B"]),
            mb_artist_ids: strings(&[
                "00000000-0000-0000-0000-00000000000a/00000000-0000-0000-0000-00000000000b",
            ]),
            genres: strings(&["Rock/Pop"]),
            ..full()
        };
        assert_eq!(tags, expected);
    }

    #[test]
    fn mp3_id3v1_only() {
        let (tags, _) = fixture("id3v1.mp3");
        let expected = RawTags {
            title: Some("Title".into()),
            artist: strings(&["Artist"]),
            album: Some("Album".into()),
            track_number: Some(3),
            year: Some(2015),
            ..RawTags::default()
        };
        assert_eq!(tags, expected);
    }

    #[test]
    fn m4a() {
        let (tags, props) = fixture("full.m4a");
        assert_eq!(tags, full());
        assert_eq!(props.content_type, "audio/mp4");
        assert!(!props.lossless);
    }

    #[test]
    fn no_tags() {
        let (tags, props) = fixture("notag.flac");
        assert_eq!(tags, RawTags::default());
        assert!(props.lossless);
    }
}
