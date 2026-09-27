mod empty;

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{Method, StatusCode, Uri};
use axum::response::Response;
use axum::routing::any;
use serde_json::{Map, Value, json};

use crate::subsonic::{self, Error, ErrorCode, Format, Params};
use crate::{Config, Credentials};

#[derive(Debug, Clone)]
pub struct AppState {
    pub credentials: Credentials,
}

impl From<Config> for AppState {
    fn from(config: Config) -> Self {
        Self {
            credentials: config.credentials,
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/rest/{endpoint}", any(rest))
        .fallback(not_found)
        .with_state(Arc::new(state))
}

/// Subsonic のエンドポイントは `ping` と `ping.view` のどちらでも呼ばれるので、名前で振り分ける。
async fn rest(
    State(state): State<Arc<AppState>>,
    method: Method,
    Path(endpoint): Path<String>,
    params: Params,
) -> Response {
    let name = endpoint.strip_suffix(".view").unwrap_or(&endpoint);
    let format = Format::from_params(&params);
    tracing::info!(%method, endpoint = name, params = %params.masked(), "request");

    // OpenSubsonic の仕様で、認証なしで呼べることになっている。
    if name != "getOpenSubsonicExtensions"
        && let Err(err) = subsonic::authenticate(&params, &state.credentials)
    {
        tracing::info!(
            endpoint = name,
            code = err.code as u32,
            "authentication failed"
        );
        return subsonic::error(format, &err);
    }

    match name {
        "ping" => subsonic::ok(format, Map::new()),
        "getLicense" => subsonic::ok(format, payload(json!({ "license": { "valid": true } }))),
        "getOpenSubsonicExtensions" => subsonic::ok(format, extensions(&state)),
        _ => match empty::respond(name, &params, &state) {
            Some(Ok(payload)) => subsonic::ok(format, payload),
            Some(Err(err)) => subsonic::error(format, &err),
            None => not_implemented(name, &method, &params, format),
        },
    }
}

/// クライアントの解析中だけの仮の措置。HTTP のエラーではクライアントの同期が止まるおそれがあるので、
/// 200 と Subsonic のエラーを返し、呼ばれたことを warn で残す。
/// MVP の範囲を決めたら恒常的な応答に戻す（https://github.com/inaba-garnet/Suisei/issues/5）。
fn not_implemented(name: &str, method: &Method, params: &Params, format: Format) -> Response {
    tracing::warn!(%method, endpoint = name, params = %params.masked(), "not implemented");
    subsonic::error(
        format,
        &Error::new(ErrorCode::Generic, format!("not implemented: {name}")),
    )
}

/// `/rest/` 以外へのリクエストも、クライアントの解析のために残す。
/// クエリには認証情報が入りうるので、パスだけを残す。
async fn not_found(method: Method, uri: Uri) -> StatusCode {
    tracing::warn!(%method, path = uri.path(), "not found");
    StatusCode::NOT_FOUND
}

fn extensions(state: &AppState) -> Map<String, Value> {
    let mut list = vec![json!({ "name": "formPost", "versions": [1] })];
    if state.credentials.api_key.is_some() {
        list.push(json!({ "name": "apiKeyAuthentication", "versions": [1] }));
    }
    payload(json!({ "openSubsonicExtensions": list }))
}

pub(super) fn payload(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("payload must be an object"),
    }
}
