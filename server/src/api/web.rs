//! Web クライアントの配信（docs/server.md の「Web クライアントの配信」）。

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, Method, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// Web のビルド結果。release ビルドではバイナリに取り込み、debug ビルドでは実行時にディスクから読む。
#[derive(RustEmbed)]
#[folder = "../web/.output/public"]
#[allow_missing = true]
struct Embedded;

/// 配信するファイルの一覧。
#[derive(Debug, Clone)]
pub struct Web(Source);

#[derive(Debug, Clone)]
enum Source {
    Embedded,
    Files(Arc<HashMap<String, Vec<u8>>>),
}

impl Web {
    /// バイナリに取り込んだ Web のビルド結果。
    pub fn embedded() -> Self {
        Self(Source::Embedded)
    }

    /// 与えたファイルを配る。キーは先頭の `/` を除いたパス。
    pub fn from_files(files: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
        Self(Source::Files(Arc::new(files.into_iter().collect())))
    }

    fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        match &self.0 {
            Source::Embedded => Embedded::get(path).map(|file| file.data),
            Source::Files(files) => files.get(path).map(|data| Cow::Owned(data.clone())),
        }
    }
}

const INDEX: &str = "index.html";

/// `/rest` と `/api` 以外のリクエスト。ファイルがあれば返し、画面の URL には `index.html` を返す。
pub(super) async fn serve(web: Web, method: Method, uri: Uri, headers: HeaderMap) -> Response {
    let path = uri.path().trim_start_matches('/');
    if matches!(method, Method::GET | Method::HEAD)
        && !matches!(path.split('/').next(), Some("api" | "rest"))
        && !path.split('/').any(|segment| segment == "..")
    {
        if let Some(data) = web.get(path) {
            return file(path, data);
        }
        // 拡張子のないパスは画面の URL とみなす。画面の振り分けは Web クライアントがする
        let is_page = !path.rsplit('/').next().unwrap_or("").contains('.');
        if is_page && let Some(data) = web.get(INDEX) {
            return file(INDEX, data);
        }
        // Amperfy はログインの前にサーバーの URL そのものを GET し、400 以上なら接続できないとみなす
        if path.is_empty() {
            tracing::info!(user_agent = super::user_agent(&headers), "root");
            return "Suisei".into_response();
        }
    }
    super::not_found(method, uri, headers).await.into_response()
}

fn file(path: &str, data: Cow<'static, [u8]>) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    // `_nuxt/` の下はファイル名に内容のハッシュが入るので、変わらない
    let cache = if path.starts_with("_nuxt/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [
            (
                CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or(HeaderValue::from_static("application/octet-stream")),
            ),
            (CACHE_CONTROL, HeaderValue::from_static(cache)),
        ],
        data,
    )
        .into_response()
}
