//! お気に入りと評価のエンドポイント（docs/schema.md の「お気に入りと評価」）。

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::browse::{album_json, artist_summary_json, db_error};
use super::search::songs;
use super::{AppState, payload};
use crate::db::{annotation, browse, search};
use crate::subsonic::{Error, ErrorCode, Params};

/// `id`（曲）、`albumId`、`artistId` を送られた順に集める。ID は種類をまたいで一意なので区別しない。
fn ids(params: &Params) -> Vec<&str> {
    ["id", "albumId", "artistId"]
        .into_iter()
        .flat_map(|key| params.get_all(key))
        .collect()
}

pub async fn star(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    for id in ids(params) {
        let id = browse::resolve_alias(&state.db, id)
            .await
            .map_err(db_error)?;
        if !annotation::star(&state.db, &id, now)
            .await
            .map_err(db_error)?
        {
            skipped("star", &id);
        }
    }
    Ok(Map::new())
}

pub async fn unstar(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    for id in ids(params) {
        let id = browse::resolve_alias(&state.db, id)
            .await
            .map_err(db_error)?;
        if !annotation::unstar(&state.db, &id).await.map_err(db_error)? {
            skipped("unstar", &id);
        }
    }
    Ok(Map::new())
}

/// `rating=0` は評価を消す。
pub async fn set_rating(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let missing = |key: &str| {
        Error::new(
            ErrorCode::MissingParameter,
            format!("required parameter is missing: {key}"),
        )
    };
    let id = params.get("id").ok_or_else(|| missing("id"))?;
    let rating = params.get("rating").ok_or_else(|| missing("rating"))?;
    let rating = match rating.parse::<i64>() {
        Ok(0) => None,
        Ok(r @ 1..=5) => Some(r),
        _ => {
            return Err(Error::new(
                ErrorCode::Generic,
                format!("rating must be between 0 and 5: {rating}"),
            ));
        }
    };
    let id = browse::resolve_alias(&state.db, id)
        .await
        .map_err(db_error)?;
    if !annotation::set_rating(&state.db, &id, rating)
        .await
        .map_err(db_error)?
    {
        skipped("setRating", &id);
    }
    Ok(Map::new())
}

/// エラーにすると、クライアントがオフラインで溜めた操作を再送し続けるおそれがある。
fn skipped(endpoint: &str, id: &str) {
    tracing::warn!(endpoint, id, "unknown id skipped");
}

/// お気に入りのアーティスト、アルバム、曲を、お気に入りにした新しい順に返す。
/// 曲は独自の引数 `songSort` で並べ替えられる（docs/schema.md の「お気に入りと評価」）。
pub async fn starred2(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let sort = super::search::song_sort(params)?;
    let artists: Vec<Value> = annotation::starred_artists(&state.db)
        .await
        .map_err(db_error)?
        .iter()
        .map(artist_summary_json)
        .collect();
    let mut albums = Vec::new();
    for id in annotation::starred_albums(&state.db)
        .await
        .map_err(db_error)?
    {
        if let Some(album) = browse::album(&state.db, &id).await.map_err(db_error)? {
            albums.push(album_json(state, &album).await?);
        }
    }
    let song_ids = match sort {
        Some(sort) => search::starred_songs(&state.db, sort).await,
        None => annotation::starred_songs(&state.db).await,
    }
    .map_err(db_error)?;
    let songs = songs(state, &song_ids).await?;

    // 空の種類は、Navidrome と同じく項目ごと省く
    let mut starred = Map::new();
    for (key, values) in [("artist", artists), ("album", albums), ("song", songs)] {
        if !values.is_empty() {
            starred.insert(key.into(), Value::Array(values));
        }
    }
    Ok(payload(json!({ "starred2": starred })))
}
