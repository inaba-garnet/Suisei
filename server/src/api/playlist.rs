//! プレイリストのエンドポイント（docs/schema.md の「プレイリスト」）。

use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::browse::{db_error, seconds, timestamp};
use super::search::songs;
use super::{AppState, payload};
use crate::db::playlist::{self, Changes, Playlist, STARRED_ID};
use crate::db::{annotation, browse};
use crate::subsonic::{Error, ErrorCode, Params};

fn missing(key: &str) -> Error {
    Error::new(
        ErrorCode::MissingParameter,
        format!("required parameter is missing: {key}"),
    )
}

fn not_found() -> Error {
    Error::new(ErrorCode::NotFound, "playlist not found")
}

/// お気に入りのプレイリストは変えられない（docs/schema.md の「お気に入りのプレイリスト」）。
fn readonly() -> Error {
    Error::new(ErrorCode::NotAuthorized, "playlist is read-only")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

pub async fn list(state: &AppState) -> Result<Map<String, Value>, Error> {
    // お気に入りのプレイリストを先に置く
    let mut playlists = vec![
        playlist::starred(&state.db, now())
            .await
            .map_err(db_error)?,
    ];
    playlists.extend(playlist::list(&state.db).await.map_err(db_error)?);
    let list: Vec<Value> = playlists.iter().map(|p| playlist_json(state, p)).collect();
    Ok(payload(json!({ "playlists": { "playlist": list } })))
}

pub async fn get(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let id = params.get("id").ok_or_else(|| missing("id"))?;
    with_entries(state, id).await
}

/// `playlistId` があれば、そのプレイリストの曲を置き換える（Subsonic の仕様）。
pub async fn create(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let tracks = track_ids(state, params.get_all("songId"), "createPlaylist").await?;
    let now = now();
    let id = match params.get("playlistId") {
        Some(STARRED_ID) => return Err(readonly()),
        Some(id) => {
            let changes = Changes {
                name: params.get("name").map(str::to_owned),
                ..Changes::default()
            };
            if !playlist::update(&state.db, id, &changes, Some(&tracks), now)
                .await
                .map_err(db_error)?
            {
                return Err(not_found());
            }
            id.to_owned()
        }
        None => {
            let name = params.get("name").ok_or_else(|| missing("name"))?;
            let id = playlist::create(&state.db, name, now)
                .await
                .map_err(db_error)?;
            playlist::update(&state.db, &id, &Changes::default(), Some(&tracks), now)
                .await
                .map_err(db_error)?;
            id
        }
    };
    with_entries(state, &id).await
}

/// 削除を先に、追加を後に行う。`songIndexToRemove` は 0 始まりの位置（Navidrome と同じ）。
pub async fn update(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let id = params
        .get("playlistId")
        .ok_or_else(|| missing("playlistId"))?;
    if id == STARRED_ID {
        return Err(readonly());
    }
    let changes = Changes {
        name: params.get("name").map(str::to_owned),
        comment: params.get("comment").map(str::to_owned),
        public: params.get("public").map(|p| p == "true"),
    };
    let remove: BTreeSet<usize> = params
        .get_all("songIndexToRemove")
        .filter_map(|i| i.parse().ok())
        .collect();
    let add = track_ids(state, params.get_all("songIdToAdd"), "updatePlaylist").await?;

    let tracks = if remove.is_empty() && add.is_empty() {
        None
    } else {
        let mut tracks: Vec<String> = playlist::entries(&state.db, id)
            .await
            .map_err(db_error)?
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !remove.contains(i))
            .map(|(_, track_id)| track_id)
            .collect();
        tracks.extend(add);
        Some(tracks)
    };
    if !playlist::update(&state.db, id, &changes, tracks.as_deref(), now())
        .await
        .map_err(db_error)?
    {
        return Err(not_found());
    }
    Ok(Map::new())
}

pub async fn delete(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let id = params.get("id").ok_or_else(|| missing("id"))?;
    if id == STARRED_ID {
        return Err(readonly());
    }
    if !playlist::delete(&state.db, id).await.map_err(db_error)? {
        return Err(not_found());
    }
    Ok(Map::new())
}

/// 別名を引き直し、ライブラリにない曲は飛ばす。順序と重複は保つ。
async fn track_ids(
    state: &AppState,
    ids: impl Iterator<Item = &str>,
    endpoint: &str,
) -> Result<Vec<String>, Error> {
    let mut resolved = Vec::new();
    for id in ids {
        resolved.push(
            browse::resolve_alias(&state.db, id)
                .await
                .map_err(db_error)?,
        );
    }
    let found = playlist::existing_tracks(&state.db, &resolved)
        .await
        .map_err(db_error)?;
    if found.len() < resolved.len() {
        // エラーにすると、クライアントが操作を再送し続けるおそれがある
        let skipped: Vec<&String> = resolved.iter().filter(|id| !found.contains(id)).collect();
        tracing::warn!(endpoint, ?skipped, "unknown song ids skipped");
    }
    Ok(found)
}

/// プレイリストと、その曲。
async fn with_entries(state: &AppState, id: &str) -> Result<Map<String, Value>, Error> {
    let (playlist, ids) = if id == STARRED_ID {
        let playlist = playlist::starred(&state.db, now())
            .await
            .map_err(db_error)?;
        let ids = annotation::starred_songs(&state.db)
            .await
            .map_err(db_error)?;
        (playlist, ids)
    } else {
        let playlist = playlist::get(&state.db, id)
            .await
            .map_err(db_error)?
            .ok_or_else(not_found)?;
        let ids = playlist::entries(&state.db, id).await.map_err(db_error)?;
        (playlist, ids)
    };
    let mut value = playlist_json(state, &playlist);
    value["entry"] = json!(songs(state, &ids).await?);
    Ok(payload(json!({ "playlist": value })))
}

fn playlist_json(state: &AppState, p: &Playlist) -> Value {
    let mut value = json!({
        "id": p.id,
        "name": p.name,
        "owner": state.credentials.user,
        "public": p.public,
        "songCount": p.song_count,
        "duration": seconds(p.duration_ms),
        "created": timestamp(p.created_at),
        "changed": timestamp(p.changed_at),
        "readonly": p.id == STARRED_ID,
    });
    if let Some(comment) = &p.comment {
        value["comment"] = json!(comment);
    }
    if let Some(album_id) = &p.cover_album_id {
        value["coverArt"] = json!(album_id);
    }
    value
}
