//! Web クライアントのログインと、Cookie のセッションでの認証（docs/server.md の「Web クライアントのログイン」）。

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::State;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::AppState;
use crate::db;
use crate::subsonic::{self, Error, ErrorCode, Params};

const COOKIE_NAME: &str = "suisei_session";
/// 最後に使ってからこの長さでセッションが切れる。
const LIFETIME: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// 最終利用日時を書き込む最短の間隔。
const TOUCH_INTERVAL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Deserialize)]
pub(super) struct Login {
    username: String,
    password: String,
}

/// `Json` は `Content-Type: application/json` でなければ 415 で拒むので、別のサイトのフォームからは送れない。
pub(super) async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(login): Json<Login>,
) -> Response {
    if !subsonic::verify_password(&state.credentials, &login.username, &login.password) {
        tracing::info!("login failed");
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let now = now_ms();
    if let Err(err) = db::session::delete_unused_since(&state.db, now - millis(LIFETIME)).await {
        tracing::warn!(%err, "failed to delete expired sessions");
    }
    let token = new_token();
    if let Err(err) = db::session::create(&state.db, &hash(&token), now).await {
        tracing::error!(%err, "failed to create session");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    tracing::info!("login");
    (
        StatusCode::NO_CONTENT,
        [(SET_COOKIE, session_cookie(&token, &headers))],
    )
        .into_response()
}

pub(super) async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_json(&headers) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    if let Some(token) = cookie(&headers)
        && let Err(err) = db::session::delete(&state.db, &hash(token)).await
    {
        tracing::error!(%err, "failed to delete session");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    (
        StatusCode::NO_CONTENT,
        [(SET_COOKIE, clear_cookie(&headers))],
    )
        .into_response()
}

/// ログイン中の利用者を返し、Cookie を発行し直して期限を延ばす。
pub(super) async fn me(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let Some(token) = cookie(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match verify(&state, token).await {
        Ok(true) => (
            [(SET_COOKIE, session_cookie(token, &headers))],
            Json(json!({ "username": state.credentials.user })),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            [(SET_COOKIE, clear_cookie(&headers))],
        )
            .into_response(),
        Err(err) => {
            tracing::error!(%err, "failed to verify session");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Subsonic API の認証。認証の引数がなく Cookie があるときだけ、セッションで認証する。
pub(super) async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    params: &Params,
) -> Result<(), Error> {
    if !subsonic::has_credentials(params)
        && let Some(token) = cookie(headers)
    {
        return match verify(state, token).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(Error::new(
                ErrorCode::WrongCredentials,
                "session is expired or invalid",
            )),
            Err(err) => {
                tracing::error!(%err, "failed to verify session");
                Err(Error::new(ErrorCode::Generic, "failed to verify session"))
            }
        };
    }
    subsonic::authenticate(params, &state.credentials)
}

/// セッションが有効か確かめ、必要なら最終利用日時を更新する。
async fn verify(state: &AppState, token: &str) -> Result<bool, sqlx::Error> {
    let id_hash = hash(token);
    let Some(last_used_at) = db::session::last_used_at(&state.db, &id_hash).await? else {
        return Ok(false);
    };
    let now = now_ms();
    if now - last_used_at >= millis(LIFETIME) {
        return Ok(false);
    }
    if now - last_used_at >= millis(TOUCH_INTERVAL) {
        db::session::touch(&state.db, &id_hash, now).await?;
    }
    Ok(true)
}

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("failed to get random bytes");
    hex::encode(bytes)
}

fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token))
}

/// `Cookie` ヘッダーからセッションの値を取り出す。
fn cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value)
}

fn session_cookie(token: &str, headers: &HeaderMap) -> HeaderValue {
    build_cookie(token, LIFETIME.as_secs(), headers)
}

fn clear_cookie(headers: &HeaderMap) -> HeaderValue {
    build_cookie("", 0, headers)
}

fn build_cookie(value: &str, max_age: u64, headers: &HeaderMap) -> HeaderValue {
    let mut cookie =
        format!("{COOKIE_NAME}={value}; Path=/; Max-Age={max_age}; HttpOnly; SameSite=Strict");
    if is_https(headers) {
        cookie.push_str("; Secure");
    }
    HeaderValue::try_from(cookie).expect("cookie is ASCII")
}

/// TLS はリバースプロキシが終端するので、`X-Forwarded-Proto` で判断する。
/// プロキシが重なると `https, http` のように並ぶので、最初の値（利用者に近い側）を見る。
fn is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("https"))
}

fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
}

fn now_ms() -> i64 {
    millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default(),
    )
}

fn millis(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}
