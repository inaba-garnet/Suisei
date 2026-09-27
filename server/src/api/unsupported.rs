//! 実装しない機能と、外部の情報源が要るエンドポイントの応答（docs/server.md の「未対応のエンドポイント」）。

use serde_json::{Map, Value, json};

use super::payload;
use crate::subsonic::{Error, ErrorCode, Params};

/// 実装を予定しているエンドポイント。実装するまで仮の措置（200 と code 0）を返す（docs/verification.md）。
pub const PENDING: &[&str] = &[
    "getPlaylist",
    "createPlaylist",
    "updatePlaylist",
    "deletePlaylist",
    "savePlayQueue",
    "getPlayQueueByIndex",
    "savePlayQueueByIndex",
    "getLyrics",
    "getLyricsBySongId",
    "getMusicDirectory",
];

/// 実装しない機能の操作。HTTP 501 を返す。
pub const NOT_IMPLEMENTED: &[&str] = &[
    "refreshPodcasts",
    "createPodcastChannel",
    "deletePodcastChannel",
    "deletePodcastEpisode",
    "downloadPodcastEpisode",
    "getPodcastEpisode",
    "createShare",
    "updateShare",
    "deleteShare",
    "jukeboxControl",
    "createInternetRadioStation",
    "updateInternetRadioStation",
    "deleteInternetRadioStation",
    "addChatMessage",
    "getVideoInfo",
    "getCaptions",
    "hls",
    "createBookmark",
    "createUser",
    "updateUser",
    "deleteUser",
    "changePassword",
    "getAvatar",
    "search",
];

/// 空の一覧や空の情報を返すエンドポイントでなければ `None` を返す。
pub fn respond(name: &str, params: &Params) -> Option<Result<Map<String, Value>, Error>> {
    let value = match name {
        // 実装しない機能の一覧
        "getPodcasts" => json!({ "podcasts": {} }),
        "getNewestPodcasts" => json!({ "newestPodcasts": {} }),
        "getShares" => json!({ "shares": {} }),
        "getInternetRadioStations" => json!({ "internetRadioStations": {} }),
        "getChatMessages" => json!({ "chatMessages": {} }),
        "getVideos" => json!({ "videos": {} }),
        "getBookmarks" => json!({ "bookmarks": {} }),
        // ブックマークを持たないので、消した後の状態と一致する
        "deleteBookmark" => return Some(require(params, "id").map(|()| Map::new())),
        // 外部の情報源が要るもの
        "getArtistInfo" => return Some(info(params, "id", "artistInfo")),
        "getArtistInfo2" => return Some(info(params, "id", "artistInfo2")),
        "getAlbumInfo" | "getAlbumInfo2" => return Some(info(params, "id", "albumInfo")),
        "getSimilarSongs" => return Some(info(params, "id", "similarSongs")),
        "getSimilarSongs2" => return Some(info(params, "id", "similarSongs2")),
        _ => return None,
    };
    Some(Ok(payload(value)))
}

fn info(params: &Params, key: &str, element: &str) -> Result<Map<String, Value>, Error> {
    require(params, key)?;
    let mut map = Map::new();
    map.insert(element.into(), json!({}));
    Ok(map)
}

fn require(params: &Params, key: &str) -> Result<(), Error> {
    if params.contains(key) {
        Ok(())
    } else {
        Err(Error::new(
            ErrorCode::MissingParameter,
            format!("required parameter is missing: {key}"),
        ))
    }
}
