mod browse;
mod empty;
mod media;
mod search;

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::header::USER_AGENT;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use axum::routing::{any, get};
use serde_json::{Map, Value, json};

use crate::Credentials;
use crate::db::Pool;
use crate::scan::{self, Scanner};
use crate::subsonic::{self, Error, ErrorCode, Format, Params};

#[derive(Debug, Clone)]
pub struct AppState {
    pub credentials: Credentials,
    pub db: Pool,
    pub scanner: Arc<Scanner>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/rest/{endpoint}", any(rest))
        .fallback(not_found)
        .with_state(Arc::new(state))
}

/// Subsonic のエンドポイントは `ping` と `ping.view` のどちらでも呼ばれるので、名前で振り分ける。
async fn rest(
    State(state): State<Arc<AppState>>,
    method: Method,
    headers: HeaderMap,
    Path(endpoint): Path<String>,
    params: Params,
) -> Response {
    let name = endpoint.strip_suffix(".view").unwrap_or(&endpoint);
    let format = Format::from_params(&params);
    tracing::info!(
        %method,
        endpoint = name,
        params = %params.masked(),
        user_agent = user_agent(&headers),
        "request"
    );

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
        "stream" | "download" => {
            match media::stream(&method, &headers, &params, &state, name == "download").await {
                Ok(response) => response,
                Err(err) => subsonic::error(format, &err),
            }
        }
        "getCoverArt" => match media::cover_art(&method, &headers, &params, &state).await {
            Ok(response) => response,
            Err(err) => subsonic::error(format, &err),
        },
        "ping" => subsonic::ok(format, Map::new()),
        "getLicense" => subsonic::ok(format, payload(json!({ "license": { "valid": true } }))),
        "getOpenSubsonicExtensions" => subsonic::ok(format, extensions(&state)),
        "startScan" => {
            // Navidrome と同じく、fullScan=true ならすべてのファイルを読み直す
            let mode = if params.get("fullScan") == Some("true") {
                scan::Mode::Full
            } else {
                scan::Mode::Quick
            };
            state.scanner.start(mode);
            subsonic::ok(format, scan_status(&state))
        }
        "getScanStatus" => subsonic::ok(format, scan_status(&state)),
        _ => match browse::respond(name, &params, &state)
            .await
            .or_else(|| empty::respond(name, &params, &state))
        {
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

/// Amperfy はログインの前にサーバーの URL そのものを GET し、400 以上なら接続できないとみなす。
/// Web クライアントを `/` で配信するかが決まるまでの仮の応答。
async fn root(headers: HeaderMap) -> &'static str {
    tracing::info!(user_agent = user_agent(&headers), "root");
    "Suisei"
}

/// `/rest/` 以外へのリクエストも、クライアントの解析のために残す。
/// クエリには認証情報が入りうるので、パスだけを残す。
async fn not_found(method: Method, uri: Uri, headers: HeaderMap) -> StatusCode {
    tracing::warn!(
        %method,
        path = uri.path(),
        user_agent = user_agent(&headers),
        "not found"
    );
    StatusCode::NOT_FOUND
}

/// `c=` を付けない呼び出しでも送り主を見分けられるよう、ログに残す。
fn user_agent(headers: &HeaderMap) -> &str {
    headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
}

fn extensions(state: &AppState) -> Map<String, Value> {
    let mut list = vec![json!({ "name": "formPost", "versions": [1] })];
    if state.credentials.api_key.is_some() {
        list.push(json!({ "name": "apiKeyAuthentication", "versions": [1] }));
    }
    payload(json!({ "openSubsonicExtensions": list }))
}

/// Subsonic の scanning と count に、Navidrome と同じ folderCount と lastScan を足す。
fn scan_status(state: &AppState) -> Map<String, Value> {
    let status = state.scanner.status();
    let mut scan_status = json!({
        "scanning": status.scanning,
        "count": status.count,
        "folderCount": status.folder_count,
    });
    if let Some(at) = status.last_scan {
        scan_status["lastScan"] = json!(humantime::format_rfc3339_millis(at).to_string());
    }
    payload(json!({ "scanStatus": scan_status }))
}

pub(super) fn payload(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("payload must be an object"),
    }
}
