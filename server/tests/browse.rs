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
    dir: TempDir,
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
    year: Option<u32>,
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
        if let Some(year) = song.year {
            tag.insert_text(ItemKey::RecordingDate, year.to_string());
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
    start(dir).await
}

/// 並べ終えたディレクトリをスキャンして、サーバーを作る。
async fn start(dir: TempDir) -> Server {
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
    Server { dir, app, db }
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

    /// 本文をそのまま返す。`range` を渡すと Range ヘッダを付ける。
    async fn bytes(
        &self,
        endpoint: &str,
        query: &str,
        range: Option<&str>,
    ) -> (axum::http::StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let uri = format!("/rest/{endpoint}?u=inaba&p=sesame&f=json{query}");
        let mut request = Request::get(uri);
        if let Some(range) = range {
            request = request.header("range", range);
        }
        let res = self
            .app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, headers, body.to_vec())
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
        year: None,
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

fn names(albums: &Value) -> Vec<&str> {
    albums
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect()
}

async fn album_list_server() -> Server {
    server(&[
        Song {
            genre: Some("Rock"),
            year: Some(2010),
            ..song("b.flac", "ClariS", "b")
        },
        Song {
            genre: Some("Pop"),
            year: Some(2020),
            ..song("a.flac", "やなぎなぎ", "a")
        },
        Song {
            genre: Some("Rock"),
            year: Some(2015),
            ..song("c.flac", "4U", "c")
        },
    ])
    .await
}

#[tokio::test]
async fn album_list_orders() {
    let server = album_list_server().await;
    let list = |query: &'static str| {
        let server = &server;
        async move {
            let res = server.get("getAlbumList2", query).await;
            names(&res["albumList2"]["album"])
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        }
    };

    assert_eq!(
        list("&type=alphabeticalByName&size=10").await,
        ["a", "b", "c"]
    );
    assert_eq!(
        list("&type=alphabeticalByArtist&size=10").await,
        ["c", "b", "a"]
    );
    assert_eq!(
        list("&type=byYear&fromYear=2011&toYear=2020").await,
        ["c", "a"]
    );
    // fromYear の方が大きければ新しい年から
    assert_eq!(
        list("&type=byYear&fromYear=2020&toYear=2011").await,
        ["a", "c"]
    );
    assert_eq!(list("&type=byGenre&genre=Rock").await, ["b", "c"]);
    assert_eq!(
        list("&type=alphabeticalByName&size=1&offset=1").await,
        ["b"]
    );
    assert_eq!(list("&type=random").await.len(), 3);
    // 再生履歴やお気に入りを持つまでは空
    assert!(list("&type=recent").await.is_empty());
    assert!(list("&type=starred").await.is_empty());

    sqlx::query("UPDATE album SET created_at = created_at + 1000 WHERE name = 'c'")
        .execute(&server.db)
        .await
        .unwrap();
    assert_eq!(list("&type=newest").await, ["c", "a", "b"]);
}

#[tokio::test]
async fn album_list_errors() {
    let server = album_list_server().await;
    let res = server.raw("getAlbumList2", "").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 10);
    let res = server.raw("getAlbumList2", "&type=byYear").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 10);
    let res = server.raw("getAlbumList2", "&type=unknown").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 0);
}

#[tokio::test]
async fn genres_count_songs_and_albums() {
    let server = album_list_server().await;
    let res = server.get("getGenres", "").await;
    assert_eq!(
        res["genres"]["genre"],
        serde_json::json!([
            { "value": "Pop", "songCount": 1, "albumCount": 1 },
            { "value": "Rock", "songCount": 2, "albumCount": 2 },
        ])
    );
}

#[tokio::test]
async fn list_shapes_match_navidrome() {
    let server = album_list_server().await;
    for (endpoint, query) in [
        ("getAlbumList2", "&type=alphabeticalByName"),
        ("getGenres", ""),
    ] {
        let ours = payload(server.raw(endpoint, query).await);
        let theirs = navidrome(endpoint);
        assert_subset(endpoint, &Value::Object(ours), &Value::Object(theirs));
    }
}

