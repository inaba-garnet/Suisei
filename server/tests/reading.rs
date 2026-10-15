//! 読みの手動設定（docs/schema.md の「日本語の並べ替え」）。

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
    dir: TempDir,
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
    put(dir.path(), "a/01.flac", "大切な事", "米津玄師", "灰色と青");
    put(dir.path(), "a/02.flac", "馬と鹿", "米津玄師", "灰色と青");
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
        Server { dir, scanner, app }
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

    async fn set(&self, id: &str, reading: Value) -> (StatusCode, Value) {
        self.send(
            Request::put(format!("/api/readings/{id}"))
                .header(header::COOKIE, format!("suisei_session={SESSION}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "reading": reading }).to_string()))
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

    async fn items(&self, query: &str) -> Vec<Value> {
        let (status, body) = self.get(&format!("/api/readings?{query}")).await;
        assert_eq!(status, StatusCode::OK);
        body["items"].as_array().unwrap().clone()
    }

    async fn artist(&self) -> Value {
        self.items("kind=artist").await.remove(0)
    }

    /// `query` で探して、見つかったアーティスト、アルバム、曲の数。
    async fn found(&self, query: &str) -> (usize, usize, usize) {
        let result = &self.rest("search3", &format!("query={query}")).await["searchResult3"];
        let count = |key: &str| result[key].as_array().map_or(0, Vec::len);
        (count("artist"), count("album"), count("song"))
    }
}

#[tokio::test]
async fn manual_reading_is_used_and_kept_by_scans() {
    let s = server().await;
    let artist = s.artist().await;
    assert_eq!(artist["name"], "米津玄師");
    assert_eq!(artist["source"], "estimated");
    let auto = artist["reading"].clone();
    let id = artist["id"].as_str().unwrap().to_owned();

    // ひらがなはカタカナにそろえる。自動の読み（ヨネヅケンシ）と違う読みで、効いたかを見分ける
    assert_eq!(s.found("はち").await, (0, 0, 0));
    let (status, body) = s.set(&id, json!(" はち ")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["reading"], "ハチ");
    assert_eq!(body["source"], "manual");
    assert_eq!(body["autoReading"], auto);

    let (status, found) = s.get(&format!("/api/readings/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(found, body);

    // 並べ替えと検索にすぐ効く。アーティストの読みで、そのアルバムと曲も見つかる
    let artists = s.rest("getArtists", "").await;
    assert_eq!(
        artists["artists"]["index"][0]["artist"][0]["sortName"],
        "ハチ"
    );
    assert_eq!(s.found("はち").await, (1, 1, 2));

    // スキャンし直しても消えない
    s.scan().await;
    let artist = s.artist().await;
    assert_eq!(artist["reading"], "ハチ");
    assert_eq!(artist["source"], "manual");
    assert_eq!(s.found("はち").await, (1, 1, 2));

    // 外すと、スキャンを待たずに自動の読みに戻る
    let (status, body) = s.set(&id, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["reading"], auto);
    assert_eq!(body["source"], "estimated");
    assert_eq!(s.found("はち").await, (0, 0, 0));
    s.scan().await;
    assert_eq!(s.artist().await["source"], "estimated");
}

#[tokio::test]
async fn manual_reading_is_dropped_while_the_name_differs() {
    let s = server().await;
    let track = s
        .items("kind=track")
        .await
        .into_iter()
        .find(|t| t["name"] == "大切な事")
        .unwrap();
    let id = track["id"].as_str().unwrap().to_owned();
    assert_eq!(s.set(&id, json!("たいせつなこと")).await.0, StatusCode::OK);

    // 曲名を直すと、手動の読みは使わない
    put(
        s.dir.path(),
        "a/01.flac",
        "大切なこと",
        "米津玄師",
        "灰色と青",
    );
    s.scan().await;
    let track = s.items("kind=track&source=manual").await;
    assert!(track.is_empty(), "{track:?}");

    // 戻せばまた使う
    put(
        s.dir.path(),
        "a/01.flac",
        "大切な事",
        "米津玄師",
        "灰色と青",
    );
    s.scan().await;
    let track = s.items("kind=track&source=manual").await;
    assert_eq!(track.len(), 1);
    assert_eq!(track[0]["id"], id.as_str());
    assert_eq!(track[0]["reading"], "タイセツナコト");
}

#[tokio::test]
async fn list_filters_and_pages() {
    let s = server().await;
    let tracks = s.items("kind=track").await;
    assert_eq!(tracks.len(), 2);
    assert!(tracks.iter().all(|t| t["artist"] == "米津玄師"));
    assert_eq!(s.items("kind=track&size=1").await.len(), 1);
    assert_eq!(
        s.items("kind=track&size=1&offset=1").await[0]["id"],
        tracks[1]["id"]
    );
    assert_eq!(s.items("kind=album&source=estimated").await.len(), 1);
    assert!(s.items("kind=album&source=none").await.is_empty());
    assert!(s.items("kind=album&source=manual").await.is_empty());
    assert_eq!(
        s.get("/api/readings?kind=genre").await.0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn rejects_bad_requests() {
    let s = server().await;
    let id = s.artist().await["id"].as_str().unwrap().to_owned();
    // 漢字の混ざった読みは五十音の行に並ばない
    let (status, body) = s.set(&id, json!("米津")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalidReading");
    assert_eq!(s.set(&id, json!(" ")).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        s.set("ar-ffffffff", json!("ア")).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(s.set("pl-1", json!("ア")).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        s.get("/api/readings/ar-ffffffff").await.0,
        StatusCode::NOT_FOUND
    );

    let (status, _) = s
        .send(
            Request::get("/api/readings?kind=artist")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = s
        .send(
            Request::put(format!("/api/readings/{id}"))
                .header(header::COOKIE, format!("suisei_session={SESSION}"))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("reading=a"))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
