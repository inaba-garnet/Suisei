//! 閲覧のエンドポイント。

use std::time::UNIX_EPOCH;

use serde_json::{Map, Value, json};

use super::{AppState, payload};
use crate::db::browse::{self, ArtistEntry};
use crate::subsonic::{Error, ErrorCode, Params};
use crate::tags::index_heading;

/// 閲覧のエンドポイントでなければ `None` を返す。
pub async fn respond(
    name: &str,
    params: &Params,
    state: &AppState,
) -> Option<Result<Map<String, Value>, Error>> {
    let result = match name {
        "getArtists" => artists(state).await,
        "getIndexes" => indexes(params, state).await,
        _ => return None,
    };
    Some(result.map_err(|err| {
        tracing::error!(endpoint = name, error = %err, "db error");
        Error::new(ErrorCode::Generic, "database error")
    }))
}

async fn artists(state: &AppState) -> Result<Map<String, Value>, sqlx::Error> {
    let entries = browse::album_artists(&state.db).await?;
    let index = group(&entries, |a| {
        let mut artist = json!({ "id": a.id, "name": a.name, "albumCount": a.album_count });
        if let Some(sort_name) = &a.sort_name {
            artist["sortName"] = json!(sort_name);
        }
        artist
    });
    Ok(payload(json!({ "artists": {
        "ignoredArticles": "",
        "lastModified": last_modified(state),
        "index": index,
    }})))
}

/// 音楽フォルダが一つなので、getArtists と同じアーティストを返す。
/// `ifModifiedSince` が最後のスキャンより後なら、変わっていないので中身を返さない（Subsonic の仕様）。
async fn indexes(params: &Params, state: &AppState) -> Result<Map<String, Value>, sqlx::Error> {
    let last_modified = last_modified(state);
    let unchanged = params
        .get("ifModifiedSince")
        .and_then(|v| v.parse::<i64>().ok())
        .is_some_and(|since| since >= last_modified);
    let mut indexes = json!({ "ignoredArticles": "", "lastModified": last_modified });
    if !unchanged {
        let entries = browse::album_artists(&state.db).await?;
        indexes["index"] = json!(group(&entries, |a| json!({ "id": a.id, "name": a.name })));
    }
    Ok(payload(json!({ "indexes": indexes })))
}

/// 並べ替えキーの順に並んだアーティストを、見出しごとにまとめる。
fn group(entries: &[ArtistEntry], artist: impl Fn(&ArtistEntry) -> Value) -> Vec<Value> {
    let mut index: Vec<(String, Vec<Value>)> = Vec::new();
    for entry in entries {
        let heading = index_heading(&entry.sort_key);
        match index.last_mut() {
            Some((last, artists)) if *last == heading => artists.push(artist(entry)),
            _ => index.push((heading, vec![artist(entry)])),
        }
    }
    index
        .into_iter()
        .map(|(name, artists)| json!({ "name": name, "artist": artists }))
        .collect()
}

/// 最後にスキャンした時刻（UNIX 時刻のミリ秒）。まだなら 0。
fn last_modified(state: &AppState) -> i64 {
    state
        .scanner
        .status()
        .last_scan
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
