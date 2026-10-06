//! Web に返すスキャンの状態と、最後のスキャンの結果（docs/schema.md の「スキャン」）。

use std::sync::Arc;
use std::time::UNIX_EPOCH;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::AppState;
use super::session;
use crate::scan::{Mode, Report};

pub(super) async fn get(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err(status) = session::require(&state, &headers).await {
        return status.into_response();
    }
    let status = state.scanner.status();
    Json(json!({
        "scanning": status.scanning,
        "count": status.count,
        "last": state.scanner.report().map(|r| report(&r)),
    }))
    .into_response()
}

fn report(r: &Report) -> Value {
    let at = r
        .finished_at
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let mut value = json!({
        "at": at,
        "full": r.mode == Mode::Full,
        "elapsedMs": r.elapsed.as_millis(),
    });
    match &r.result {
        Ok(s) => {
            value["files"] = json!(s.files);
            value["read"] = json!(s.read);
            value["failed"] = json!(s.failed);
            value["failedPaths"] = json!(s.failed_paths);
            value["tracks"] = json!(s.tracks);
            value["albums"] = json!(s.albums);
            value["artists"] = json!(s.artists);
        }
        Err(err) => value["error"] = json!(err),
    }
    value
}
