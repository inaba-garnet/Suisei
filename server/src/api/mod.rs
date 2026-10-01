mod annotation;
mod browse;
mod cover;
mod empty;
mod history;
mod lyrics;
mod media;
mod playlist;
mod search;
mod session;
mod transcode;
mod unsupported;
mod web;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::header::USER_AGENT;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use serde_json::{Map, Value, json};

use crate::Credentials;
use crate::db::Pool;
use crate::scan::{self, Scanner};
use crate::subsonic::{self, Error, ErrorCode, Format, Params};

pub use history::NowPlaying;
pub use web::Web;

#[derive(Debug, Clone)]
pub struct AppState {
    pub credentials: Credentials,
    pub db: Pool,
    pub scanner: Arc<Scanner>,
    pub now_playing: Arc<NowPlaying>,
    /// 作り直せるデータの置き場所。データの置き場所の下の `cache/`（docs/server.md）
    pub cache_dir: PathBuf,
    /// トランスコードに使う ffmpeg（docs/server.md）
    pub ffmpeg: PathBuf,
    /// 開発モード（docs/server.md の「設定」）
    pub dev: bool,
}

pub fn router(state: AppState) -> Router {
    router_with(state, Web::embedded())
}

/// 配信する Web クライアントを差し替えられる [`router`]。
pub fn router_with(state: AppState, web: Web) -> Router {
    Router::new()
        .route("/rest/{endpoint}", any(rest))
        .route("/api/login", post(session::login))
        .route("/api/logout", post(session::logout))
        .route("/api/me", get(session::me))
        .with_state(Arc::new(state))
        .fallback(move |method: Method, uri: Uri, headers: HeaderMap| {
            web::serve(web.clone(), method, uri, headers)
        })
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
        && let Err(err) = session::authenticate(&state, &headers, &params).await
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
        "scrobble" => reply(format, history::scrobble(&params, &state).await),
        "getNowPlaying" => reply(format, history::now_playing(&state).await),
        "getTopSongs" => reply(format, history::top_songs(&params, &state).await),
        "star" => reply(format, annotation::star(&params, &state).await),
        "unstar" => reply(format, annotation::unstar(&params, &state).await),
        "setRating" => reply(format, annotation::set_rating(&params, &state).await),
        "getStarred2" => reply(format, annotation::starred2(&params, &state).await),
        "getPlaylists" => reply(format, playlist::list(&state).await),
        "getPlaylist" => reply(format, playlist::get(&params, &state).await),
        "createPlaylist" => reply(format, playlist::create(&params, &state).await),
        "updatePlaylist" => reply(format, playlist::update(&params, &state).await),
        "deletePlaylist" => reply(format, playlist::delete(&params, &state).await),
        "getLyricsBySongId" => reply(format, lyrics::by_song_id(&params, &state).await),
        "getLyrics" => reply(format, lyrics::by_name(&params, &state).await),
        _ => match browse::respond(name, &params, &state)
            .await
            .or_else(|| empty::respond(name, &params, &state))
            .or_else(|| unsupported::respond(name, &params))
        {
            Some(Ok(payload)) => subsonic::ok(format, payload),
            Some(Err(err)) => subsonic::error(format, &err),
            None if unsupported::PENDING.contains(&name) => pending(name, &method, &params, format),
            None if unsupported::NOT_IMPLEMENTED.contains(&name) => {
                (StatusCode::NOT_IMPLEMENTED, "not implemented").into_response()
            }
            None => {
                tracing::warn!(%method, endpoint = name, "unknown endpoint");
                StatusCode::NOT_FOUND.into_response()
            }
        },
    }
}

fn reply(format: Format, result: Result<Map<String, Value>, Error>) -> Response {
    match result {
        Ok(payload) => subsonic::ok(format, payload),
        Err(err) => subsonic::error(format, &err),
    }
}

/// 実装を予定しているエンドポイントの、実装するまでの仮の措置（docs/verification.md）。
/// HTTP のエラーではクライアントの同期が止まるおそれがあるので、200 と Subsonic のエラーを返し、呼ばれたことを warn で残す。
fn pending(name: &str, method: &Method, params: &Params, format: Format) -> Response {
    tracing::warn!(%method, endpoint = name, params = %params.masked(), "not implemented");
    subsonic::error(
        format,
        &Error::new(ErrorCode::Generic, format!("not implemented: {name}")),
    )
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
    let mut list = vec![
        json!({ "name": "formPost", "versions": [1] }),
        json!({ "name": "songLyrics", "versions": [1] }),
        json!({ "name": "transcodeOffset", "versions": [1] }),
    ];
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