async fn search_server() -> Server {
    server(&[
        Song {
            title: Some("感電"),
            artist_sort: Some("Yonezu, Kenshi"),
            ..song("1.flac", "米津玄師", "STRAY SHEEP")
        },
        Song {
            title: Some("again"),
            ..song("2.flac", "ClariS", "Fairy Castle")
        },
        Song {
            title: Some("border"),
            artist: "ゲスト",
            ..song("3.flac", "ClariS", "Fairy Castle")
        },
    ])
    .await
}

fn search_names(result: &Value, kind: &str, field: &str) -> Vec<String> {
    result[kind]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|i| i[field].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn search_with_empty_query_returns_everything() {
    let server = search_server().await;
    // Symfonium は引用符二つ、substreamer は空の文字列で全件を求める
    for query in ["%22%22", ""] {
        let res = server
            .get(
                "search3",
                &format!("&query={query}&artistCount=500&albumCount=500&songCount=500"),
            )
            .await;
        let result = &res["searchResult3"];
        // 曲にだけ参加しているアーティストも含める
        assert_eq!(
            search_names(result, "artist", "name"),
            ["ClariS", "ゲスト", "米津玄師"]
        );
        assert_eq!(
            search_names(result, "album", "name"),
            ["Fairy Castle", "STRAY SHEEP"]
        );
        assert_eq!(search_names(result, "song", "title").len(), 3);
    }
}

#[tokio::test]
async fn search_pages_without_gaps() {
    let server = search_server().await;
    let mut titles = Vec::new();
    for offset in 0..3 {
        let res = server
            .get(
                "search3",
                &format!("&query=&artistCount=0&albumCount=0&songCount=1&songOffset={offset}"),
            )
            .await;
        titles.extend(search_names(&res["searchResult3"], "song", "title"));
    }
    titles.sort();
    assert_eq!(titles, ["again", "border", "感電"]);
}

#[tokio::test]
async fn search_matches_reading_and_all_words() {
    let server = search_server().await;
    // 読み（ヨネズ ケンシ）にひらがなで一致する
    let res = server
        .get("search3", "&query=%E3%82%88%E3%81%AD%E3%81%9A")
        .await;
    let result = &res["searchResult3"];
    assert_eq!(search_names(result, "artist", "name"), ["米津玄師"]);
    assert_eq!(search_names(result, "album", "name"), ["STRAY SHEEP"]);
    assert_eq!(search_names(result, "song", "title"), ["感電"]);

    // 語はすべてを含むものだけ。曲はアーティスト名とアルバム名でも一致する
    let res = server.get("search3", "&query=fairy%20BORDER").await;
    let result = &res["searchResult3"];
    assert!(result.get("artist").is_none());
    assert!(result.get("album").is_none());
    assert_eq!(search_names(result, "song", "title"), ["border"]);
}

#[tokio::test]
async fn search_shapes_match_navidrome() {
    let server = search_server().await;
    let res = server
        .raw(
            "search3",
            "&query=claris&artistCount=1&albumCount=1&songCount=1",
        )
        .await;
    let ours = payload(res);
    let result = &ours["searchResult3"];
    let artists = navidrome("getArtists");
    let albums = navidrome("getAlbumList2");
    let songs = navidrome("search3");
    assert_subset(
        "artist",
        &result["artist"],
        &artists["artists"]["index"][0]["artist"],
    );
    assert_subset("album", &result["album"], &albums["albumList2"]["album"]);
    assert_subset("song", &result["song"], &songs["searchResult3"]["song"]);
}

#[tokio::test]
async fn stream_returns_the_file() {
    let server = server(&[track("a/1-01.flac", "a", 1, 1)]).await;
    let id = server.id("track", "title", "a").await;
    let original = std::fs::read(server.dir.path().join("a/1-01.flac")).unwrap();

    // トランスコードはしないので、maxBitRate と format は無視して元のファイルを返す
    let (status, headers, body) = server
        .bytes(
            "stream",
            &format!("&id={id}&maxBitRate=128&format=mp3"),
            None,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "audio/flac");
    assert_eq!(body, original);

    // シークのための部分取得
    let (status, headers, body) = server
        .bytes("stream", &format!("&id={id}"), Some("bytes=4-9"))
        .await;
    assert_eq!(status, 206);
    assert_eq!(
        headers["content-range"],
        format!("bytes 4-9/{}", original.len()).as_str()
    );
    assert_eq!(body, original[4..10]);
}

#[tokio::test]
async fn download_names_the_file() {
    let server = server(&[track("a/1-01 曲.flac", "a", 1, 1)]).await;
    let id = server.id("track", "title", "a").await;
    let (status, headers, _) = server.bytes("download", &format!("&id={id}"), None).await;
    assert_eq!(status, 200);
    assert_eq!(
        headers["content-disposition"],
        "attachment; filename*=UTF-8''1-01%20%E6%9B%B2.flac"
    );
}

#[tokio::test]
async fn stream_errors() {
    let server = server(&[track("a/1-01.flac", "a", 1, 1)]).await;
    let res = server.raw("stream", "&id=tr-00000000").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 70);
    let res = server.raw("stream", "").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 10);

    // スキャンの後にファイルが消えた
    let id = server.id("track", "title", "a").await;
    std::fs::remove_file(server.dir.path().join("a/1-01.flac")).unwrap();
    let res = server.raw("stream", &format!("&id={id}")).await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 70);
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tags")
        .join(name)
}

