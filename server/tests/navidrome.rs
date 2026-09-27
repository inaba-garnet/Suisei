//! 空のライブラリとしての応答を、Navidrome の応答（`tests/fixtures/navidrome/`）と比べる。
//! 値ではなく、応答の要素名と、各要素が持つ項目の名前と型を比べる。

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{Map, Value};

mod common;
use common::{assert_subset, navidrome, payload};
use suisei::{AppState, Credentials};
use tower::ServiceExt;

async fn suisei(endpoint: &str, query: &str) -> Map<String, Value> {
    let db = suisei::db::open_in_memory().await.unwrap();
    let app = suisei::router(AppState {
        credentials: Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: None,
        },
        db: db.clone(),
        scanner: suisei::scan::Scanner::new(db, "/nonexistent".into()),
    });
    let uri = format!("/rest/{endpoint}?u=inaba&p=sesame&f=json{query}");
    let res = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    payload(serde_json::from_slice(&body).unwrap())
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
