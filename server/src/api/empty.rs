//! ライブラリを実装するまでの、空のライブラリとしての応答。
//! エラーを返すと Symfonium が同期を中止するので、正しい形の空の応答で同期を先に進め、呼ばれる流れを記録する。
//! 形は Navidrome の応答（`tests/fixtures/navidrome/`）に合わせる。本実装ができたものから置き換える。

use serde_json::{Map, Value, json};

use super::{AppState, payload};
use crate::subsonic::{Error, ErrorCode, Params};

/// 唯一の音楽フォルダの ID。
pub const MUSIC_FOLDER_ID: u32 = 1;

/// 空応答を持たないエンドポイントには `None` を返す。
pub fn respond(
    name: &str,
    params: &Params,
    state: &AppState,
) -> Option<Result<Map<String, Value>, Error>> {
    let value = match name {
        "getMusicFolders" => json!({ "musicFolders": {
            "musicFolder": [{ "id": MUSIC_FOLDER_ID, "name": "Music" }],
        }}),
        "getAlbumList" => json!({ "albumList": {} }),
        "getRandomSongs" => json!({ "randomSongs": {} }),
        "getSongsByGenre" => json!({ "songsByGenre": {} }),
        "search2" => json!({ "searchResult2": {} }),
        "getStarred" => json!({ "starred": {} }),
        "getPlaylists" => json!({ "playlists": {} }),
        // 保存した再生キューがなければ、中身のない応答を返す（OpenSubsonic の仕様）。
        "getPlayQueue" => json!({}),
        "getUser" => return Some(user(params, state)),
        "getUsers" => json!({ "users": { "user": [user_entry(state)] } }),
        _ => return None,
    };
    Some(Ok(payload(value)))
}

fn user(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let username = &state.credentials.user;
    let requested = params.get("username").ok_or_else(|| {
        Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: username",
        )
    })?;
    if requested != username {
        return Err(Error::new(ErrorCode::NotFound, "user not found"));
    }
    Ok(payload(json!({ "user": user_entry(state) })))
}

/// 利用者は一人なので、その一人にすべての権限を与える。
fn user_entry(state: &AppState) -> Value {
    json!({
        "username": state.credentials.user,
        "scrobblingEnabled": true,
        "adminRole": true,
        "settingsRole": true,
        "downloadRole": true,
        "uploadRole": false,
        "playlistRole": true,
        "coverArtRole": true,
        "commentRole": false,
        "podcastRole": false,
        "streamRole": true,
        "jukeboxRole": false,
        "shareRole": false,
        "videoConversionRole": false,
        "folder": [MUSIC_FOLDER_ID],
    })
}
