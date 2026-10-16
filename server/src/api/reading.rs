//! Web クライアントから読みを手動で直す API（docs/schema.md の「日本語の並べ替え」）。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};

use super::AppState;
use super::session;
use crate::db::reading::{self, Entry, Filter, Kind};
use crate::tags::manual_reading;

/// 一覧の一度に返す数の既定と上限。
const DEFAULT_SIZE: i64 = 50;
const MAX_SIZE: i64 = 500;

fn internal(err: impl std::fmt::Display) -> Response {
    tracing::error!(%err, "reading api failed");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

fn to_json(entry: Entry) -> Value {
    json!({
        "id": entry.id,
        "name": entry.name,
        "artist": entry.artist,
        "reading": entry.reading,
        "source": entry.source,
        "autoReading": entry.auto_reading,
        "autoSource": entry.auto_source,
    })
}

#[derive(Debug, Deserialize)]
pub(super) struct ListQuery {
    /// `artist`、`album`、`track`
    kind: String,
    /// `none`（読みがない）か、出どころ（`manual`、`sort_tag`、`name`、`estimated`）。なければすべて
    source: Option<String>,
    offset: Option<i64>,
    size: Option<i64>,
}

/// 読みを直す対象を、読みの順に返す。
pub(super) async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(status) = session::require(&state, &headers).await {
        return status.into_response();
    }
    let kind = match query.kind.as_str() {
        "artist" => Kind::Artist,
        "album" => Kind::Album,
        "track" => Kind::Track,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let filter = match query.source.as_deref() {
        None => Filter::All,
        Some("none") => Filter::None,
        Some(source) => Filter::Source(source),
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let size = query.size.unwrap_or(DEFAULT_SIZE).clamp(1, MAX_SIZE);
    match reading::list(&state.db, kind, filter, offset, size).await {
        Ok(entries) => {
            let items: Vec<Value> = entries.into_iter().map(to_json).collect();
            Json(json!({ "items": items })).into_response()
        }
        Err(err) => internal(err),
    }
}

/// 一件の読みと出どころ、自動の読みを返す。
pub(super) async fn get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(status) = session::require(&state, &headers).await {
        return status.into_response();
    }
    match reading::find(&state.db, &id).await {
        Ok(Some(entry)) => Json(to_json(entry)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => internal(err),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct Body {
    /// None なら手動の読みを外し、自動の読みに戻す
    reading: Option<String>,
}

/// 手動の読みを設定する、外す。`Json` は `Content-Type: application/json` でなければ 415 で拒む。
pub(super) async fn put(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Body>,
) -> Response {
    if let Err(status) = session::require(&state, &headers).await {
        return status.into_response();
    }
    let reading = match body.reading.as_deref().map(manual_reading) {
        None => None,
        Some(Some(reading)) => Some(reading),
        Some(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalidReading" })),
            )
                .into_response();
        }
    };
    // スキャンが始めに読んだ状態で書き戻して、直した読みを消さないよう、スキャンを止めて書く
    let _hold = state.scanner.hold().await;
    match reading::set(&state.db, &id, reading.as_deref()).await {
        Ok(Some(entry)) => {
            tracing::info!(id, reading = entry.reading, "reading updated");
            Json(to_json(entry)).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => internal(err),
    }
}
