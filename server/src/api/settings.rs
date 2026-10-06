//! Web から変える設定の API（docs/server.md の「設定」）。

use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::AppState;
use super::session;
use crate::settings::{Settings, UpdateError};

/// 間隔は秒で受け渡す。
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Body {
    scan_interval: u64,
    backup_interval: u64,
    backup_keep: u32,
    split_characters: bool,
    spotify_client_id: Option<String>,
}

impl From<Settings> for Body {
    fn from(s: Settings) -> Self {
        Self {
            scan_interval: s.scan_interval.as_secs(),
            backup_interval: s.backup_interval.as_secs(),
            backup_keep: s.backup_keep,
            split_characters: s.split_characters,
            spotify_client_id: s.spotify_client_id,
        }
    }
}

impl From<Body> for Settings {
    fn from(b: Body) -> Self {
        Self {
            scan_interval: Duration::from_secs(b.scan_interval),
            backup_interval: Duration::from_secs(b.backup_interval),
            backup_keep: b.backup_keep,
            split_characters: b.split_characters,
            // 空欄は設定しないものとして扱う
            spotify_client_id: b
                .spotify_client_id
                .map(|id| id.trim().to_owned())
                .filter(|id| !id.is_empty()),
        }
    }
}

pub(super) async fn get(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err(status) = session::require(&state, &headers).await {
        return status.into_response();
    }
    Json(Body::from(state.settings.get())).into_response()
}

/// すべての項目を置き換える。`Json` は `Content-Type: application/json` でなければ 415 で拒む。
pub(super) async fn put(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Body>,
) -> Response {
    if let Err(status) = session::require(&state, &headers).await {
        return status.into_response();
    }
    let settings = Settings::from(body);
    let previous = state.settings.get();
    match state.settings.update(settings.clone()).await {
        Ok(()) => {}
        Err(UpdateError::Invalid(field)) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid", "field": field })),
            )
                .into_response();
        }
        Err(UpdateError::Db(err)) => {
            tracing::error!(%err, "failed to save settings");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    // 接続は Client ID ごとに発行されるので、変えたら接続を切る（docs/spotify.md）
    if previous.spotify_client_id != settings.spotify_client_id
        && let Err(err) = state.spotify.disconnect().await
    {
        tracing::error!(%err, "failed to disconnect spotify");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    tracing::info!(?settings, "settings updated");
    Json(Body::from(settings)).into_response()
}
