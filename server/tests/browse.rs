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

mod common;
use common::{assert_subset, navidrome, payload};

struct Server {
    _dir: TempDir,
    app: axum::Router,
    db: suisei::db::Pool,
}

struct Song<'a> {
    path: &'a str,
    title: Option<&'a str>,
    artist: &'a str,
    album: &'a str,
    album_artist: &'a str,
    artist_sort: Option<&'a str>,
    disc: Option<u32>,
    track: Option<u32>,
    genre: Option<&'a str>,
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
        if let Some(title) = song.title {
            tag.set_title(title.to_owned());
        }
        if let Some(disc) = song.disc {
            tag.set_disk(disc);
        }
        if let Some(track) = song.track {
            tag.set_track(track);
        }
        if let Some(genre) = song.genre {
            tag.set_genre(genre.to_owned());
        }
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
        db: db.clone(),
        scanner: Arc::clone(&scanner),
    });
    Server { _dir: dir, app, db }
}

impl Server {
    async fn raw(&self, endpoint: &str, query: &str) -> Value {
        let uri = format!("/rest/{endpoint}?u=inaba&p=sesame&f=json{query}");
        let res = self
            .app
            .clone()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    async fn get(&self, endpoint: &str, query: &str) -> Value {
        let value = self.raw(endpoint, query).await;
        assert_eq!(value["subsonic-response"]["status"], "ok", "{value}");
        value["subsonic-response"].clone()
    }

    async fn id(&self, table: &str, name_column: &str, name: &str) -> String {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT id FROM {table} WHERE {name_column} = ?"
        )))
        .bind(name)
        .fetch_one(&self.db)
        .await
        .unwrap()
    }
}

fn song<'a>(path: &'a str, album_artist: &'a str, album: &'a str) -> Song<'a> {
    Song {
        path,
        title: None,
        artist: album_artist,
        album,
        album_artist,
        artist_sort: None,
        disc: None,
        track: None,
        genre: None,
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

fn track<'a>(path: &'a str, title: &'a str, disc: u32, track: u32) -> Song<'a> {
    Song {
        title: Some(title),
        disc: Some(disc),
        track: Some(track),
        genre: Some("Rock"),
        ..song(path, "ClariS", "Fairy Castle")
    }
}

fn titles(songs: &Value) -> Vec<&str> {
    songs
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["title"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn album_lists_songs_in_disc_and_track_order() {
    let server = server(&[
        track("a/2-01.flac", "c", 2, 1),
        track("a/1-02.flac", "b", 1, 2),
        track("a/1-01.flac", "a", 1, 1),
        Song {
            genre: Some("Pop"),
            ..track("a/1-03.flac", "d", 1, 3)
        },
    ])
    .await;
    let album_id = server.id("album", "name", "Fairy Castle").await;

    let res = server.get("getAlbum", &format!("&id={album_id}")).await;
    let album = &res["album"];
    assert_eq!(titles(&album["song"]), ["a", "b", "d", "c"]);
    assert_eq!(album["songCount"], 4);
    assert_eq!(album["displayArtist"], "ClariS");
    assert_eq!(album["artists"][0]["name"], "ClariS");
    // ジャンルは曲数の多い順
    assert_eq!(album["genre"], "Rock");
    assert_eq!(
        album["genres"],
        serde_json::json!([{ "name": "Rock" }, { "name": "Pop" }])
    );
    assert_eq!(album["isCompilation"], false);

    let song = &album["song"][0];
    assert_eq!(song["parent"], album_id.as_str());
    assert_eq!(song["path"], "a/1-01.flac");
    assert_eq!(song["discNumber"], 1);
    assert_eq!(song["track"], 1);
    assert_eq!(song["duration"], 1);
    assert_eq!(song["suffix"], "flac");
    assert_eq!(song["albumArtists"][0]["name"], "ClariS");
}

#[tokio::test]
async fn artist_includes_albums_with_guest_appearances() {
    let server = server(&[
        song("1.flac", "ClariS", "a"),
        song("2.flac", "ClariS", "b"),
        Song {
            artist: "ゲスト",
            ..song("3.flac", "やなぎなぎ", "c")
        },
    ])
    .await;

    let claris = server.id("artist", "name", "ClariS").await;
    let res = server.get("getArtist", &format!("&id={claris}")).await;
    assert_eq!(res["artist"]["albumCount"], 2);
    assert_eq!(res["artist"]["album"].as_array().unwrap().len(), 2);

    // 曲にだけ参加しているアーティストも、参加したアルバムを返す
    let guest = server.id("artist", "name", "ゲスト").await;
    let res = server.get("getArtist", &format!("&id={guest}")).await;
    assert_eq!(res["artist"]["albumCount"], 1);
    assert_eq!(res["artist"]["album"][0]["name"], "c");
}

#[tokio::test]
async fn old_id_is_resolved_through_alias() {
    let server = server(&[track("a/1-01.flac", "a", 1, 1)]).await;
    let id = server.id("track", "title", "a").await;
    sqlx::query("INSERT INTO id_alias (old_id, new_id) VALUES ('tr-00000000', ?)")
        .bind(&id)
        .execute(&server.db)
        .await
        .unwrap();

    let res = server.get("getSong", "&id=tr-00000000").await;
    assert_eq!(res["song"]["id"], id.as_str());
}

#[tokio::test]
async fn unknown_id_is_not_found() {
    let server = server(&[track("a/1-01.flac", "a", 1, 1)]).await;
    for endpoint in ["getArtist", "getAlbum", "getSong"] {
        let res = server.raw(endpoint, "&id=xx-00000000").await;
        assert_eq!(res["subsonic-response"]["error"]["code"], 70, "{endpoint}");
    }
    let res = server.raw("getSong", "").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 10);
}

/// こちらの項目がすべて Navidrome の応答にあり、型が同じことを確かめる。
#[tokio::test]
async fn shapes_match_navidrome() {
    let server = server(&[Song {
        artist_sort: Some("Kurarisu"),
        ..track("a/1-01.flac", "a", 1, 1)
    }])
    .await;
    let artist = server.id("artist", "name", "ClariS").await;
    let album = server.id("album", "name", "Fairy Castle").await;
    let song = server.id("track", "title", "a").await;
    for (endpoint, id) in [
        ("getArtist", artist),
        ("getAlbum", album),
        ("getSong", song),
    ] {
        let ours = payload(server.raw(endpoint, &format!("&id={id}")).await);
        let theirs = navidrome(endpoint);
        assert_subset(endpoint, &Value::Object(ours), &Value::Object(theirs));
    }
}
