//! 一覧の頭文字の索引に使う、頭文字ごとの先頭の位置（docs/web.md の「一覧」）。
//!
//! 100 件ずつ読み足す一覧（アルバム、トラック）は、まだ読んでいない位置へ飛ぶために、
//! 頭文字ごとの先頭が何件目かと全体の件数を先に知る必要がある。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use super::AppState;
use super::session;
use crate::db::{browse, search};
use crate::tags::index_heading;

#[derive(Debug, Deserialize)]
pub(super) struct HeadingsQuery {
    /// `albums` か `tracks`
    list: String,
    /// アルバムは `name`、`artist`。トラックは `title`、`album`、`artist`
    sort: String,
    /// アルバムをお気に入りに絞るか
    starred: Option<bool>,
}

/// 並び順と絞り込みごとに、頭文字と先頭の位置（offset）の組と全体の件数を返す。
pub(super) async fn get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HeadingsQuery>,
) -> Response {
    if let Err(status) = session::require(&state, &headers).await {
        return status.into_response();
    }
    let keys = match (query.list.as_str(), query.sort.as_str()) {
        ("albums", "name" | "artist") => {
            let by_artist = query.sort == "artist";
            browse::album_heading_keys(&state.db, by_artist, query.starred.unwrap_or(false)).await
        }
        ("tracks", sort) => {
            let sort = match sort {
                "title" => search::SongSort::Title,
                "album" => search::SongSort::Album,
                "artist" => search::SongSort::Artist,
                _ => return StatusCode::BAD_REQUEST.into_response(),
            };
            search::song_heading_keys(&state.db, sort).await
        }
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    match keys {
        Ok(keys) => {
            let total = keys.len();
            let headings: Vec<_> = first_offsets(keys)
                .into_iter()
                .map(|(heading, offset)| json!({ "heading": heading, "offset": offset }))
                .collect();
            Json(json!({ "headings": headings, "total": total })).into_response()
        }
        Err(err) => {
            tracing::error!(%err, "headings api failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// 並びの順のキーから、頭文字ごとに最初に現れる位置を返す。キーのない行は見出しを持たない。
fn first_offsets(keys: Vec<Option<String>>) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for (offset, key) in keys.into_iter().enumerate() {
        let Some(key) = key else { continue };
        let heading = index_heading(&key);
        if !out.iter().any(|(h, _)| *h == heading) {
            out.push((heading, offset));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags::sort_key;

    #[test]
    fn first_offsets_keeps_the_first_row_of_each_heading() {
        let keys = [
            None,
            Some("1"),
            Some("Abc"),
            Some("あい"),
            Some("アオ"),
            Some("Bcd"),
        ]
        .map(|name| name.map(|n| sort_key(n, None)))
        .to_vec();
        assert_eq!(
            first_offsets(keys),
            vec![
                ("#".to_owned(), 1),
                ("A".to_owned(), 2),
                ("あ".to_owned(), 3),
                ("B".to_owned(), 5),
            ]
        );
    }
}
