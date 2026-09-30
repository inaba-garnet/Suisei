//! lofty でタグと音声の情報を読む。フォーマットの差はここで吸収する。

use std::path::Path;

use lofty::file::{AudioFile, FileType, TaggedFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey, Tag};

/// タグの値をそのまま持つ。空白だけの値は無いものとして扱う。
/// DB に JSON で保存するので、項目を足しても古い JSON を読めるよう、欠けた項目は既定値にする。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
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
    pub composers: Vec<String>,
    pub lyricists: Vec<String>,
    pub arrangers: Vec<String>,
    pub compilation: bool,
    /// 埋め込みの画像があるか。カバーアートを探すのに使う
    pub has_picture: bool,
}

/// DB に保存するタグの版。タグから読む項目を足したら上げ、古い版で保存したファイルを読み直させる。
/// 版 1 で `has_picture`、版 2 で `composers`、`lyricists`、`arrangers` を足した。
const STORED_VERSION: u32 = 2;

#[derive(serde::Serialize, serde::Deserialize)]
struct Stored {
    /// 版を持つ前の JSON は 0 になる
    #[serde(default)]
    version: u32,
    #[serde(flatten)]
    tags: RawTags,
}

/// DB に保存する JSON にする。
pub fn to_stored(tags: &RawTags) -> String {
    serde_json::to_string(&Stored {
        version: STORED_VERSION,
        tags: tags.clone(),
    })
    .expect("RawTags は JSON にできる")
}

/// 保存した JSON を読む。壊れていれば None。`current` は今の版で保存したものか。
pub fn from_stored(json: &str) -> Option<(RawTags, bool)> {
    let stored: Stored = serde_json::from_str(json).ok()?;
    Some((stored.tags, stored.version >= STORED_VERSION))
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
        composers: many(ItemKey::Composer),
        lyricists: many(ItemKey::Lyricist),
        arrangers: many(ItemKey::Arranger),
        compilation: tag
            .get_string(ItemKey::FlagCompilation)
            .is_some_and(|value| matches!(value.trim(), "1") || value.eq_ignore_ascii_case("true")),
        has_picture: !tag.pictures().is_empty(),
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
            composers: Vec::new(),
            lyricists: Vec::new(),
            arrangers: Vec::new(),
            compilation: true,
            has_picture: false,
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
    fn embedded_picture() {
        let (tags, _) = fixture("picture.flac");
        assert!(tags.has_picture);
        assert!(!fixture("full.flac").0.has_picture);
    }

    #[test]
    fn stored_json_round_trips() {
        let tags = full();
        assert_eq!(from_stored(&to_stored(&tags)), Some((tags, true)));
        // 版を持つ前の JSON は、読めても今の版ではない
        let (old, current) = from_stored(r#"{"title":"a"}"#).unwrap();
        assert_eq!(old.title.as_deref(), Some("a"));
        assert!(!current);
        assert_eq!(from_stored("broken"), None);
    }

    #[test]
    fn no_tags() {
        let (tags, props) = fixture("notag.flac");
        assert_eq!(tags, RawTags::default());
        assert!(props.lossless);
    }
}
