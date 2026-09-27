//! 空のライブラリとしての応答を、Navidrome の応答（`tests/fixtures/navidrome/`）と比べる。
//! 値ではなく、応答の要素名と、各要素が持つ項目の名前と型を比べる。

use std::fs;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{Map, Value};
use suisei::{AppState, Credentials};
use tower::ServiceExt;

const ENVELOPE: [&str; 5] = ["status", "version", "type", "serverVersion", "openSubsonic"];

async fn suisei(endpoint: &str, query: &str) -> Map<String, Value> {
    let app = suisei::router(AppState {
        credentials: Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: None,
        },
        db: suisei::db::open_in_memory().await.unwrap(),
    });
    let uri = format!("/rest/{endpoint}?u=inaba&p=sesame&f=json{query}");
    let res = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    payload(serde_json::from_slice(&body).unwrap())
}

fn navidrome(name: &str) -> Map<String, Value> {
    let path = format!(
        "{}/tests/fixtures/navidrome/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    payload(serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap())
}

/// 共通の項目を除いた、応答の中身。
fn payload(value: Value) -> Map<String, Value> {
    let Value::Object(mut res) = value["subsonic-response"].clone() else {
        panic!("not a subsonic response: {value}");
    };
    assert_eq!(res["status"], "ok", "{res:?}");
    res.retain(|k, _| !ENVELOPE.contains(&k.as_str()));
    res
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// `ours` の項目がすべて `theirs` にあり、型が同じであることを確かめる。
fn assert_subset(path: &str, ours: &Value, theirs: &Value) {
    assert_eq!(kind(ours), kind(theirs), "{path}");
    match (ours, theirs) {
        (Value::Object(ours), Value::Object(theirs)) => {
            for (key, value) in ours {
                let other = theirs
                    .get(key)
                    .unwrap_or_else(|| panic!("{path}.{key} is not in Navidrome's response"));
                assert_subset(&format!("{path}.{key}"), value, other);
            }
        }
        (Value::Array(ours), Value::Array(theirs)) => {
            if let (Some(ours), Some(theirs)) = (ours.first(), theirs.first()) {
                assert_subset(&format!("{path}[]"), ours, theirs);
            }
        }
        _ => {}
    }
}

async fn assert_same_shape(endpoint: &str, query: &str, fixture: &str) {
    let ours = suisei(endpoint, query).await;
    let theirs = navidrome(fixture);
    let keys = |m: &Map<String, Value>| m.keys().cloned().collect::<Vec<_>>();
    assert_eq!(keys(&ours), keys(&theirs), "{endpoint}");
    assert_subset(endpoint, &Value::Object(ours), &Value::Object(theirs));
}

#[tokio::test]
async fn empty_library_matches_navidrome() {
    for (endpoint, query, fixture) in [
        ("getMusicFolders", "", "getMusicFolders"),
        ("getArtists", "", "getArtists"),
        ("getIndexes", "", "getIndexes"),
        ("getAlbumList2", "&type=newest", "getAlbumList2"),
        ("getRandomSongs", "", "getRandomSongs"),
        ("getGenres", "", "getGenres"),
        ("search3", "&query=%22%22", "search3-empty"),
        ("getStarred2", "", "getStarred2"),
        ("getBookmarks", "", "getBookmarks"),
        ("getPlaylists", "", "getPlaylists"),
        ("getShares", "", "getShares"),
        ("getInternetRadioStations", "", "getInternetRadioStations"),
        ("getNowPlaying", "", "getNowPlaying"),
        ("getScanStatus", "", "getScanStatus"),
        ("getUser", "&username=inaba", "getUser"),
    ] {
        assert_same_shape(endpoint, query, fixture).await;
    }
}

/// 権限の項目は省略できないので、Navidrome と同じ項目がすべてあることも確かめる。
#[tokio::test]
async fn user_has_all_roles() {
    let ours = suisei("getUser", "&username=inaba").await;
    let theirs = navidrome("getUser");
    let keys = |m: &Map<String, Value>| {
        let mut keys: Vec<_> = m["user"].as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    };
    assert_eq!(keys(&ours), keys(&theirs));
}
