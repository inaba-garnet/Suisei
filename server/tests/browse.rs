//! スキャンしたライブラリを、閲覧のエンドポイントから読む。

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
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
    /// 縮小したカバーアートの置き場所
    cache: TempDir,
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
/// ffmpeg は、テストを動かす環境にあるかどうかで結果が変わらないよう、ない場所を指す。
async fn start(dir: TempDir) -> Server {
    start_with_ffmpeg(dir, "/nonexistent/ffmpeg".into()).await
}

async fn start_with_ffmpeg(dir: TempDir, ffmpeg: std::path::PathBuf) -> Server {
    let db = suisei::db::open_in_memory().await.unwrap();
    let scanner = Scanner::new(db.clone(), dir.path().to_owned());
    assert!(scanner.start(Mode::Quick));
    scanner.wait().await;
    let cache = tempfile::tempdir().unwrap();
    let app = suisei::router(AppState {
        credentials: Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: None,
        },
        db: db.clone(),
        scanner: Arc::clone(&scanner),
        now_playing: Default::default(),
        cache_dir: cache.path().to_owned(),
        ffmpeg,
    });
    Server {
        dir,
        cache,
        app,
        db,
    }
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
async fn character_and_voice_actor_are_separate_artists() {
    let server = server(&[
        Song {
            artist: "宝鐘マリン(cv.宝鐘マリン), 因幡てゐ(cv.兎田ぺこら)",
            ..song("1.flac", "COOL&CREATE", "a")
        },
        song("2.flac", "兎田ぺこら", "b"),
    ])
    .await;

    // 声優の名前で、ほかの曲とつながる
    let pekora = server.id("artist", "name", "兎田ぺこら").await;
    let res = server.get("getArtist", &format!("&id={pekora}")).await;
    assert_eq!(res["artist"]["album"].as_array().unwrap().len(), 2);

    let album_id = server.id("album", "name", "a").await;
    let res = server.get("getAlbum", &format!("&id={album_id}")).await;
    let song = &res["album"]["song"][0];
    let artists: Vec<&str> = song["artists"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    // 同じ名前のキャラクターと声優は一人にまとめる
    assert_eq!(artists, ["宝鐘マリン", "因幡てゐ", "兎田ぺこら"]);
    // 表示用の文字列はタグのまま
    assert_eq!(
        song["displayArtist"],
        "宝鐘マリン(cv.宝鐘マリン), 因幡てゐ(cv.兎田ぺこら)"
    );
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
async fn genres_are_split_and_merged() {
    let server = server(&[
        Song {
            genre: Some("J-Pop"),
            ..song("a.flac", "ClariS", "a")
        },
        Song {
            genre: Some("J-Pop"),
            ..song("b.flac", "ClariS", "b")
        },
        Song {
            genre: Some("J-POP/General"),
            ..song("c.flac", "ClariS", "c")
        },
        Song {
            genre: Some("Rock, Pop"),
            ..song("d.flac", "ClariS", "d")
        },
    ])
    .await;
    let res = server.get("getGenres", "").await;
    assert_eq!(
        res["genres"]["genre"],
        serde_json::json!([
            { "value": "General", "songCount": 1, "albumCount": 1 },
            { "value": "J-Pop", "songCount": 3, "albumCount": 3 },
            { "value": "Pop", "songCount": 1, "albumCount": 1 },
            { "value": "Rock", "songCount": 1, "albumCount": 1 },
        ])
    );

    // 表示名と違う表記で求めても見つかる
    let res = server
        .get("getAlbumList2", "&type=byGenre&genre=j-pop&size=10")
        .await;
    assert_eq!(names(&res["albumList2"]["album"]), ["a", "b", "c"]);

    let res = server
        .get("getAlbumList2", "&type=byGenre&genre=General")
        .await;
    let album_id = res["albumList2"]["album"][0]["id"].as_str().unwrap();
    let res = server.get("getAlbum", &format!("&id={album_id}")).await;
    let song = &res["album"]["song"][0];
    assert_eq!(song["genre"], "J-Pop");
    assert_eq!(
        song["genres"],
        serde_json::json!([{ "name": "J-Pop" }, { "name": "General" }])
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

    let (status, headers, body) = server.bytes("stream", &format!("&id={id}"), None).await;
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

/// b（ClariS）を 2 回、a（やなぎなぎ）を 1 回、あとから再生する。
async fn played_server() -> Server {
    let server = album_list_server().await;
    let a = server.id("track", "title", "a").await;
    let b = server.id("track", "title", "b").await;
    let query = format!("&id={b}&time=1700000000000&id={b}&time=1700000050000");
    server.get("scrobble", &query).await;
    let query = format!("&id={a}&time=1700000100000");
    server.get("scrobble", &query).await;
    server
}

#[tokio::test]
async fn scrobble_counts_plays() {
    let server = played_server().await;
    let b = server.id("track", "title", "b").await;

    let res = server.get("getSong", &format!("&id={b}")).await;
    assert_eq!(res["song"]["playCount"], 2);
    assert_eq!(res["song"]["played"], "2023-11-14T22:14:10.000Z");
    let album = server.id("album", "name", "b").await;
    let res = server.get("getAlbum", &format!("&id={album}")).await;
    assert_eq!(res["album"]["playCount"], 2);
    assert_eq!(res["album"]["played"], "2023-11-14T22:14:10.000Z");

    // 再生していない曲には played を付けない
    let c = server.id("track", "title", "c").await;
    let res = server.get("getSong", &format!("&id={c}")).await;
    assert_eq!(res["song"]["playCount"], 0);
    assert!(res["song"].get("played").is_none());
}

/// オフラインで溜めた再生の再送では、回数を増やさない。
#[tokio::test]
async fn resent_scrobble_is_counted_once() {
    let server = played_server().await;
    let b = server.id("track", "title", "b").await;
    server
        .get("scrobble", &format!("&id={b}&time=1700000000000"))
        .await;

    let res = server.get("getSong", &format!("&id={b}")).await;
    assert_eq!(res["song"]["playCount"], 2);
}

/// 知らない ID は飛ばし、残りを記録する。
#[tokio::test]
async fn scrobble_skips_unknown_ids() {
    let server = album_list_server().await;
    let c = server.id("track", "title", "c").await;
    server
        .get("scrobble", &format!("&id=tr-00000000&id={c}"))
        .await;

    let res = server.get("getSong", &format!("&id={c}")).await;
    assert_eq!(res["song"]["playCount"], 1);
}

#[tokio::test]
async fn scrobble_requires_id() {
    let server = album_list_server().await;
    let res = server.raw("scrobble", "").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 10);
}

#[tokio::test]
async fn album_list_by_plays() {
    let server = played_server().await;
    let list = |kind: &'static str| {
        let server = &server;
        async move {
            let res = server.get("getAlbumList2", &format!("&type={kind}")).await;
            names(&res["albumList2"]["album"])
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        }
    };
    // 再生したことのない c は含めない
    assert_eq!(list("recent").await, ["a", "b"]);
    assert_eq!(list("frequent").await, ["b", "a"]);
}

#[tokio::test]
async fn top_songs_follow_play_count() {
    let server = played_server().await;
    let res = server.get("getTopSongs", "&artist=ClariS").await;
    assert_eq!(titles(&res["topSongs"]["song"]), ["b"]);
    let res = server.get("getTopSongs", "&artist=4U").await;
    assert!(res["topSongs"]["song"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn now_playing_until_submission() {
    let server = album_list_server().await;
    let a = server.id("track", "title", "a").await;
    server
        .get("scrobble", &format!("&id={a}&submission=false&c=Symfonium"))
        .await;

    let res = server.get("getNowPlaying", "").await;
    let entry = &res["nowPlaying"]["entry"][0];
    assert_eq!(entry["id"], a.as_str());
    assert_eq!(entry["username"], "inaba");
    assert_eq!(entry["playerName"], "Symfonium");
    assert_eq!(entry["minutesAgo"], 0);

    // 再生を終えたら外す
    server
        .get("scrobble", &format!("&id={a}&c=Symfonium"))
        .await;
    let res = server.get("getNowPlaying", "").await;
    assert_eq!(res["nowPlaying"], serde_json::json!({}));
}

/// 曲の長さ（1 秒）を過ぎたものは返さない。
#[tokio::test]
async fn now_playing_expires_after_duration() {
    let server = album_list_server().await;
    let a = server.id("track", "title", "a").await;
    server
        .get("scrobble", &format!("&id={a}&submission=false&c=Symfonium"))
        .await;

    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let res = server.get("getNowPlaying", "").await;
    assert_eq!(res["nowPlaying"], serde_json::json!({}));
}

#[tokio::test]
async fn star_and_unstar() {
    let server = album_list_server().await;
    let song = server.id("track", "title", "a").await;
    let album = server.id("album", "name", "b").await;
    let artist = server.id("artist", "name", "4U").await;
    let query = format!("&id={song}&albumId={album}&artistId={artist}&id=tr-00000000");
    server.get("star", &query).await;

    let res = server.get("getSong", &format!("&id={song}")).await;
    assert!(res["song"]["starred"].is_string());
    let res = server.get("getAlbum", &format!("&id={album}")).await;
    assert!(res["album"]["starred"].is_string());
    let res = server.get("getArtist", &format!("&id={artist}")).await;
    assert!(res["artist"]["starred"].is_string());
    let res = server.get("getArtists", "").await;
    let starred: Vec<&str> = res["artists"]["index"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|i| i["artist"].as_array().unwrap())
        .filter(|a| a.get("starred").is_some())
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert_eq!(starred, ["4U"]);

    let res = server.get("getStarred2", "").await;
    assert_eq!(titles(&res["starred2"]["song"]), ["a"]);
    assert_eq!(names(&res["starred2"]["album"]), ["b"]);
    assert_eq!(names(&res["starred2"]["artist"]), ["4U"]);
    let res = server.get("getAlbumList2", "&type=starred").await;
    assert_eq!(names(&res["albumList2"]["album"]), ["b"]);

    server.get("unstar", &query).await;
    let res = server.get("getSong", &format!("&id={song}")).await;
    assert!(res["song"].get("starred").is_none());
    let res = server.get("getStarred2", "").await;
    assert_eq!(res["starred2"], serde_json::json!({}));
}

/// お気に入りにし直しても日時は変えない。
#[tokio::test]
async fn restar_keeps_date() {
    let server = album_list_server().await;
    let song = server.id("track", "title", "a").await;
    server.get("star", &format!("&id={song}")).await;
    let first = server.get("getSong", &format!("&id={song}")).await["song"]["starred"].clone();

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    server.get("star", &format!("&id={song}")).await;
    let res = server.get("getSong", &format!("&id={song}")).await;
    assert_eq!(res["song"]["starred"], first);
}

#[tokio::test]
async fn rating_orders_highest() {
    let server = played_server().await;
    let a = server.id("album", "name", "a").await;
    let b = server.id("album", "name", "b").await;
    let c = server.id("album", "name", "c").await;
    for (id, rating) in [(&a, 4), (&b, 4), (&c, 5)] {
        server
            .get("setRating", &format!("&id={id}&rating={rating}"))
            .await;
    }

    let res = server.get("getAlbum", &format!("&id={c}")).await;
    assert_eq!(res["album"]["userRating"], 5);
    // 同じ評価なら再生回数の多い順（b が 2 回、a が 1 回）
    let res = server.get("getAlbumList2", "&type=highest").await;
    assert_eq!(names(&res["albumList2"]["album"]), ["c", "b", "a"]);

    server.get("setRating", &format!("&id={c}&rating=0")).await;
    let res = server.get("getAlbum", &format!("&id={c}")).await;
    assert!(res["album"].get("userRating").is_none());
    let res = server.get("getAlbumList2", "&type=highest").await;
    assert_eq!(names(&res["albumList2"]["album"]), ["b", "a"]);
}

#[tokio::test]
async fn rating_errors() {
    let server = album_list_server().await;
    let song = server.id("track", "title", "a").await;
    let res = server
        .raw("setRating", &format!("&id={song}&rating=6"))
        .await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 0);
    let res = server.raw("setRating", &format!("&id={song}")).await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 10);
    // 知らない ID は飛ばす
    server.get("setRating", "&id=tr-00000000&rating=3").await;
}

#[tokio::test]
async fn search_shows_starred_artist() {
    let server = album_list_server().await;
    let artist = server.id("artist", "name", "4U").await;
    server.get("star", &format!("&artistId={artist}")).await;
    server
        .get("setRating", &format!("&id={artist}&rating=2"))
        .await;

    let res = server.get("search3", "&query=4U").await;
    let hit = &res["searchResult3"]["artist"][0];
    assert!(hit["starred"].is_string());
    assert_eq!(hit["userRating"], 2);
}

/// 作成、取得、更新、置き換え、削除を順に行う。
#[tokio::test]
async fn playlist_lifecycle() {
    let server = album_list_server().await;
    let a = server.id("track", "title", "a").await;
    let b = server.id("track", "title", "b").await;
    let c = server.id("track", "title", "c").await;

    // 同じ曲は何度でも入り、知らない曲は飛ばす
    let query = format!("&name=mix&songId={a}&songId={b}&songId={a}&songId=tr-00000000");
    let res = server.get("createPlaylist", &query).await;
    let playlist = &res["playlist"];
    let id = playlist["id"].as_str().unwrap().to_owned();
    assert!(id.starts_with("pl-"));
    assert_eq!(playlist["name"], "mix");
    assert_eq!(playlist["owner"], "inaba");
    assert_eq!(playlist["songCount"], 3);
    assert_eq!(playlist["readonly"], false);
    assert_eq!(titles(&playlist["entry"]), ["a", "b", "a"]);

    let res = server.get("getPlaylists", "").await;
    assert_eq!(res["playlists"]["playlist"][0]["id"], id.as_str());

    // 0 始まりの位置で消してから、末尾に足す
    let query = format!(
        "&playlistId={id}&name=renamed&comment=memo&public=true\
         &songIndexToRemove=0&songIndexToRemove=2&songIdToAdd={c}"
    );
    server.get("updatePlaylist", &query).await;
    let res = server.get("getPlaylist", &format!("&id={id}")).await;
    assert_eq!(res["playlist"]["name"], "renamed");
    assert_eq!(res["playlist"]["comment"], "memo");
    assert_eq!(res["playlist"]["public"], true);
    assert_eq!(titles(&res["playlist"]["entry"]), ["b", "c"]);

    // playlistId を付けた createPlaylist は曲を置き換える
    let query = format!("&playlistId={id}&songId={c}");
    let res = server.get("createPlaylist", &query).await;
    assert_eq!(titles(&res["playlist"]["entry"]), ["c"]);
    assert_eq!(res["playlist"]["name"], "renamed");

    server.get("deletePlaylist", &format!("&id={id}")).await;
    let res = server.get("getPlaylists", "").await;
    assert_eq!(res["playlists"], serde_json::json!({}));
    let res = server.raw("getPlaylist", &format!("&id={id}")).await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 70);
}

#[tokio::test]
async fn playlist_errors() {
    let server = album_list_server().await;
    let res = server.raw("createPlaylist", "").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 10);
    let res = server.raw("updatePlaylist", "&name=x").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 10);
    let res = server
        .raw("updatePlaylist", "&playlistId=pl-00000000&name=x")
        .await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 70);
    let res = server.raw("deletePlaylist", "&id=pl-00000000").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 70);
}

