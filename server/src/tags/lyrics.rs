//! 歌詞の読み取り（docs/schema.md の「歌詞」）。求められたときに、その曲のファイルを読む。

use std::fs::File;
use std::path::Path;

use lofty::config::ParseOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::id3::v2::{Frame, SynchronizedTextFrame, TimestampFormat};
use lofty::mpeg::MpegFile;
use lofty::tag::ItemKey;

/// 言語が分からないときのコード（ISO 639-2）。
const UNDETERMINED: &str = "und";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lyrics {
    /// ISO 639-2 の言語コード。分からなければ `und`
    pub lang: String,
    pub synced: bool,
    /// 表示を早める時間（ミリ秒）。LRC の `[offset:]` と同じ向き
    pub offset_ms: i64,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// 時刻付きの歌詞でだけ持つ（ミリ秒）
    pub start_ms: Option<u64>,
    pub value: String,
}

/// 音声と同じ名前の `.lrc` と、埋め込みの歌詞をすべて読む。時刻付きを先に並べる。
pub fn read(path: &Path) -> Vec<Lyrics> {
    let mut found = Vec::new();
    for ext in ["lrc", "LRC"] {
        if let Ok(bytes) = std::fs::read(path.with_extension(ext)) {
            found.extend(parse(&decode(&bytes), UNDETERMINED));
            break;
        }
    }
    found.extend(embedded(path));
    let mut found = dedup(found);
    // 並べ替えは安定なので、同じ種類の中では見つけた順を保つ
    found.sort_by_key(|l| !l.synced);
    found
}

/// 本文と時刻が同じ歌詞を一つにまとめ、言語が分かっているほうの言語を残す。
/// 言語だけ違う `USLT` を二つ埋め込んだ MP3 があるため。
fn dedup(found: Vec<Lyrics>) -> Vec<Lyrics> {
    let mut unique: Vec<Lyrics> = Vec::with_capacity(found.len());
    for lyrics in found {
        match unique
            .iter_mut()
            .find(|u| u.synced == lyrics.synced && u.lines == lyrics.lines)
        {
            Some(same) if same.lang == UNDETERMINED => same.lang = lyrics.lang,
            Some(_) => {}
            None => unique.push(lyrics),
        }
    }
    unique
}

/// BOM があればそれに従い、なければ UTF-8、読めなければ Shift_JIS として読む。
fn decode(bytes: &[u8]) -> String {
    if let Some((encoding, bom)) = encoding_rs::Encoding::for_bom(bytes) {
        return encoding
            .decode_without_bom_handling(&bytes[bom..])
            .0
            .into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}

fn embedded(path: &Path) -> Vec<Lyrics> {
    let is_mp3 = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("mp3"));
    if is_mp3 {
        return id3v2(path).unwrap_or_default();
    }
    let Ok(file) = lofty::read_from_path(path) else {
        return Vec::new();
    };
    let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) else {
        return Vec::new();
    };
    tag.items()
        .filter(|item| matches!(item.key(), ItemKey::Lyrics | ItemKey::UnsyncLyrics))
        .filter_map(|item| {
            let text = item.value().text()?;
            parse(text, &language(item.lang()))
        })
        .collect()
}

/// `SYLT` は汎用のタグに変換されないので、ID3v2 のフレームを直接読む。
fn id3v2(path: &Path) -> Option<Vec<Lyrics>> {
    let mut file = File::open(path).ok()?;
    let mpeg = MpegFile::read_from(&mut file, ParseOptions::new()).ok()?;
    let tag = mpeg.id3v2()?;
    let mut found: Vec<Lyrics> = tag
        .unsync_text()
        .filter_map(|frame| parse(&frame.content, &language(&frame.language)))
        .collect();
    for frame in tag {
        let Frame::Binary(binary) = frame else {
            continue;
        };
        if binary.id().as_str() != "SYLT" {
            continue;
        }
        let Ok(sylt) = SynchronizedTextFrame::parse(&binary.data, binary.flags()) else {
            continue;
        };
        // MPEG のフレーム数で数えた時刻は、ミリ秒に直せないので使わない
        if sylt.timestamp_format != TimestampFormat::MS || sylt.content.is_empty() {
            continue;
        }
        found.push(Lyrics {
            lang: language(&sylt.language),
            synced: true,
            offset_ms: 0,
            lines: sylt
                .content
                .iter()
                .map(|(start, text)| Line {
                    start_ms: Some(u64::from(*start)),
                    value: text.trim_end_matches(['\r', '\n']).to_owned(),
                })
                .collect(),
        });
    }
    Some(found)
}

/// タグの言語コード。空や `XXX` は分からないものとして `und` にする。
fn language(code: &[u8; 3]) -> String {
    match std::str::from_utf8(code) {
        Ok(code) if code.chars().all(|c| c.is_ascii_alphabetic()) && code != "XXX" => {
            code.to_ascii_lowercase()
        }
        _ => UNDETERMINED.to_owned(),
    }
}

