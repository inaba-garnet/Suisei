//! 音声を返すエンドポイント。

use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::http::header::CONTENT_DISPOSITION;
use axum::http::{HeaderMap, HeaderValue, Method, Request};
use axum::response::Response;
use tower_http::services::ServeFile;

use super::AppState;
use super::browse::db_error;
use crate::db::browse;
use crate::subsonic::{Error, ErrorCode, Params};

/// 曲の配信ファイルを返す。`download` なら保存用に `Content-Disposition` を付ける。
/// トランスコードはまだしないので、`maxBitRate` と `format` は無視して元のファイルを返す。
/// Range、HEAD、Last-Modified は tower-http の ServeFile に任せる。
pub async fn stream(
    method: &Method,
    headers: &HeaderMap,
    params: &Params,
    state: &AppState,
    download: bool,
) -> Result<Response, Error> {
    let id = params.get("id").ok_or_else(|| {
        Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: id",
        )
    })?;
    let id = browse::resolve_alias(&state.db, id)
        .await
        .map_err(db_error)?;
    let file = browse::stream_file(&state.db, &id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| Error::new(ErrorCode::NotFound, "song not found"))?;
    let path = resolve(state.scanner.music_dir(), &file.path)
        .filter(|p| p.is_file())
        .ok_or_else(|| Error::new(ErrorCode::NotFound, "file not found"))?;
    let mime = file
        .content_type
        .parse()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);

    let mut request = Request::builder()
        .method(method.clone())
        .body(Body::empty())
        .expect("メソッドだけのリクエストは作れる");
    *request.headers_mut() = headers.clone();
    let mut response = ServeFile::new_with_mime(&path, &mime)
        .try_call(request)
        .await
        .map_err(|err| {
            tracing::error!(path = %path.display(), error = %err, "cannot read file");
            Error::new(ErrorCode::Generic, "cannot read file")
        })?
        .map(Body::new);
    if download && let Some(name) = Path::new(&file.path).file_name() {
        let value = format!(
            "attachment; filename*=UTF-8''{}",
            percent_encode(&name.to_string_lossy())
        );
        if let Ok(value) = HeaderValue::from_str(&value) {
            response.headers_mut().insert(CONTENT_DISPOSITION, value);
        }
    }
    Ok(response)
}

/// 音楽フォルダからの相対パスを、音楽フォルダの外に出ないことを確かめてからつなぐ。
fn resolve(music_dir: &Path, rel: &str) -> Option<PathBuf> {
    let rel = Path::new(rel);
    rel.components()
        .all(|c| matches!(c, Component::Normal(_)))
        .then(|| music_dir.join(rel))
}

/// RFC 5987 の形で、英数字と一部の記号以外を %XX にする。日本語のファイル名を保存できるようにするため。
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 3);
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_stay_inside_music_dir() {
        let dir = Path::new("/music");
        assert_eq!(
            resolve(dir, "a/b.flac"),
            Some(PathBuf::from("/music/a/b.flac"))
        );
        assert_eq!(resolve(dir, "../etc/passwd"), None);
        assert_eq!(resolve(dir, "/etc/passwd"), None);
    }

    #[test]
    fn filenames_are_percent_encoded() {
        assert_eq!(percent_encode("01 曲.flac"), "01%20%E6%9B%B2.flac");
    }
}
