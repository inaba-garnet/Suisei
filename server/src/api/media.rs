//! 音声を返すエンドポイント。

use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, Method, Request};
use axum::response::Response;
use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;
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

    let mut response = serve_file(
        ServeFile::new_with_mime(&path, &mime),
        &path,
        method,
        headers,
    )
    .await?;
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

/// アルバムか曲のカバーアートを返す。`size` による縮小はまだしない（#23）。
pub async fn cover_art(
    method: &Method,
    headers: &HeaderMap,
    params: &Params,
    state: &AppState,
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
    let not_found = || Error::new(ErrorCode::NotFound, "cover art not found");
    let rel = browse::cover_path(&state.db, &id)
        .await
        .map_err(db_error)?
        .flatten()
        .ok_or_else(not_found)?;
    let path = resolve(state.scanner.music_dir(), &rel)
        .filter(|p| p.is_file())
        .ok_or_else(not_found)?;

    if is_image(&path) {
        return serve_file(ServeFile::new(&path), &path, method, headers).await;
    }
    // 音声のファイルなら、埋め込みの画像を読む。表紙の種類を優先する
    let picture = tokio::task::spawn_blocking(move || {
        let file = lofty::read_from_path(&path).ok()?;
        let tag = file.primary_tag().or_else(|| file.first_tag())?;
        let pictures = tag.pictures();
        let picture = pictures
            .iter()
            .find(|p| p.pic_type() == PictureType::CoverFront)
            .or_else(|| pictures.first())?;
        Some((
            picture.mime_type().map(|m| m.as_str().to_owned()),
            picture.data().to_vec(),
        ))
    })
    .await
    .expect("画像を読むタスクが panic した")
    .ok_or_else(not_found)?;
    let (mime, data) = picture;
    let content_type = mime.unwrap_or_else(|| "image/jpeg".to_owned());
    Response::builder()
        .header(CONTENT_TYPE, content_type)
        .body(Body::from(data))
        .map_err(|_| Error::new(ErrorCode::Generic, "cannot build response"))
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "gif" | "jpeg" | "jpg" | "png" | "webp"
            )
        })
}

/// ServeFile にリクエストを渡す。Range、HEAD、Last-Modified を任せるため、元のメソッドとヘッダを写す。
async fn serve_file(
    mut service: ServeFile,
    path: &Path,
    method: &Method,
    headers: &HeaderMap,
) -> Result<Response, Error> {
    let mut request = Request::builder()
        .method(method.clone())
        .body(Body::empty())
        .expect("メソッドだけのリクエストは作れる");
    *request.headers_mut() = headers.clone();
    Ok(service
        .try_call(request)
        .await
        .map_err(|err| {
            tracing::error!(path = %path.display(), error = %err, "cannot read file");
            Error::new(ErrorCode::Generic, "cannot read file")
        })?
        .map(Body::new))
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
