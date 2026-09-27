//! 再生履歴のエンドポイント（docs/schema.md の「再生履歴」）。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::browse::db_error;
use super::search::songs;
use super::{AppState, payload};
use crate::db::{browse, history};
use crate::subsonic::{Error, ErrorCode, Params};

/// クライアント（`c`）ごとの再生中の曲。再起動で消えてよいので DB に持たない。
#[derive(Debug, Default)]
pub struct NowPlaying(Mutex<HashMap<String, Playing>>);

#[derive(Debug, Clone)]
struct Playing {
    track_id: String,
    started_at: SystemTime,
}

impl NowPlaying {
    fn start(&self, client: &str, track_id: &str) {
        let playing = Playing {
            track_id: track_id.to_owned(),
            started_at: SystemTime::now(),
        };
        self.lock().insert(client.to_owned(), playing);
    }

    /// 再生を終えた曲なら、再生中から外す。
    fn finish(&self, client: &str, track_id: &str) {
        let mut map = self.lock();
        if map.get(client).is_some_and(|p| p.track_id == track_id) {
            map.remove(client);
        }
    }

    /// クライアントの名前の順。
    fn entries(&self) -> Vec<(String, Playing)> {
        let mut entries: Vec<_> = self
            .lock()
            .iter()
            .map(|(c, p)| (c.clone(), p.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Playing>> {
        // 中身は置き換えるだけで途中の状態を残さないので、パニックした後も使い続ける
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

pub async fn scrobble(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let ids: Vec<&str> = params.get_all("id").collect();
    if ids.is_empty() {
        return Err(Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: id",
        ));
    }
    // id と time は同じ順に繰り返される
    let times: Vec<Option<i64>> = params.get_all("time").map(|t| t.parse().ok()).collect();
    let submission = params.get("submission") != Some("false");
    let client = params.get("c").unwrap_or_default();
    let now = millis(SystemTime::now());

    for (i, id) in ids.iter().enumerate() {
        let track_id = browse::resolve_alias(&state.db, id)
            .await
            .map_err(db_error)?;
        if !submission {
            state.now_playing.start(client, &track_id);
            continue;
        }
        let played_at = times.get(i).copied().flatten().unwrap_or(now);
        let recorded = history::record(&state.db, &track_id, played_at)
            .await
            .map_err(db_error)?;
        if !recorded {
            // エラーにすると、クライアントが再生の束ごと再送し続けるおそれがある
            tracing::warn!(id, "scrobble for unknown track skipped");
        }
        state.now_playing.finish(client, &track_id);
    }
    Ok(Map::new())
}

/// Subsonic の `getNowPlaying`。曲の長さを過ぎたものは返さない。
pub async fn now_playing(state: &AppState) -> Result<Map<String, Value>, Error> {
    let entries = state.now_playing.entries();
    let ids: Vec<String> = entries.iter().map(|(_, p)| p.track_id.clone()).collect();
    let mut by_id: HashMap<String, Value> = songs(state, &ids)
        .await?
        .into_iter()
        .map(|song| (song["id"].as_str().unwrap_or_default().to_owned(), song))
        .collect();
    let now = SystemTime::now();
    let mut list = Vec::new();
    for (player_id, (client, playing)) in entries.iter().enumerate() {
        let Some(mut song) = by_id.remove(&playing.track_id) else {
            continue;
        };
        let elapsed = now
            .duration_since(playing.started_at)
            .unwrap_or(Duration::ZERO);
        let duration = Duration::from_secs(song["duration"].as_u64().unwrap_or(0));
        if elapsed > duration {
            continue;
        }
        song["username"] = json!(state.credentials.user);
        song["minutesAgo"] = json!(elapsed.as_secs() / 60);
        song["playerId"] = json!(player_id);
        song["playerName"] = json!(client);
        list.push(song);
    }
    // Navidrome と同じく、再生中の曲がなければ `entry` を省く
    if list.is_empty() {
        return Ok(payload(json!({ "nowPlaying": {} })));
    }
    Ok(payload(json!({ "nowPlaying": { "entry": list } })))
}

/// Subsonic の `count` の既定値と上限。
const TOP_SONGS_DEFAULT: i64 = 50;
const TOP_SONGS_MAX: i64 = 500;

/// 外部の人気順ではなく、自分の再生回数の多い順に返す。
pub async fn top_songs(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let artist = params.get("artist").ok_or_else(|| {
        Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: artist",
        )
    })?;
    let count = params
        .get("count")
        .and_then(|c| c.parse::<i64>().ok())
        .unwrap_or(TOP_SONGS_DEFAULT)
        .clamp(0, TOP_SONGS_MAX);
    let ids = history::top_songs(&state.db, artist, count)
        .await
        .map_err(db_error)?;
    Ok(payload(
        json!({ "topSongs": { "song": songs(state, &ids).await? } }),
    ))
}

fn millis(at: SystemTime) -> i64 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
