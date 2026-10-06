//! Spotify の曲とローカルの曲の照合（docs/spotify.md の「曲の対応」）。

use std::collections::HashMap;

use crate::db::spotify::{LocalTrack, MatchMethod, SpotifyTrack};
use crate::tags::{self, ExternalTrackKeys};

/// 曲名とアーティストだけで照らすときの、長さの差の上限。
const DURATION_TOLERANCE_MS: i64 = 3000;

/// ローカルの曲を鍵で引く表。
#[derive(Debug, Default)]
pub struct Index {
    by_isrc: HashMap<String, Vec<usize>>,
    by_match_key: HashMap<String, Vec<usize>>,
    by_loose_key: HashMap<String, Vec<usize>>,
    tracks: Vec<LocalTrack>,
}

impl Index {
    pub fn new(tracks: Vec<LocalTrack>) -> Self {
        let mut index = Self::default();
        for (i, t) in tracks.iter().enumerate() {
            for isrc in &t.isrcs {
                index
                    .by_isrc
                    .entry(normalize_isrc(isrc))
                    .or_default()
                    .push(i);
            }
            index
                .by_match_key
                .entry(t.match_key.clone())
                .or_default()
                .push(i);
            let loose = tags::loose_key(&t.title, t.artist_keys.iter().map(String::as_str));
            index.by_loose_key.entry(loose).or_default().push(i);
        }
        index.tracks = tracks;
        index
    }

    /// ISRC、`match_key`、曲名とアーティストと長さの順に探し、最初に一曲に決まったものを返す。
    /// どの段でも一曲に決まらなければ None。
    pub fn find(
        &self,
        track: &SpotifyTrack,
        options: tags::Options,
    ) -> Option<(&str, MatchMethod)> {
        if let Some(isrc) = &track.isrc
            && let Some(i) = only(self.by_isrc.get(&normalize_isrc(isrc)))
        {
            return Some((&self.tracks[i].id, MatchMethod::Isrc));
        }
        let keys = ExternalTrackKeys::new(
            &track.title,
            &track.artists,
            &track.album,
            track.disc_number.and_then(|n| u32::try_from(n).ok()),
            track.track_number.and_then(|n| u32::try_from(n).ok()),
            options,
        );
        if let Some(i) = only(self.by_match_key.get(&keys.match_key)) {
            return Some((&self.tracks[i].id, MatchMethod::MatchKey));
        }
        let near: Vec<usize> = self
            .by_loose_key
            .get(&keys.loose_key)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&i| {
                (self.tracks[i].duration_ms - track.duration_ms).abs() <= DURATION_TOLERANCE_MS
            })
            .collect();
        only(Some(&near)).map(|i| (self.tracks[i].id.as_str(), MatchMethod::Fuzzy))
    }
}

fn only(candidates: Option<&Vec<usize>>) -> Option<usize> {
    match candidates.map(Vec::as_slice) {
        Some([i]) => Some(*i),
        _ => None,
    }
}

/// `JP-XX0-15-00001` のように区切って書かれた ISRC も同じものにする。
fn normalize_isrc(isrc: &str) -> String {
    isrc.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(id: &str, title: &str, artist: &str, album: &str, track: u32, ms: i64) -> LocalTrack {
        let keys = ExternalTrackKeys::new(
            title,
            &[artist.to_owned()],
            album,
            Some(1),
            Some(track),
            tags::Options::default(),
        );
        LocalTrack {
            id: id.into(),
            title: title.into(),
            match_key: keys.match_key,
            artist_keys: vec![tags::normalize(artist)],
            duration_ms: ms,
            isrcs: Vec::new(),
        }
    }

    fn spotify(title: &str, artist: &str, album: &str, track: i64, ms: i64) -> SpotifyTrack {
        SpotifyTrack {
            spotify_id: "s".into(),
            isrc: None,
            title: title.into(),
            artists: vec![artist.into()],
            album: album.into(),
            disc_number: Some(1),
            track_number: Some(track),
            duration_ms: ms,
            added_at: 0,
        }
    }

    fn find(index: &Index, track: &SpotifyTrack) -> Option<(String, MatchMethod)> {
        index
            .find(track, tags::Options::default())
            .map(|(id, method)| (id.to_owned(), method))
    }

    #[test]
    fn isrc_first() {
        let mut a = local("tr-a", "曲", "歌手", "アルバム", 1, 200_000);
        a.isrcs = vec!["JPXX01500001".into()];
        let index = Index::new(vec![a, local("tr-b", "曲", "歌手", "アルバム", 1, 200_000)]);
        let mut s = spotify("別名", "別人", "別盤", 9, 1);
        s.isrc = Some("jp-xx0-15-00001".into());
        assert_eq!(find(&index, &s), Some(("tr-a".into(), MatchMethod::Isrc)));
    }

    #[test]
    fn same_release_by_match_key() {
        // ISRC が同じ曲が二つ（アルバムとベスト盤）あれば、盤まで一致する曲を採る
        let mut a = local("tr-a", "曲", "歌手", "アルバム", 1, 200_000);
        a.isrcs = vec!["JPXX01500001".into()];
        let mut b = local("tr-b", "曲", "歌手", "ベスト", 5, 200_000);
        b.isrcs = vec!["JPXX01500001".into()];
        let index = Index::new(vec![a, b]);
        let mut s = spotify("曲", "歌手", "ベスト", 5, 201_000);
        s.isrc = Some("JPXX01500001".into());
        assert_eq!(
            find(&index, &s),
            Some(("tr-b".into(), MatchMethod::MatchKey))
        );
    }

    #[test]
    fn other_release_by_title_artist_and_duration() {
        let index = Index::new(vec![local("tr-a", "曲", "歌手", "アルバム", 3, 200_000)]);
        let s = spotify("曲", "歌手", "シングル", 1, 202_500);
        assert_eq!(find(&index, &s), Some(("tr-a".into(), MatchMethod::Fuzzy)));
        // 長さが離れていれば別の録音とみなす
        let live = spotify("曲", "歌手", "ライブ", 1, 260_000);
        assert_eq!(find(&index, &live), None);
    }

    #[test]
    fn ambiguous_is_not_matched() {
        let index = Index::new(vec![
            local("tr-a", "曲", "歌手", "アルバム", 3, 200_000),
            local("tr-b", "曲", "歌手", "ベスト", 1, 201_000),
        ]);
        let s = spotify("曲", "歌手", "シングル", 1, 200_500);
        assert_eq!(find(&index, &s), None);
    }
}