/// カバーアートは、画像のある最初の曲のアルバム。
#[tokio::test]
async fn playlist_cover_is_first_album_with_image() {
    let server = cover_server().await;
    let none = server.id("album", "name", "none").await;
    let embedded = server.id("album", "name", "embedded").await;
    let first = server.id("track", "album_id", &none).await;
    let second = server.id("track", "album_id", &embedded).await;

    let query = format!("&name=p&songId={first}&songId={second}");
    let res = server.get("createPlaylist", &query).await;
    assert_eq!(res["playlist"]["coverArt"], embedded.as_str());
}

/// 64×32 の PNG をフォルダの画像として置く。左半分を透明に、右半分を暗い色の雑音にする。
/// 一色だと PNG のほうが縮小した JPEG より小さくなり、元の画像が返るため。
async fn resize_server() -> Server {
    let dir = tempfile::tempdir().unwrap();
    place(&dir, &[("notag.flac", "a/1.flac")]);
    write_cover(&dir);
    start(dir).await
}

fn write_cover(dir: &TempDir) {
    std::fs::write(dir.path().join("a/cover.png"), cover_png()).unwrap();
}

fn cover_png() -> Vec<u8> {
    encode(&noise(64, 32), image::ImageFormat::Png)
}

fn noise(width: u32, height: u32) -> image::RgbaImage {
    let mut seed = 1u32;
    image::RgbaImage::from_fn(width, height, |x, _| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let v = u8::try_from(seed >> 25).unwrap();
        image::Rgba([v, v / 2, v / 3, if x < width / 2 { 0 } else { 255 }])
    })
}

