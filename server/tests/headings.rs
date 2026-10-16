//! 一覧の頭文字の索引に使う、頭文字ごとの先頭の位置（docs/web.md の「一覧」）。

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey, Tag};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use suisei::scan::{Mode, Scanner};
use suisei::{AppState, Credentials};
use tempfile::TempDir;
use tower::ServiceExt;

const SESSION: &str = "session-token";

struct Server {
    _dir: TempDir,
    scanner: Arc<Scanner>,
    app: axum::Router,
}

/// 曲名、アーティスト、アルバムを書いた FLAC を置く。
fn put(dir: &Path, rel: &str, title: &str, artist: &str, album: &str) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tags/notag.flac");
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::copy(&fixture, &path).unwrap();
    let mut file = lofty::read_from_path(&path).unwrap();
    let mut tag = Tag::new(file.primary_tag_type());
    tag.set_title(title.to_owned());
    tag.set_artist(artist.to_owned());
    tag.set_album(album.to_owned());
    tag.insert_text(ItemKey::AlbumArtist, artist.to_owned());
    file.insert_tag(tag);
    file.save_to_path(&path, WriteOptions::default()).unwrap();
}

async fn server() -> Server {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "a/01.flac", "zz", "Bee", "123");
    put(dir.path(), "b/01.flac", "Hello", "Zed", "Abc");
    put(dir.path(), "c/01.flac", "いろは", "Bee", "あいうえお");
    put(dir.path(), "d/01.flac", "さくら", "あ", "かきく");
    let db = suisei::db::open_in_memory().await.unwrap();
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    suisei::db::session::create(&db, &hex::encode(Sha256::digest(SESSION)), now)
        .await
        .unwrap();
    let scanner = Scanner::new(db.clone(), dir.path().to_owned(), Default::default());
    let server = {
        let settings = suisei::settings::Store::new(db.clone(), Default::default());
        let app = suisei::router(AppState {
            credentials: Credentials {
                user: "inaba".into(),
                password: "sesame".into(),
                api_key: None,
            },
            db: db.clone(),
            scanner: Arc::clone(&scanner),
            now_playing: Default::default(),
            cache_dir: "/nonexistent".into(),
            ffmpeg: "/nonexistent/ffmpeg".into(),
            dev: false,
            throttle: Default::default(),
            trust_forwarded_for: false,
            spotify: suisei::spotify::Spotify::new(
                db.clone(),
                settings.clone(),
                Default::default(),
            ),
            settings,
        });
        Server {
            _dir: dir,
            scanner,
            app,
        }
    };
    server.scan().await;
    server
}

impl Server {
    async fn scan(&self) {
        assert!(self.scanner.start(Mode::Full));
        self.scanner.wait().await;
    }

    async fn send(&self, req: Request<Body>) -> (StatusCode, Value) {
        let res = self.app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.send(
            Request::get(uri)
                .header(header::COOKIE, format!("suisei_session={SESSION}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn rest(&self, endpoint: &str, query: &str) -> Value {
        let uri = format!("/rest/{endpoint}?u=inaba&p=sesame&f=json&{query}");
        self.send(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .1["subsonic-response"]
            .clone()
    }

    /// 頭文字と先頭の位置の組と、全体の件数。
    async fn headings(&self, query: &str) -> (Value, Value) {
        let (status, body) = self.get(&format!("/api/headings?{query}")).await;
        assert_eq!(status, StatusCode::OK);
        (body["headings"].clone(), body["total"].clone())
    }
}

fn pairs(list: &[(&str, i64)]) -> Value {
    Value::Array(
        list.iter()
            .map(|(heading, offset)| json!({ "heading": heading, "offset": offset }))
            .collect(),
    )
}

#[tokio::test]
async fn album_headings_follow_the_album_list() {
    let s = server().await;
    let names = s.rest("getAlbumList2", "type=alphabeticalByName").await["albumList2"]["album"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(names, ["123", "Abc", "あいうえお", "かきく"]);
    assert_eq!(
        s.headings("list=albums&sort=name").await,
        (pairs(&[("#", 0), ("A", 1), ("あ", 2), ("か", 3)]), json!(4))
    );
    // アーティスト順は先頭のアルバムアーティストで分ける
    assert_eq!(
        s.headings("list=albums&sort=artist").await,
        (pairs(&[("B", 0), ("Z", 2), ("あ", 3)]), json!(4))
    );

    // お気に入りに絞ると、絞った一覧の中の位置を返す
    let abc = s.rest("search3", "query=Abc").await["searchResult3"]["album"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        s.rest("star", &format!("albumId={abc}")).await["status"],
        "ok"
    );
    assert_eq!(
        s.headings("list=albums&sort=name&starred=true").await,
        (pairs(&[("A", 0)]), json!(1))
    );
}

#[tokio::test]
async fn track_headings_follow_the_song_sort() {
    let s = server().await;
    assert_eq!(
        s.headings("list=tracks&sort=title").await,
        (pairs(&[("H", 0), ("Z", 1), ("あ", 2), ("さ", 3)]), json!(4))
    );
    assert_eq!(
        s.headings("list=tracks&sort=album").await,
        (pairs(&[("#", 0), ("A", 1), ("あ", 2), ("か", 3)]), json!(4))
    );
    assert_eq!(
        s.headings("list=tracks&sort=artist").await,
        (pairs(&[("B", 0), ("Z", 2), ("あ", 3)]), json!(4))
    );
}

#[tokio::test]
async fn headings_reject_unknown_lists_and_require_a_session() {
    let s = server().await;
    assert_eq!(
        s.get("/api/headings?list=albums&sort=newest").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        s.get("/api/headings?list=artists&sort=name").await.0,
        StatusCode::BAD_REQUEST
    );
    let (status, _) = s
        .send(
            Request::get("/api/headings?list=tracks&sort=title")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
