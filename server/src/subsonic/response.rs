use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value, json};

use super::{Params, xml};

/// 準拠する Subsonic API のバージョン。
pub const API_VERSION: &str = "1.16.1";
const SERVER_TYPE: &str = "suisei";
const XMLNS: &str = "http://subsonic.org/restapi";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Xml,
    Json,
}

impl Format {
    /// `f` の既定は Subsonic に合わせて XML。
    pub fn from_params(params: &Params) -> Self {
        match params.get("f") {
            Some("json") => Self::Json,
            _ => Self::Xml,
        }
    }
}

/// Subsonic のエラーコード。OpenSubsonic の定義に従う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Generic = 0,
    MissingParameter = 10,
    WrongCredentials = 40,
    AuthMechanismNotSupported = 42,
    ConflictingAuth = 43,
    InvalidApiKey = 44,
    NotAuthorized = 50,
    NotFound = 70,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub fn ok(format: Format, payload: Map<String, Value>) -> Response {
    render(format, "ok", payload)
}

/// Subsonic のエラーは HTTP 200 で返す。
pub fn error(format: Format, err: &Error) -> Response {
    let mut payload = Map::new();
    payload.insert(
        "error".into(),
        json!({ "code": err.code as u32, "message": err.message }),
    );
    render(format, "failed", payload)
}

fn render(format: Format, status: &str, payload: Map<String, Value>) -> Response {
    let mut body = Map::new();
    body.insert("status".into(), status.into());
    body.insert("version".into(), API_VERSION.into());
    body.insert("type".into(), SERVER_TYPE.into());
    body.insert("serverVersion".into(), env!("CARGO_PKG_VERSION").into());
    body.insert("openSubsonic".into(), true.into());
    body.extend(payload);

    match format {
        Format::Json => (
            [(CONTENT_TYPE, "application/json")],
            json!({ "subsonic-response": body }).to_string(),
        )
            .into_response(),
        Format::Xml => {
            body.insert("xmlns".into(), XMLNS.into());
            (
                [(CONTENT_TYPE, "text/xml; charset=utf-8")],
                xml::document("subsonic-response", &body),
            )
                .into_response()
        }
    }
}
