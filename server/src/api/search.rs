//! 検索のエンドポイント。

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use super::browse::{album_json, artist_summary_json, db_error, songs_json};
use super::{AppState, payload};
use crate::db::{browse, search};
use crate::subsonic::{Error, ErrorCode, Params};
use crate::tags::search_form;

/// Subsonic の件数の既定値
const COUNT_DEFAULT: i64 = 20;

pub async fn search3(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let query = params.get("query").ok_or_else(|| {
        Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: query",
        )
    })?;
    let words = words(query);
    let sort = song_sort(params)?;
    let page = |kind: &str| {
        let number = |key: String| params.get(&key).and_then(|v| v.parse::<i64>().ok());
        let count = number(format!("{kind}Count"))
            .unwrap_or(COUNT_DEFAULT)
            .max(0);
        let offset = number(format!("{kind}Offset")).unwrap_or(0).max(0);
        (count, offset)
    };

    let mut result = Map::new();
    let (count, offset) = page("artist");
    if count > 0 {
        let hits = search::artists(&state.db, &words, count, offset)
            .await
            .map_err(db_error)?;
        let artists: Vec<Value> = hits.iter().map(artist_summary_json).collect();
        insert(&mut result, "artist", artists);
    }

    let (count, offset) = page("album");
    if count > 0 {
        let ids = search::albums(&state.db, &words, count, offset)
            .await
            .map_err(db_error)?;
        let mut albums = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Some(album) = browse::album(&state.db, id).await.map_err(db_error)? {
                albums.push(album_json(state, &album).await?);
            }
        }
        insert(&mut result, "album", albums);
    }

    let (count, offset) = page("song");
    if count > 0 {
        let ids = search::songs(&state.db, &words, sort, count, offset)
            .await
            .map_err(db_error)?;
        insert(&mut result, "song", songs(state, &ids).await?);
    }

    Ok(payload(json!({ "searchResult3": result })))
}

/// 独自の引数 `songSort` で曲の順を変える（docs/schema.md の「検索」）。
pub(super) fn song_sort(params: &Params) -> Result<search::SongSort, Error> {
    match params.get("songSort") {
        None | Some("title") => Ok(search::SongSort::Title),
        Some("album") => Ok(search::SongSort::Album),
        Some("artist") => Ok(search::SongSort::Artist),
        Some(other) => Err(Error::new(
            ErrorCode::Generic,
            format!("unknown songSort: {other}"),
        )),
    }
}

/// 結果が空の種類は、Navidrome と同じく項目ごと省く。
fn insert(result: &mut Map<String, Value>, key: &str, values: Vec<Value>) {
    if !values.is_empty() {
        result.insert(key.into(), Value::Array(values));
    }
}

/// 検索語を語に分ける。Symfonium は全件を `query=""`（引用符二つ）で、substreamer は空で求めるので、
/// 引用符を除いて空になれば語なし（全件）とする。
fn words(query: &str) -> Vec<String> {
    let query = query.trim().trim_matches('"');
    search_form(query)
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// 曲を順序どおりに Child にする。アーティストとジャンルはアルバムごとにまとめて読む。
pub(super) async fn songs(state: &AppState, ids: &[String]) -> Result<Vec<Value>, Error> {
    let mut by_album: HashMap<String, Vec<(usize, browse::Song)>> = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        if let Some(song) = browse::song(&state.db, id).await.map_err(db_error)? {
            by_album
                .entry(song.album_id.clone())
                .or_default()
                .push((i, song));
        }
    }
    let mut slots: Vec<Option<Value>> = vec![None; ids.len()];
    for (album_id, entries) in by_album {
        let (indexes, songs): (Vec<usize>, Vec<browse::Song>) = entries.into_iter().unzip();
        let values = songs_json(state, &album_id, &songs).await?;
        for (i, value) in indexes.into_iter().zip(values) {
            slots[i] = Some(value);
        }
    }
    Ok(slots.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_queries_mean_everything() {
        assert!(words("").is_empty());
        assert!(words("\"\"").is_empty());
        assert!(words("  ").is_empty());
    }

    #[test]
    fn words_are_normalized() {
        assert_eq!(words("  ＣｌａｒｉＳ  よねず "), ["claris", "ヨネズ"]);
    }
}
