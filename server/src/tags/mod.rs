//! タグを内部のモデルに変換する。DB には依存しない。
//! ファイルをまたぐ集計（表示名や読みの多数決）はスキャンで行う。

mod match_key;
mod raw;
mod reading;

use std::path::Path;

pub use match_key::normalize;
pub use raw::{AudioProps, RawTags, read};
pub use reading::{Reading, ReadingSource, reading};

pub const UNKNOWN_ARTIST: &str = "[Unknown Artist]";
pub const UNKNOWN_ALBUM: &str = "[Unknown Album]";

/// 曲とアルバムに付くアーティスト。track_artist と album_artist の一行に対応する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    pub name: String,
    /// アーティストが一人のファイルに限り、ソート用タグの値を持つ
    pub sort: Option<String>,
    pub match_key: String,
}

impl Credit {
    fn new(name: &str, sort: Option<&str>) -> Self {
        Self {
            name: name.to_owned(),
            sort: sort.map(str::to_owned),
            match_key: normalize(name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumInfo {
    pub name: String,
    pub reading: Option<Reading>,
    pub display_artist: String,
    pub artists: Vec<Credit>,
    pub compilation: bool,
    pub match_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackInfo {
    pub title: String,
    pub reading: Option<Reading>,
    pub display_artist: String,
    pub artists: Vec<Credit>,
    pub album: AlbumInfo,
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
    pub year: Option<i32>,
    pub genres: Vec<String>,
    pub match_key: String,
}

impl TrackInfo {
    /// `path` は音楽フォルダからの相対パス。タグが欠けたときの代わりの値に使う。
    pub fn new(tags: &RawTags, path: &Path) -> Self {
        let title = tags.title.clone().unwrap_or_else(|| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        let artists = track_artists(tags);
        let display_artist = display_artist(&tags.artist, &tags.artists)
            .unwrap_or_else(|| UNKNOWN_ARTIST.to_owned());
        let album = album(tags, path, &artists, &display_artist);
        let disc = tags.disc_number.map(|n| n.to_string()).unwrap_or_default();
        let track = tags.track_number.map(|n| n.to_string()).unwrap_or_default();
        let match_key = match_key::join([
            normalize(&title).as_str(),
            &credit_keys(&artists),
            &normalize(&album.name),
            &disc,
            &track,
        ]);
        Self {
            reading: reading(&title, tags.title_sort.as_deref()),
            title,
            display_artist,
            artists,
            album,
            disc_number: tags.disc_number,
            track_number: tags.track_number,
            year: tags.year,
            genres: tags.genres.clone(),
            match_key,
        }
    }
}

/// ARTISTS を使い、なければ ARTIST の値を分割せずに一人ずつ使う。
fn track_artists(tags: &RawTags) -> Vec<Credit> {
    let names: Vec<&str> = if !tags.artists.is_empty() {
        tags.artists.iter().map(String::as_str).collect()
    } else if !tags.artist.is_empty() {
        tags.artist.iter().map(String::as_str).collect()
    } else {
        vec![UNKNOWN_ARTIST]
    };
    credits(&names, tags.artist_sort.as_deref())
}

/// ソート用タグは、名前が一つのときだけ誰のものか分かる。
fn credits<S: AsRef<str>>(names: &[S], sort: Option<&str>) -> Vec<Credit> {
    let sort = if names.len() == 1 { sort } else { None };
    names
        .iter()
        .map(|name| Credit::new(name.as_ref(), sort))
        .collect()
}

/// ARTIST を使い、なければ ARTISTS を使う。複数値は " / " でつなぐ。
fn display_artist(artist: &[String], artists: &[String]) -> Option<String> {
    [artist, artists]
        .into_iter()
        .find(|values| !values.is_empty())
        .map(|values| values.join(" / "))
}

fn album(tags: &RawTags, path: &Path, track_artists: &[Credit], track_display: &str) -> AlbumInfo {
    let name = tags.album.clone().unwrap_or_else(|| {
        // 音楽フォルダの直下にあるファイルは、フォルダ名を使えない
        path.parent()
            .and_then(Path::file_name)
            .map(|dir| dir.to_string_lossy().into_owned())
            .unwrap_or_else(|| UNKNOWN_ALBUM.to_owned())
    });
    let artists = album_artists(tags, track_artists).unwrap_or_else(|| track_artists.to_vec());
    let display_artist = display_artist(&tags.album_artist, &tags.album_artists)
        .unwrap_or_else(|| track_display.to_owned());
    let match_key = match_key::join([normalize(&name).as_str(), &credit_keys(&artists)]);
    AlbumInfo {
        reading: reading(&name, tags.album_sort.as_deref()),
        name,
        display_artist,
        artists,
        compilation: tags.compilation,
        match_key,
    }
}

/// アルバムのアーティスト。曲のアーティストをそのまま使うときは None。
fn album_artists(tags: &RawTags, track_artists: &[Credit]) -> Option<Vec<Credit>> {
    if !tags.album_artists.is_empty() {
        return Some(credits(
            &tags.album_artists,
            tags.album_artist_sort.as_deref(),
        ));
    }
    if let Some(credits) = album_artists_by_mbid(tags, track_artists) {
        return Some(credits);
    }
    if tags.album_artist.is_empty() || same_names(&tags.album_artist, &tags.artist) {
        return None;
    }
    Some(credits(
        &tags.album_artist,
        tags.album_artist_sort.as_deref(),
    ))
}

/// MusicBrainz のアルバムアーティスト ID を曲のアーティスト ID と対応させ、ARTISTS の名前を使う。
/// 一つでも対応しなければ None。
fn album_artists_by_mbid(tags: &RawTags, track_artists: &[Credit]) -> Option<Vec<Credit>> {
    if tags.mb_album_artist_ids.is_empty()
        || tags.artists.is_empty()
        || tags.mb_artist_ids.len() != tags.artists.len()
    {
        return None;
    }
    tags.mb_album_artist_ids
        .iter()
        .map(|id| {
            let i = tags.mb_artist_ids.iter().position(|a| a == id)?;
            Some(track_artists[i].clone())
        })
        .collect()
}

fn same_names(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| normalize(a) == normalize(b))
}

fn credit_keys(credits: &[Credit]) -> String {
    match_key::list(credits.iter().map(|c| c.match_key.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(credits: &[Credit]) -> Vec<&str> {
        credits.iter().map(|c| c.name.as_str()).collect()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    fn info(tags: &RawTags) -> TrackInfo {
        TrackInfo::new(tags, Path::new("アーティスト/アルバム/01 曲.flac"))
    }

    #[test]
    fn no_tags() {
        let track = info(&RawTags::default());
        assert_eq!(track.title, "01 曲");
        assert_eq!(track.display_artist, UNKNOWN_ARTIST);
        assert_eq!(names(&track.artists), [UNKNOWN_ARTIST]);
        assert_eq!(track.album.name, "アルバム");
        assert_eq!(names(&track.album.artists), [UNKNOWN_ARTIST]);
    }

    #[test]
    fn file_at_folder_root_has_unknown_album() {
        let track = TrackInfo::new(&RawTags::default(), Path::new("notag.flac"));
        assert_eq!(track.title, "notag");
        assert_eq!(track.album.name, UNKNOWN_ALBUM);
    }

    #[test]
    fn artists_tag_is_preferred() {
        let tags = RawTags {
            artist: strings(&["吉本おじさん feat. 雨衣"]),
            artists: strings(&["吉本おじさん", "雨衣"]),
            artist_sort: Some("よしもとおじさん".into()),
            ..RawTags::default()
        };
        let track = info(&tags);
        assert_eq!(track.display_artist, "吉本おじさん feat. 雨衣");
        assert_eq!(names(&track.artists), ["吉本おじさん", "雨衣"]);
        // 二人いるので、どちらのソート用タグか分からない
        assert!(track.artists.iter().all(|c| c.sort.is_none()));
    }

    #[test]
    fn artist_tag_is_not_split() {
        let tags = RawTags {
            artist: strings(&["A、B"]),
            artist_sort: Some("えー".into()),
            ..RawTags::default()
        };
        let track = info(&tags);
        assert_eq!(names(&track.artists), ["A、B"]);
        assert_eq!(track.artists[0].sort.as_deref(), Some("えー"));
    }

    #[test]
    fn multi_valued_artist_tag() {
        // fixture の鷹嶺ルイは ARTIST そのものが複数値
        let tags = RawTags {
            artist: strings(&["鷹嶺ルイ", "柊キライ"]),
            album_artist: strings(&["鷹嶺ルイ"]),
            ..RawTags::default()
        };
        let track = info(&tags);
        assert_eq!(track.display_artist, "鷹嶺ルイ / 柊キライ");
        assert_eq!(names(&track.artists), ["鷹嶺ルイ", "柊キライ"]);
        assert_eq!(names(&track.album.artists), ["鷹嶺ルイ"]);
    }

    #[test]
    fn display_artist_joins_artists() {
        let tags = RawTags {
            artists: strings(&["鷹嶺ルイ", "B"]),
            ..RawTags::default()
        };
        assert_eq!(info(&tags).display_artist, "鷹嶺ルイ / B");
    }

    #[test]
    fn album_artists_tag() {
        let tags = RawTags {
            artist: strings(&["A feat. B"]),
            artists: strings(&["A", "B"]),
            album_artist: strings(&["A"]),
            album_artists: strings(&["A"]),
            album_artist_sort: Some("えー".into()),
            ..RawTags::default()
        };
        let album = info(&tags).album;
        assert_eq!(album.display_artist, "A");
        assert_eq!(album.artists, [Credit::new("A", Some("えー"))]);
    }

    #[test]
    fn album_artists_by_musicbrainz_id() {
        let tags = RawTags {
            artist: strings(&["角巻わため feat. Mori Calliope"]),
            artists: strings(&["角巻わため", "Mori Calliope"]),
            album_artist: strings(&["わため"]),
            mb_artist_ids: strings(&["id-a", "id-b"]),
            mb_album_artist_ids: strings(&["id-a"]),
            ..RawTags::default()
        };
        let album = info(&tags).album;
        assert_eq!(names(&album.artists), ["角巻わため"]);
        assert_eq!(album.display_artist, "わため");
    }

    #[test]
    fn album_artist_same_as_artist_uses_track_artists() {
        let tags = RawTags {
            artist: strings(&["A feat. B"]),
            artists: strings(&["A", "B"]),
            album_artist: strings(&["A  FEAT. B"]),
            ..RawTags::default()
        };
        let album = info(&tags).album;
        assert_eq!(names(&album.artists), ["A", "B"]);
        assert_eq!(album.display_artist, "A  FEAT. B");
    }

    #[test]
    fn album_artist_differs_from_artist() {
        let tags = RawTags {
            artist: strings(&["supercell"]),
            album_artist: strings(&["〈物語〉シリーズ"]),
            album_artist_sort: Some("ものがたりしりーず".into()),
            ..RawTags::default()
        };
        let album = info(&tags).album;
        assert_eq!(
            album.artists,
            [Credit::new("〈物語〉シリーズ", Some("ものがたりしりーず"))]
        );
    }

    #[test]
    fn no_album_artist_uses_track_artists() {
        let tags = RawTags {
            artist: strings(&["A feat. B"]),
            artists: strings(&["A", "B"]),
            ..RawTags::default()
        };
        let album = info(&tags).album;
        assert_eq!(names(&album.artists), ["A", "B"]);
        assert_eq!(album.display_artist, "A feat. B");
    }

    #[test]
    fn match_key_absorbs_notation() {
        let base = RawTags {
            title: Some("border".into()),
            artist: strings(&["ClariS"]),
            album: Some("Fairy Castle".into()),
            disc_number: Some(1),
            track_number: Some(2),
            ..RawTags::default()
        };
        let varied = RawTags {
            title: Some(" ＢＯＲＤＥＲ ".into()),
            artist: strings(&["ＣｌａｒｉＳ"]),
            album: Some("Fairy  Castle".into()),
            ..base.clone()
        };
        assert_eq!(info(&base).match_key, info(&varied).match_key);
        assert_eq!(info(&base).album.match_key, info(&varied).album.match_key);
    }

    #[test]
    fn match_key_uses_artist_list() {
        // ARTIST の連結表記が違っても、ARTISTS が同じなら同じ曲
        let a = RawTags {
            title: Some("曲".into()),
            artist: strings(&["A feat. B"]),
            artists: strings(&["A", "B"]),
            ..RawTags::default()
        };
        let b = RawTags {
            artist: vec![],
            ..a.clone()
        };
        assert_eq!(info(&a).match_key, info(&b).match_key);
    }

    #[test]
    fn match_key_distinguishes_track_number() {
        let a = RawTags {
            title: Some("曲".into()),
            track_number: Some(1),
            ..RawTags::default()
        };
        let b = RawTags {
            track_number: Some(2),
            ..a.clone()
        };
        assert_ne!(info(&a).match_key, info(&b).match_key);
    }

    #[test]
    fn readings() {
        let tags = RawTags {
            title: Some("キャンディスターにお願い".into()),
            title_sort: Some("きゃんでぃすたーにおねがい".into()),
            album: Some("螺旋の果実".into()),
            album_sort: Some("らせんのかじつ".into()),
            ..RawTags::default()
        };
        let track = info(&tags);
        assert_eq!(
            track.reading.map(|r| r.kana).as_deref(),
            Some("キャンディスターニオネガイ")
        );
        assert_eq!(
            track.album.reading.map(|r| r.kana).as_deref(),
            Some("ラセンノカジツ")
        );
    }
}