fn encode(image: &image::RgbaImage, format: image::ImageFormat) -> Vec<u8> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut bytes), format)
        .unwrap();
    bytes
}

impl Server {
    /// 縮小したカバーアートのファイル名。
    fn cached_covers(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.cache.path().join("cover")) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

#[tokio::test]
async fn cover_art_is_resized_and_cached() {
    let server = resize_server().await;
    let album = server.id("album", "name", "a").await;

    let (status, headers, body) = server
        .bytes("getCoverArt", &format!("&id={album}&size=16"), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "image/jpeg");
    let image = image::load_from_memory(&body).unwrap().to_rgb8();
    // 縦横比を保ち、長いほうの辺を size に合わせる
    assert_eq!(image.dimensions(), (16, 8));
    // 透明なところは白で塗りつぶす
    assert!(image.get_pixel(0, 0).0.iter().all(|&c| c > 100));
    assert!(image.get_pixel(15, 7).0.iter().all(|&c| c < 128));

    let cached = server.cached_covers();
    assert_eq!(cached.len(), 1);
    assert!(cached[0].starts_with(&format!("{album}-16-")));

    // 二回目はキャッシュを返す
    let (_, _, again) = server
        .bytes("getCoverArt", &format!("&id={album}&size=16"), None)
        .await;
    assert_eq!(again, body);
}

/// 元の画像が size 以下なら、拡大せずに元の画像を返し、キャッシュも作らない。
#[tokio::test]
async fn small_cover_art_is_not_enlarged() {
    let server = resize_server().await;
    let album = server.id("album", "name", "a").await;

    let (_, headers, body) = server
        .bytes("getCoverArt", &format!("&id={album}&size=64"), None)
        .await;
    assert_eq!(headers["content-type"], "image/png");
    assert_eq!(image::load_from_memory(&body).unwrap().width(), 64);
    assert!(server.cached_covers().is_empty());
}

/// 画像を差し替えたら作り直し、古いキャッシュは消す。
#[tokio::test]
async fn replaced_cover_art_is_resized_again() {
    let server = resize_server().await;
    let album = server.id("album", "name", "a").await;
    server
        .bytes("getCoverArt", &format!("&id={album}&size=16"), None)
        .await;
    let before = server.cached_covers();

    let path = server.dir.path().join("a/cover.png");
    let later =
        std::fs::metadata(&path).unwrap().modified().unwrap() + std::time::Duration::from_secs(60);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();
    server
        .bytes("getCoverArt", &format!("&id={album}&size=16"), None)
        .await;
    let after = server.cached_covers();
    assert_eq!(after.len(), 1);
    assert_ne!(after, before);
}

/// 埋め込みの画像も縮小する。
#[tokio::test]
async fn embedded_cover_art_is_resized() {
    let dir = tempfile::tempdir().unwrap();
    place(&dir, &[("notag.flac", "a/1.flac")]);
    let path = dir.path().join("a/1.flac");
    let mut file = lofty::read_from_path(&path).unwrap();
    let mut tag = Tag::new(file.primary_tag_type());
    tag.set_album("a".to_owned());
    tag.push_picture(
        Picture::unchecked(cover_png())
            .pic_type(PictureType::CoverFront)
            .mime_type(MimeType::Png)
            .build(),
    );
    file.insert_tag(tag);
    file.save_to_path(&path, WriteOptions::default()).unwrap();
    let server = start(dir).await;
    let album = server.id("album", "name", "a").await;

    let (_, headers, body) = server
        .bytes("getCoverArt", &format!("&id={album}&size=16"), None)
        .await;
    assert_eq!(headers["content-type"], "image/jpeg");
    let resized = image::load_from_memory(&body).unwrap();
    assert_eq!((resized.width(), resized.height()), (16, 8));
}

/// size がなければ 1024 を上限に縮める。
#[tokio::test]
async fn cover_art_without_size_is_capped() {
    let dir = tempfile::tempdir().unwrap();
    place(&dir, &[("notag.flac", "a/1.flac")]);
    std::fs::write(
        dir.path().join("a/cover.png"),
        encode(&noise(2048, 16), image::ImageFormat::Png),
    )
    .unwrap();
    let server = start(dir).await;
    let album = server.id("album", "name", "a").await;

    let (_, headers, body) = server
        .bytes("getCoverArt", &format!("&id={album}"), None)
        .await;
    assert_eq!(headers["content-type"], "image/jpeg");
    assert_eq!(image::load_from_memory(&body).unwrap().width(), 1024);
}

/// 縮小した JPEG が元より大きくなるなら、元の画像を返す。
#[tokio::test]
async fn cover_art_is_not_resized_into_larger_file() {
    let dir = tempfile::tempdir().unwrap();
    place(&dir, &[("notag.flac", "a/1.flac")]);
    // 一色の PNG は、縮小した JPEG よりずっと小さい
    let original = encode(
        &image::RgbaImage::from_pixel(64, 32, image::Rgba([0, 0, 0, 255])),
        image::ImageFormat::Png,
    );
    std::fs::write(dir.path().join("a/cover.png"), &original).unwrap();
    let server = start(dir).await;
    let album = server.id("album", "name", "a").await;

    let (_, headers, body) = server
        .bytes("getCoverArt", &format!("&id={album}&size=63"), None)
        .await;
    assert_eq!(headers["content-type"], "image/png");
    assert_eq!(body, original);
    // 元の画像をキャッシュに置き、次からは縮小を試さない
    let cached = server.cached_covers();
    assert_eq!(cached.len(), 1);
    assert!(cached[0].ends_with(".png"));
}

/// 音声と同じ名前の `.lrc`（時刻付き）と、FLAC に埋め込んだ歌詞（時刻なし）を置く。
async fn lyrics_server() -> Server {
    let dir = tempfile::tempdir().unwrap();
    place(
        &dir,
        &[("notag.flac", "a/1.flac"), ("notag.flac", "a/2.flac")],
    );
    for (rel, title) in [("a/1.flac", "歌う曲"), ("a/2.flac", "歌詞のない曲")] {
        let path = dir.path().join(rel);
        let mut file = lofty::read_from_path(&path).unwrap();
        let mut tag = Tag::new(file.primary_tag_type());
        tag.set_title(title.to_owned());
        tag.set_artist("歌手".to_owned());
        tag.set_album("a".to_owned());
        if rel == "a/1.flac" {
            tag.insert_text(ItemKey::Lyrics, "\n一番\n\n二番\n".to_owned());
        }
        file.insert_tag(tag);
        file.save_to_path(&path, WriteOptions::default()).unwrap();
    }
    std::fs::write(
        dir.path().join("a/1.lrc"),
        "\u{feff}[ti:歌う曲]\r\n[00:01.50]はじまり\r\n[00:03.00]おわり\r\n",
    )
    .unwrap();
    start(dir).await
}

#[tokio::test]
async fn lyrics_by_song_id() {
    let server = lyrics_server().await;
    let id = server.id("track", "title", "歌う曲").await;
    let res = server.get("getLyricsBySongId", &format!("&id={id}")).await;
    let list = res["lyricsList"]["structuredLyrics"].as_array().unwrap();
    assert_eq!(list.len(), 2);

    // 時刻付きを先に並べる
    assert_eq!(list[0]["synced"], true);
    assert_eq!(list[0]["lang"], "und");
    assert_eq!(list[0]["displayTitle"], "歌う曲");
    assert_eq!(list[0]["line"][0]["start"], 1500);
    assert_eq!(list[0]["line"][1]["value"], "おわり");

    assert_eq!(list[1]["synced"], false);
    let lines: Vec<&str> = list[1]["line"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["value"].as_str().unwrap())
        .collect();
    assert_eq!(lines, ["一番", "", "二番"]);
    assert!(list[1]["line"][0].get("start").is_none());

    let id = server.id("track", "title", "歌詞のない曲").await;
    let res = server.get("getLyricsBySongId", &format!("&id={id}")).await;
    assert_eq!(res["lyricsList"], serde_json::json!({}));
    let res = server.raw("getLyricsBySongId", "&id=tr-00000000").await;
    assert_eq!(res["subsonic-response"]["error"]["code"], 70);
}

/// v1 の getLyrics は、時刻なしの本文を一つ返す。
#[tokio::test]
async fn lyrics_by_name() {
    let server = lyrics_server().await;
    let res = server
        .get(
            "getLyrics",
            "&artist=%E6%AD%8C%E6%89%8B&title=%E6%AD%8C%E3%81%86%E6%9B%B2",
        )
        .await;
    assert_eq!(res["lyrics"]["title"], "歌う曲");
    assert_eq!(res["lyrics"]["artist"], "歌手");
    assert_eq!(res["lyrics"]["value"], "一番\n\n二番");

    let res = server
        .get("getLyrics", "&artist=x&title=%E6%AD%8C%E3%81%86%E6%9B%B2")
        .await;
    assert_eq!(res["lyrics"], serde_json::json!({}));
}

/// MP3 は USLT（時刻なし）と SYLT（時刻付き）を ID3v2 から読む。
#[tokio::test]
async fn lyrics_from_id3v2() {
    use lofty::TextEncoding;
    use lofty::id3::v2::{
        BinaryFrame, Frame, FrameId, Id3v2Tag, SyncTextContentType, SynchronizedTextFrame,
        TimestampFormat, UnsynchronizedTextFrame,
    };
    use lofty::tag::TagExt;

    let dir = tempfile::tempdir().unwrap();
    place(&dir, &[("full.mp3", "a/1.mp3")]);
    let mut tag = Id3v2Tag::new();
    tag.set_title("エムピースリー".to_owned());
    tag.insert(Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
        TextEncoding::UTF8,
        *b"jpn",
        "",
        "埋め込み",
    )));
    let sylt = SynchronizedTextFrame::new(
        TextEncoding::UTF8,
        *b"eng",
        TimestampFormat::MS,
        SyncTextContentType::Lyrics,
        None,
        vec![(1000, "one".to_owned()), (2000, "two".to_owned())],
    )
    .as_bytes(WriteOptions::default())
    .unwrap();
    tag.insert(Frame::Binary(BinaryFrame::new(
        FrameId::Valid("SYLT".into()),
        sylt,
    )));
    tag.save_to_path(dir.path().join("a/1.mp3"), WriteOptions::default())
        .unwrap();
    let server = start(dir).await;

    let id = server.id("track", "title", "エムピースリー").await;
    let res = server.get("getLyricsBySongId", &format!("&id={id}")).await;
    let list = &res["lyricsList"]["structuredLyrics"];
    assert_eq!(list[0]["synced"], true);
    assert_eq!(list[0]["lang"], "eng");
    assert_eq!(list[0]["line"][1]["start"], 2000);
    assert_eq!(list[1]["synced"], false);
    assert_eq!(list[1]["lang"], "jpn");
    assert_eq!(list[1]["line"][0]["value"], "埋め込み");
}

