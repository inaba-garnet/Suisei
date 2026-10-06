//! Web クライアントから使う Spotify 連携の API（docs/spotify.md の「API」）。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};

use super::AppState;
use super::session;
use crate::db;
use crate::spotify::{ConnectError, REDIRECT_URI, Spotify};

/// ログインを確かめ、Spotify 連携が設定されていればそれを返す。
async fn require(state: &AppState, headers: &HeaderMap) -> Result<Arc<Spotify>, StatusCode> {
    session::require(state, headers).await?;
    state.spotify.clone().ok_or(StatusCode::NOT_FOUND)
}

/// 書き込む要求は `Content-Type: application/json` に限る。別のサイトのフォームから送らせないため。
async fn require_json(state: &AppState, headers: &HeaderMap) -> Result<Arc<Spotify>, StatusCode> {
    if !session::is_json(headers) {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    require(state, headers).await
}

fn internal(err: impl std::fmt::Display) -> Response {
    tracing::error!(%err, "spotify api failed");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

pub(super) async fn status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err(status) = session::require(&state, &headers).await {
        return status.into_response();
    }
    let Some(spotify) = &state.spotify else {
        return Json(json!({ "configured": false })).into_response();
    };
    match spotify.status().await {
        Ok(status) => Json(json!({
            "configured": true,
            "redirectUri": REDIRECT_URI,
            "connected": status.connected,
            "syncing": status.syncing,
            "lastSync": status.last_sync.map(|last| json!({
                "at": last.at,
                "error": last.error,
                "fetched": last.fetched,
                "matched": last.matched,
            })),
            "total": status.counts.total,
            "matched": status.counts.matched,
        }))
        .into_response(),
        Err(err) => internal(err),
    }
}

pub(super) async fn authorize(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match require_json(&state, &headers).await {
        Ok(spotify) => Json(json!({ "url": spotify.authorize_url() })).into_response(),
        Err(status) => status.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct Callback {
    url: String,
}

/// `Json` は `Content-Type: application/json` でなければ 415 で拒む。
pub(super) async fn callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Callback>,
) -> Response {
    let spotify = match require(&state, &headers).await {
        Ok(spotify) => spotify,
        Err(status) => return status.into_response(),
    };
    let (status, error) = match spotify.connect(&body.url).await {
        Ok(()) => return StatusCode::NO_CONTENT.into_response(),
        Err(ConnectError::InvalidUrl) => {
            (StatusCode::BAD_REQUEST, json!({ "error": "invalidUrl" }))
        }
        Err(ConnectError::Denied(reason)) => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "denied", "reason": reason }),
        ),
        Err(ConnectError::UnknownState) => {
            (StatusCode::BAD_REQUEST, json!({ "error": "unknownState" }))
        }
        Err(ConnectError::Failed(err)) => {
            tracing::warn!(%err, "failed to connect to spotify");
            (
                StatusCode::BAD_GATEWAY,
                json!({ "error": "spotify", "message": err.to_string() }),
            )
        }
    };
    (status, Json(error)).into_response()
}

pub(super) async fn disconnect(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let spotify = match require_json(&state, &headers).await {
        Ok(spotify) => spotify,
        Err(status) => return status.into_response(),
    };
    match spotify.disconnect().await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal(err),
    }
}

/// 取り込みを始める。走っていれば何もしない。終わるのを待たずに返す。
pub(super) async fn sync(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let spotify = match require_json(&state, &headers).await {
        Ok(spotify) => spotify,
        Err(status) => return status.into_response(),
    };
    spotify.start_sync();
    StatusCode::ACCEPTED.into_response()
}

#[derive(Debug, Deserialize)]
pub(super) struct TracksQuery {
    matched: Option<bool>,
}

pub(super) async fn tracks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TracksQuery>,
) -> Response {
    if let Err(status) = require(&state, &headers).await {
        return status.into_response();
    }
    match db::spotify::tracks(&state.db, query.matched).await {
        Ok(rows) => {
            let tracks: Vec<Value> = rows
                .into_iter()
                .map(|row| {
                    json!({
                        "id": row.track.spotify_id,
                        "title": row.track.title,
                        "artists": row.track.artists,
                        "album": row.track.album,
                        "isrc": row.track.isrc,
                        "durationMs": row.track.duration_ms,
                        "addedAt": row.track.added_at,
                        "trackId": row.track_id,
                        "method": row.match_method,
                    })
                })
                .collect();
            Json(json!({ "tracks": tracks })).into_response()
        }
        Err(err) => internal(err),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Link {
    /// None なら対応を外し、自動の対応も付けない
    track_id: Option<String>,
}

/// 手動で対応を付ける、外す。付けたローカルの曲はお気に入りにする。
pub(super) async fn link(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(spotify_id): Path<String>,
    Json(body): Json<Link>,
) -> Response {
    if let Err(status) = require(&state, &headers).await {
        return status.into_response();
    }
    let method = if body.track_id.is_some() {
        db::spotify::MatchMethod::Manual
    } else {
        db::spotify::MatchMethod::Ignored
    };
    match db::spotify::link(&state.db, &spotify_id, body.track_id.as_deref(), method).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => internal(err),
    }
}