/// テスト用の音声や画像を、ディレクトリに並べる。
fn place(dir: &TempDir, files: &[(&str, &str)]) {
    for (source, rel) in files {
        let path = dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(fixture(source), path).unwrap();
    }
}

async fn cover_server() -> Server {
    let dir = tempfile::tempdir().unwrap();
    place(
        &dir,
        &[
            // フォルダの画像が埋め込みより先
            ("picture.flac", "folder/1.flac"),
            ("cover.png", "folder/Folder.PNG"),
            // 埋め込みだけ
            ("picture.flac", "embedded/1.flac"),
            // どちらもない
            ("notag.flac", "none/1.flac"),
        ],
    );
    // 同じタグのファイルが一枚にまとまらないよう、アルバム名を変える
    for (rel, album) in [("folder/1.flac", "folder"), ("embedded/1.flac", "embedded")] {
        let path = dir.path().join(rel);
        let mut file = lofty::read_from_path(&path).unwrap();
        file.primary_tag_mut().unwrap().set_album(album.to_owned());
        file.save_to_path(&path, WriteOptions::default()).unwrap();
    }
    start(dir).await
}

#[tokio::test]
async fn cover_art_prefers_folder_image() {
    let server = cover_server().await;
    let png = std::fs::read(fixture("cover.png")).unwrap();

    for album in ["folder", "embedded"] {
        let id = server.id("album", "name", album).await;
        let (status, headers, body) = server
            .bytes("getCoverArt", &format!("&id={id}"), None)
            .await;
        assert_eq!(status, 200, "{album}");
        assert_eq!(headers["content-type"], "image/png", "{album}");
        assert_eq!(body, png, "{album}");

        // 曲の ID でも、アルバムの画像を返す
        let res = server.get("getAlbum", &format!("&id={id}")).await;
        assert_eq!(res["album"]["coverArt"], id.as_str());
        let song = res["album"]["song"][0]["id"].as_str().unwrap().to_owned();
        assert_eq!(res["album"]["song"][0]["coverArt"], id.as_str());
        let (status, _, body) = server
            .bytes("getCoverArt", &format!("&id={song}"), None)
            .await;
        assert_eq!(status, 200);
        assert_eq!(body, png);
    }
    let path: Option<String> =
        sqlx::query_scalar("SELECT cover_path FROM album WHERE name = 'folder'")
            .fetch_one(&server.db)
            .await
            .unwrap();
    assert_eq!(path.as_deref(), Some("folder/Folder.PNG"));
}

#[tokio::test]
async fn album_without_cover_has_no_cover_art() {
    let server = cover_server().await;
    let id = server.id("album", "name", "none").await;
    let res = server.get("getAlbum", &format!("&id={id}")).await;
    assert!(res["album"].get("coverArt").is_none());
    assert!(res["album"]["song"][0].get("coverArt").is_none());
    let res = server.raw("getCoverArt", &format!("&id={id}")).await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 70);
    let res = server.raw("getCoverArt", "&id=al-00000000").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 70);
}