/// ffmpeg の代わりに、引数を `<スクリプト>.args` に書き、決まった本文を出すスクリプト。
fn fake_ffmpeg(dir: &TempDir) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.path().join("ffmpeg");
    std::fs::write(
        &path,
        "#!/bin/sh\necho \"$@\" > \"$0.args\"\nprintf CONVERTED\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

struct Transcoding {
    server: Server,
    tools: TempDir,
    id: String,
}

impl Transcoding {
    async fn new() -> Self {
        let tools = tempfile::tempdir().unwrap();
        let ffmpeg = fake_ffmpeg(&tools);
        let dir = tempfile::tempdir().unwrap();
        place(&dir, &[("full.flac", "a/1.flac")]);
        let server = start_with_ffmpeg(dir, ffmpeg).await;
        let id = server.id("track", "title", "テスト曲").await;
        // 試験用の FLAC は 1 秒の無音で 1 kbps しかないので、実際の FLAC に近い値にする
        sqlx::query("UPDATE file SET bit_rate = 900")
            .execute(&server.db)
            .await
            .unwrap();
        Self { server, tools, id }
    }

    async fn stream(&self, query: &str) -> (axum::http::HeaderMap, Vec<u8>) {
        let (status, headers, body) = self
            .server
            .bytes("stream", &format!("&id={}{query}", self.id), None)
            .await;
        assert_eq!(status, 200);
        (headers, body)
    }

    /// ffmpeg に渡した引数。起動していなければ None。
    fn args(&self) -> Option<String> {
        std::fs::read_to_string(self.tools.path().join("ffmpeg.args")).ok()
    }
}

#[tokio::test]
async fn stream_without_arguments_is_original() {
    let t = Transcoding::new().await;
    let original = std::fs::read(fixture("full.flac")).unwrap();
    for query in ["", "&format=raw&maxBitRate=128", "&format=aac"] {
        let (headers, body) = t.stream(query).await;
        assert_eq!(headers["content-type"], "audio/flac", "{query}");
        assert_eq!(body, original, "{query}");
    }
    assert_eq!(t.args(), None);
}

/// maxBitRate がなく形式の指定があれば 320 kbps にする。
#[tokio::test]
async fn stream_with_format_is_transcoded() {
    let t = Transcoding::new().await;
    let (headers, body) = t.stream("&format=mp3").await;
    assert_eq!(headers["content-type"], "audio/mpeg");
    assert_eq!(headers["accept-ranges"], "none");
    assert_eq!(body, b"CONVERTED");
    let args = t.args().unwrap();
    assert!(
        args.contains("-c:a libmp3lame -f mp3 -b:a 320k pipe:1"),
        "{args}"
    );
    assert!(args.contains("a/1.flac"), "{args}");

    let (headers, _) = t.stream("&format=opus").await;
    assert_eq!(headers["content-type"], "audio/ogg");
    assert!(t.args().unwrap().contains("-c:a libopus -f ogg -b:a 320k"));
}

#[tokio::test]
async fn stream_with_limit_is_mp3() {
    let t = Transcoding::new().await;
    let (headers, _) = t.stream("&maxBitRate=128").await;
    assert_eq!(headers["content-type"], "audio/mpeg");
    assert!(t.args().unwrap().contains("-b:a 128k"));
}

/// timeOffset から変換し、見積もった長さを付ける。
#[tokio::test]
async fn transcoded_stream_starts_at_offset() {
    let t = Transcoding::new().await;
    let (headers, _) = t
        .stream("&format=mp3&timeOffset=0.5&estimateContentLength=true")
        .await;
    let args = t.args().unwrap();
    assert!(args.contains("-ss 0.500 -i"), "{args}");
    // 残り 0.5 秒 × 320 kbps
    assert_eq!(headers["content-length"], "20000");
}

#[tokio::test]
async fn download_is_never_transcoded() {
    let t = Transcoding::new().await;
    let (status, headers, _) = t
        .server
        .bytes("download", &format!("&id={}&format=mp3", t.id), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "audio/flac");
    assert_eq!(t.args(), None);
}

/// ffmpeg がなければ、元のファイルを返す。
#[tokio::test]
async fn stream_without_ffmpeg_is_original() {
    let dir = tempfile::tempdir().unwrap();
    place(&dir, &[("full.flac", "a/1.flac")]);
    let server = start_with_ffmpeg(dir, "/nonexistent/ffmpeg".into()).await;
    let id = server.id("track", "title", "テスト曲").await;
    let (status, headers, body) = server
        .bytes("stream", &format!("&id={id}&format=mp3"), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "audio/flac");
    assert_eq!(body, std::fs::read(fixture("full.flac")).unwrap());
}
