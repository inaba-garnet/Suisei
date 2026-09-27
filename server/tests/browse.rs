//! スキャンしたライブラリを、閲覧のエンドポイントから読む。

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey, Tag};
use serde_json::Value;
use suisei::scan::{Mode, Scanner};
use suisei::{AppState, Credentials};
use tempfile::TempDir;
use tower::ServiceExt;

struct Server {
    _dir: TempDir,
    app: axum::Router,
}

struct Song<'a> {
    path: &'a str,
    artist: &'a str,
    album: &'a str,
    album_artist: &'a str,
    artist_sort: Option<&'a str>,
}

/// タグのない FLAC にタグを書いて並べ、スキャンしてからサーバーを作る。
async fn server(songs: &[Song<'_>]) -> Server {
    let dir = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tags/notag.flac");
    for song in songs {
        let path = dir.path().join(song.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(&fixture, &path).unwrap();
        let mut file = lofty::read_from_path(&path).unwrap();
        let mut tag = Tag::new(file.primary_tag_type());
        tag.set_artist(song.artist.to_owned());
        tag.set_album(song.album.to_owned());
        tag.insert_text(ItemKey::AlbumArtist, song.album_artist.to_owned());
        if let Some(sort) = song.artist_sort {
            tag.insert_text(ItemKey::TrackArtistSortOrder, sort.to_owned());
            tag.insert_text(ItemKey::AlbumArtistSortOrder, sort.to_owned());
        }
        file.insert_tag(tag);
        file.save_to_path(&path, WriteOptions::default()).unwrap();
    }

    let db = suisei::db::open_in_memory().await.unwrap();
    let scanner = Scanner::new(db.clone(), dir.path().to_owned());
    assert!(scanner.start(Mode::Quick));
    scanner.wait().await;
    let app = suisei::router(AppState {
        credentials: Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: None,
        },
        db,
        scanner: Arc::clone(&scanner),
    });
    Server { _dir: dir, app }
}

impl Server {
    async fn get(&self, endpoint: &str, query: &str) -> Value {
        let uri = format!("/rest/{endpoint}?u=inaba&p=sesame&f=json{query}");
        let res = self
            .app
            .clone()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["subsonic-response"]["status"], "ok", "{value}");
        value["subsonic-response"].clone()
    }
}

fn song<'a>(path: &'a str, album_artist: &'a str, album: &'a str) -> Song<'a> {
    Song {
        path,
        artist: album_artist,
        album,
        album_artist,
        artist_sort: None,
    }
}

/// 見出しと、その下のアーティスト名
fn headings(index: &Value) -> Vec<(String, Vec<String>)> {
    index
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            let names = i["artist"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["name"].as_str().unwrap().to_owned())
                .collect();
            (i["name"].as_str().unwrap().to_owned(), names)
        })
        .collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).to_owned()).collect()
}

#[tokio::test]
async fn artists_are_grouped_by_reading() {
    let server = server(&[
        song("1.flac", "田村ゆかり", "a"),
        Song {
            artist_sort: Some("Yonezu, Kenshi"),
            ..song("2.flac", "米津玄師", "b")
        },
        song("3.flac", "やなぎなぎ", "c"),
        song("4.flac", "サカナクション", "d"),
        song("5.flac", "ClariS", "e"),
        song("6.flac", "ClariS", "f"),
        song("7.flac", "4U", "g"),
        song("8.flac", "[Alexandros]", "h"),
        // 曲にだけ参加しているアーティストは載せない
        Song {
            artist: "ゲスト",
            ..song("9.flac", "ClariS", "e")
        },
    ])
    .await;

    let res = server.get("getArtists", "").await;
    let artists = &res["artists"];
    assert_eq!(
        headings(&artists["index"]),
        [
            ("#".to_owned(), strings(&["[Alexandros]", "4U"])),
            ("C".to_owned(), strings(&["ClariS"])),
            ("さ".to_owned(), strings(&["サカナクション"])),
            ("や".to_owned(), strings(&["やなぎなぎ", "米津玄師"])),
            ("他".to_owned(), strings(&["田村ゆかり"])),
        ]
    );
    let clairs = &artists["index"][1]["artist"][0];
    assert_eq!(clairs["albumCount"], 2);
    assert!(clairs.get("sortName").is_none());
    let yonezu = &artists["index"][3]["artist"][1];
    assert_eq!(yonezu["sortName"], "ヨネズ ケンシ");
    assert!(artists["lastModified"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn indexes_respect_if_modified_since() {
    let server = server(&[song("1.flac", "ClariS", "a")]).await;

    let res = server.get("getIndexes", "").await;
    let indexes = &res["indexes"];
    assert_eq!(
        headings(&indexes["index"]),
        [("C".to_owned(), strings(&["ClariS"]))]
    );
    let last_modified = indexes["lastModified"].as_i64().unwrap();

    let res = server
        .get("getIndexes", &format!("&ifModifiedSince={last_modified}"))
        .await;
    assert!(res["indexes"].get("index").is_none());
    assert_eq!(res["indexes"]["lastModified"], last_modified);
}
