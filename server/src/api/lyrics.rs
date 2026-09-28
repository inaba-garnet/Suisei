//! 歌詞のエンドポイント（docs/schema.md の「歌詞」）。

use serde_json::{Map, Value, json};

use super::browse::db_error;
use super::media::resolve;
use super::{AppState, payload};
use crate::db::browse;
use crate::subsonic::{Error, ErrorCode, Params};
use crate::tags::lyrics::{self, Lyrics};

/// OpenSubsonic の `getLyricsBySongId`。歌詞がなければ `structuredLyrics` を省く。
pub async fn by_song_id(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let id = params.get("id").ok_or_else(|| {
        Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: id",
        )
    })?;
    let id = browse::resolve_alias(&state.db, id)
        .await
        .map_err(db_error)?;
    let song = browse::song(&state.db, &id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| Error::new(ErrorCode::NotFound, "song not found"))?;
    let found = read(state, &song.path).await;
    if found.is_empty() {
        return Ok(payload(json!({ "lyricsList": {} })));
    }
    let list: Vec<Value> = found
        .iter()
        .map(|l| structured(l, &song.title, &song.display_artist))
        .collect();
    Ok(payload(
        json!({ "lyricsList": { "structuredLyrics": list } }),
    ))
}

/// Subsonic の `getLyrics`。アーティスト名と曲名で曲を引き、時刻なしの本文を一つ返す。
/// 見つからなければ中身のない `lyrics` を返す（Subsonic の仕様）。
pub async fn by_name(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let empty = || Ok(payload(json!({ "lyrics": {} })));
    let Some(title) = params.get("title") else {
        return empty();
    };
    let Some(song) = browse::song_by_name(&state.db, params.get("artist"), title)
        .await
        .map_err(db_error)?
    else {
        return empty();
    };
    let found = read(state, &song.path).await;
    // 時刻なしを優先し、なければ時刻付きの本文をつなげる
    let Some(lyrics) = found.iter().find(|l| !l.synced).or(found.first()) else {
        return empty();
    };
    let text: Vec<&str> = lyrics.lines.iter().map(|l| l.value.as_str()).collect();
    Ok(payload(json!({ "lyrics": {
        "artist": song.display_artist,
        "title": song.title,
        "value": text.join("\n"),
    }})))
}

async fn read(state: &AppState, rel: &str) -> Vec<Lyrics> {
    let Some(path) = resolve(state.scanner.music_dir(), rel) else {
        return Vec::new();
    };
    tokio::task::spawn_blocking(move || lyrics::read(&path))
        .await
        .expect("歌詞を読むタスクが panic した")
}

fn structured(lyrics: &Lyrics, title: &str, artist: &str) -> Value {
    let lines: Vec<Value> = lyrics
        .lines
        .iter()
        .map(|line| {
            let mut value = json!({ "value": line.value });
            if let Some(start) = line.start_ms {
                value["start"] = json!(start);
            }
            value
        })
        .collect();
    json!({
        "displayArtist": artist,
        "displayTitle": title,
        "lang": lyrics.lang,
        "offset": lyrics.offset_ms,
        "synced": lyrics.synced,
        "line": lines,
    })
}