/// LRC の形の本文を読む。行頭に時刻があれば時刻付き、なければ時刻なしの歌詞にする。空なら None。
pub fn parse(text: &str, lang: &str) -> Option<Lyrics> {
    let text = text.trim_start_matches('\u{feff}');
    let mut offset_ms = 0;
    let mut synced = Vec::new();
    let mut plain = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim_end_matches('\r');
        let mut starts = Vec::new();
        // 行頭の `[..]` をすべて読む。時刻は一行に複数あることがある
        while let Some(tag) = rest.strip_prefix('[').and_then(|r| r.split_once(']')) {
            let (inside, after) = tag;
            if let Some(ms) = timestamp(inside) {
                starts.push(ms);
            } else if let Some(value) = inside.strip_prefix("offset:") {
                offset_ms = value.trim().parse().unwrap_or(0);
            } else if !is_metadata(inside) {
                break;
            }
            rest = after;
        }
        let value = strip_word_timestamps(rest).trim().to_owned();
        if starts.is_empty() {
            plain.push(value);
        } else {
            synced.extend(starts.into_iter().map(|ms| (ms, value.clone())));
        }
    }
    if !synced.is_empty() {
        synced.sort_by_key(|(ms, _)| *ms);
        return Some(Lyrics {
            lang: lang.to_owned(),
            synced: true,
            offset_ms,
            lines: synced
                .into_iter()
                .map(|(ms, value)| Line {
                    start_ms: Some(ms),
                    value,
                })
                .collect(),
        });
    }
    // 前後の空行は落とし、途中の空行（段落の区切り）は残す
    let first = plain.iter().position(|l| !l.is_empty())?;
    let last = plain.iter().rposition(|l| !l.is_empty())?;
    Some(Lyrics {
        lang: lang.to_owned(),
        synced: false,
        offset_ms: 0,
        lines: plain[first..=last]
            .iter()
            .map(|value| Line {
                start_ms: None,
                value: value.clone(),
            })
            .collect(),
    })
}

/// `mm:ss`、`mm:ss.xx`、`mm:ss.xxx`、`mm:ss:xx` をミリ秒にする。
fn timestamp(text: &str) -> Option<u64> {
    let (minutes, rest) = text.split_once(':')?;
    let minutes: u64 = minutes.parse().ok()?;
    let (seconds, fraction) = match rest.split_once(['.', ':']) {
        Some((s, f)) => (s, f),
        None => (rest, ""),
    };
    let seconds: u64 = seconds.parse().ok()?;
    if seconds >= 60 || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // 小数部は桁数で単位が変わる（.5 は 500 ms、.05 は 50 ms）
    let fraction_ms = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u64>().ok()? * 100,
        2 => fraction.parse::<u64>().ok()? * 10,
        _ => fraction[..3].parse::<u64>().ok()?,
    };
    Some((minutes * 60 + seconds) * 1000 + fraction_ms)
}

/// `[ti:曲名]` のような LRC の情報の行。
fn is_metadata(inside: &str) -> bool {
    inside
        .split_once(':')
        .is_some_and(|(key, _)| !key.is_empty() && key.chars().all(|c| c.is_ascii_alphabetic()))
}

/// 拡張 LRC の単語ごとの時刻（`<mm:ss.xx>`）を除く。
fn strip_word_timestamps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else {
            break;
        };
        out.push_str(&rest[..open]);
        let inside = &rest[open + 1..open + close];
        if timestamp(inside).is_none() {
            out.push_str(&rest[open..=open + close]);
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starts(lyrics: &Lyrics) -> Vec<(Option<u64>, &str)> {
        lyrics
            .lines
            .iter()
            .map(|l| (l.start_ms, l.value.as_str()))
            .collect()
    }

    #[test]
    fn synced_lrc() {
        let text = "\u{feff}[ti:曲]\r\n[ar:歌手]\r\n[offset:+250]\r\n\
                    [00:12.34]一行目\r\n[01:02.5][00:05.00]くり返し\r\n\r\n[00:20.123]<00:20.20>単<00:21.00>語\r\n";
        let lyrics = parse(text, "und").unwrap();
        assert!(lyrics.synced);
        assert_eq!(lyrics.offset_ms, 250);
        assert_eq!(
            starts(&lyrics),
            [
                (Some(5000), "くり返し"),
                (Some(12340), "一行目"),
                (Some(20123), "単語"),
                (Some(62500), "くり返し"),
            ]
        );
    }

    #[test]
    fn plain_text_keeps_paragraph_breaks() {
        let lyrics = parse("\n一番\n\n二番\n\n", "jpn").unwrap();
        assert!(!lyrics.synced);
        assert_eq!(lyrics.lang, "jpn");
        assert_eq!(
            starts(&lyrics),
            [(None, "一番"), (None, ""), (None, "二番")]
        );
    }

    #[test]
    fn brackets_in_lyrics_are_text() {
        let lyrics = parse("[Chorus] 歌う", "und").unwrap();
        assert!(!lyrics.synced);
        assert_eq!(starts(&lyrics), [(None, "[Chorus] 歌う")]);
    }

    #[test]
    fn empty_text_has_no_lyrics() {
        assert_eq!(parse(" \n\r\n", "und"), None);
    }

    #[test]
    fn shift_jis_is_decoded() {
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("[00:01.00]歌詞");
        assert_eq!(decode(&bytes), "[00:01.00]歌詞");
        assert_eq!(decode("\u{feff}歌詞".as_bytes()), "歌詞");
    }

    #[test]
    fn same_text_is_returned_once() {
        let found = vec![
            parse("歌詞", "und").unwrap(),
            parse("歌詞", "jpn").unwrap(),
            parse("[00:01.00]歌詞", "und").unwrap(),
            parse("別の歌詞", "eng").unwrap(),
        ];
        let unique = dedup(found);
        let summary: Vec<(bool, &str, &str)> = unique
            .iter()
            .map(|l| (l.synced, l.lang.as_str(), l.lines[0].value.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                (false, "jpn", "歌詞"),
                (true, "und", "歌詞"),
                (false, "eng", "別の歌詞"),
            ]
        );
    }

    #[test]
    fn language_codes() {
        assert_eq!(language(b"jpn"), "jpn");
        assert_eq!(language(b"ENG"), "eng");
        assert_eq!(language(b"XXX"), "und");
        assert_eq!(language(&[0, 0, 0]), "und");
    }
}
