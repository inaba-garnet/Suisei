//! Spotify のお気に入りの取り込み（docs/spotify.md）。Spotify の API は偽物のサーバーで置き換える。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use http_body_util::BodyExt;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use suisei::db::{self, Pool};
use suisei::scan::{Mode, Scanner};
use suisei::spotify::{Endpoints, REDIRECT_URI, Spotify};
use suisei::{AppState, Credentials};
use tempfile::TempDir;
use tower::ServiceExt;

const ADDED_AT: &str = "2024-05-01T12:00:00Z";
const ADDED_AT_MS: i64 = 1_714_564_800_000;
const SESSION: &str = "session-token";

/// 偽物の Spotify。受け取った要求を数え、決まった応答を返す。
#[derive(Default)]
struct Mock {
    tracks: Vec<Value>,
    /// 発行したアクセストークン
    access_tokens: Vec<String>,
    /// 取り消されたリフレッシュトークン
    revoked: Vec<String>,
    track_requests: usize,
}

type Shared = Arc<Mutex<Mock>>;

async fn token(State(mock): State<Shared>, Form(form): Form<HashMap<String, String>>) -> Response {
    let mut mock = mock.lock().unwrap();
    assert_eq!(form["client_id"], "client");
    let refresh = match form["grant_type"].as_str() {
        "authorization_code" => {
            assert_eq!(form["code"], "good-code");
            assert_eq!(form["redirect_uri"], REDIRECT_URI);
            assert!(!form["code_verifier"].is_empty());
            "refresh-1".to_owned()
        }
        "refresh_token" => {
            let given = &form["refresh_token"];
            if mock.revoked.contains(given) {
                return (
                    StatusCode::BAD_REQUEST,
                    axum::Json(json!({ "error": "invalid_grant" })),
                )
                    .into_response();
            }
            // 使うたびに替える
            format!("{given}-rotated")
        }
        other => panic!("unexpected grant_type {other}"),
    };
    let access = format!("access-{}", mock.access_tokens.len());
    mock.access_tokens.push(access.clone());
    axum::Json(json!({
        "access_token": access,
        "token_type": "Bearer",
        "expires_in": 3600,
        "refresh_token": refresh,
        "scope": "user-library-read",
    }))
    .into_response()
}

async fn saved_tracks(
    State(mock): State<Shared>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, usize>>,
) -> Response {
    let mut mock = mock.lock().unwrap();
    let auth = headers[header::AUTHORIZATION].to_str().unwrap();
    if !mock
        .access_tokens
        .iter()
        .any(|t| auth == format!("Bearer {t}"))
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    mock.track_requests += 1;
    let (offset, limit) = (query["offset"], query["limit"]);
    let items: Vec<Value> = mock
        .tracks
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    let next = (offset + limit < mock.tracks.len()).then_some("https://example.invalid/next");
    axum::Json(json!({ "items": items, "next": next, "total": mock.tracks.len() })).into_response()
}

fn item(id: &str, isrc: Option<&str>, title: &str, album: &str, track: u32) -> Value {
    json!({
        "added_at": ADDED_AT,
        "track": {
            "id": id,
            "name": title,
            "duration_ms": 1000,
            "disc_number": if track == 3 { 2 } else { 1 },
            "track_number": track,
            "external_ids": isrc.map_or(json!({}), |isrc| json!({ "isrc": isrc })),
            "artists": [{ "name": "歌手A" }, { "name": "歌手B" }],
            "album": { "name": album },
        }
    })
}

struct Fixture {
    dir: TempDir,
    pool: Pool,
    mock: Shared,
    spotify: Arc<Spotify>,
    router: Router,
}

impl Fixture {
    async fn new() -> Self {
        let mut tracks = vec![
            // ローカルに同じ ISRC の曲が二つあるので、盤まで一致する曲（a/）に付く
            item(
                "s-same",
                Some("JPXX01500001"),
                "テスト曲",
                "テストアルバム",
                3,
            ),
            // 盤が違うので、曲名とアーティストと長さで b/ に付く
            item("s-single", None, "二曲目", "シングル", 1),
            // 後で足す c/ に付く
            item("s-later", None, "三曲目", "シングル", 1),
        ];
        // 一ページ（50 曲）に収まらない数にする
        for n in 0..50 {
            tracks.push(item(
                &format!("s-{n}"),
                None,
                &format!("ない曲{n}"),
                "どこか",
                1,
            ));
        }
        let mock: Shared = Arc::new(Mutex::new(Mock {
            tracks,
            ..Mock::default()
        }));
        let app = Router::new()
            .route("/api/token", post(token))
            .route("/v1/me/tracks", get(saved_tracks))
            .with_state(mock.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let dir = tempfile::tempdir().unwrap();
        let pool = db::open_in_memory().await.unwrap();
        db::session::create(&pool, &hex::encode(Sha256::digest(SESSION)), now_ms())
            .await
            .unwrap();
        let spotify = Spotify::new(
            pool.clone(),
            "client".into(),
            Endpoints {
                accounts: base.clone(),
                api: base,
            },
            Default::default(),
        );
        let router = suisei::router(AppState {
            credentials: Credentials {
                user: "inaba".into(),
                password: "sesame".into(),
                api_key: None,
            },
            db: pool.clone(),
            scanner: Scanner::new(pool.clone(), dir.path().to_owned(), Default::default()),
            now_playing: Default::default(),
            cache_dir: "/nonexistent".into(),
            ffmpeg: "ffmpeg".into(),
            dev: false,
            throttle: Default::default(),
            trust_forwarded_for: false,
            spotify: Some(spotify.clone()),
        });
        let fixture = Self {
            dir,
            pool,
            mock,
            spotify,
            router,
        };
        fixture.put("a/01.flac", None);
        fixture.put("b/02.flac", Some("二曲目"));
        fixture
    }

    /// tests/fixtures/tags/full.flac を置く。`title` を渡せば曲名を書き換える（ISRC は残る）。
    fn put(&self, rel: &str, title: Option<&str>) {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tags/full.flac");
        let dst = self.dir.path().join(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src, &dst).unwrap();
        if let Some(title) = title {
            let mut file = lofty::read_from_path(&dst).unwrap();
            file.primary_tag_mut().unwrap().set_title(title.to_owned());
            file.save_to_path(&dst, WriteOptions::default()).unwrap();
        }
    }

    async fn request(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::COOKIE, format!("suisei_session={SESSION}"))
            .header(header::CONTENT_TYPE, "application/json");
        if body.is_none() && method == "GET" {
            req = Request::builder()
                .method(method)
                .uri(uri)
                .header(header::COOKIE, format!("suisei_session={SESSION}"));
        }
        let req = req
            .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
            .unwrap();
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
    }

    /// 認可を始め、Spotify から戻った URL を貼り付けて接続する。
    async fn connect(&self) {
        let (status, body) = self.request("POST", "/api/spotify/authorize", None).await;
        assert_eq!(status, StatusCode::OK);
        let url = reqwest::Url::parse(body["url"].as_str().unwrap()).unwrap();
        let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["redirect_uri"], REDIRECT_URI);
        assert_eq!(query["scope"], "user-library-read");
        assert_eq!(query["code_challenge_method"], "S256");
        let pasted = format!("{REDIRECT_URI}?code=good-code&state={}", query["state"]);
        let (status, _) = self
            .request(
                "POST",
                "/api/spotify/callback",
                Some(json!({ "url": pasted })),
            )
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    async fn status(&self) -> Value {
        let (status, body) = self.request("GET", "/api/spotify", None).await;
        assert_eq!(status, StatusCode::OK);
        body
    }

    /// 取り込みが終わるまで待つ。
    async fn wait_idle(&self) -> Value {
        for _ in 0..200 {
            let status = self.status().await;
            if status["syncing"] == false && !status["lastSync"].is_null() {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("sync did not finish");
    }

    async fn links(&self) -> HashMap<String, (Option<String>, Option<String>)> {
        let rows: Vec<(String, Option<String>, Option<String>)> =
            sqlx::query_as("SELECT spotify_id, track_id, match_method FROM spotify_track")
                .fetch_all(&self.pool)
                .await
                .unwrap();
        rows.into_iter().map(|(s, t, m)| (s, (t, m))).collect()
    }

    async fn track_id(&self, rel: &str) -> String {
        sqlx::query_scalar("SELECT track_id FROM file WHERE path = ?")
            .bind(rel)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn starred_at(&self, track_id: &str) -> Option<i64> {
        sqlx::query_scalar("SELECT starred_at FROM track WHERE id = ?")
            .bind(track_id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

#[tokio::test]
async fn sync_stars_matched_tracks() {
    let f = Fixture::new().await;
    let scanner = Scanner::new(f.pool.clone(), f.dir.path().to_owned(), Default::default());
    scanner.start(Mode::Quick);
    scanner.wait().await;

    let status = f.status().await;
    assert_eq!(status["configured"], true);
    assert_eq!(status["connected"], false);
    assert_eq!(status["redirectUri"], REDIRECT_URI);

    f.connect().await;
    assert_eq!(f.status().await["connected"], true);

    let (status, _) = f.request("POST", "/api/spotify/sync", None).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let status = f.wait_idle().await;
    assert_eq!(status["lastSync"]["error"], Value::Null);
    assert_eq!(status["lastSync"]["fetched"], 53);
    assert_eq!(status["lastSync"]["matched"], 2);
    assert_eq!(status["total"], 53);
    assert_eq!(status["matched"], 2);
    assert_eq!(f.mock.lock().unwrap().track_requests, 2);

    let a = f.track_id("a/01.flac").await;
    let b = f.track_id("b/02.flac").await;
    let links = f.links().await;
    assert_eq!(links["s-same"], (Some(a.clone()), Some("match_key".into())));
    assert_eq!(links["s-single"], (Some(b.clone()), Some("fuzzy".into())));
    assert_eq!(links["s-later"], (None, None));
    assert_eq!(f.starred_at(&a).await, Some(ADDED_AT_MS));
    assert_eq!(f.starred_at(&b).await, Some(ADDED_AT_MS));

    let (status, body) = f
        .request("GET", "/api/spotify/tracks?matched=false", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["tracks"].as_array().unwrap().len(), 51);
    assert!(
        body["tracks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["trackId"].is_null())
    );
}

#[tokio::test]
async fn manual_link_and_unlink() {
    let f = Fixture::new().await;
    let scanner = Scanner::new(f.pool.clone(), f.dir.path().to_owned(), Default::default());
    scanner.start(Mode::Quick);
    scanner.wait().await;
    f.connect().await;
    f.request("POST", "/api/spotify/sync", None).await;
    f.wait_idle().await;

    // 手動で外した曲は、自動の対応を付け直さない
    let (status, _) = f
        .request(
            "PUT",
            "/api/spotify/tracks/s-single",
            Some(json!({ "trackId": null })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    // 手動で付けた曲はお気に入りにする
    let a = f.track_id("a/01.flac").await;
    sqlx::query("UPDATE track SET starred_at = NULL WHERE id = ?")
        .bind(&a)
        .execute(&f.pool)
        .await
        .unwrap();
    let (status, _) = f
        .request(
            "PUT",
            "/api/spotify/tracks/s-0",
            Some(json!({ "trackId": a })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(f.starred_at(&a).await, Some(ADDED_AT_MS));

    f.request("POST", "/api/spotify/sync", None).await;
    f.wait_idle().await;
    let links = f.links().await;
    assert_eq!(links["s-single"], (None, Some("ignored".into())));
    assert_eq!(links["s-0"], (Some(a), Some("manual".into())));

    let (status, _) = f
        .request(
            "PUT",
            "/api/spotify/tracks/unknown",
            Some(json!({ "trackId": null })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = f
        .request(
            "PUT",
            "/api/spotify/tracks/s-1",
            Some(json!({ "trackId": "tr-none" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn scans_fetch_hourly_and_rematch() {
    let f = Fixture::new().await;
    let scanner = Scanner::new(f.pool.clone(), f.dir.path().to_owned(), Default::default());
    tokio::spawn(f.spotify.clone().follow_scans(scanner.subscribe(), true));

    // 接続していなければ Spotify を呼ばない
    scanner.start(Mode::Quick);
    scanner.wait().await;
    f.wait_idle().await;
    assert_eq!(f.mock.lock().unwrap().track_requests, 0);

    // 接続して最初のスキャンでは、まだ読んでいないので読む
    f.connect().await;
    scanner.start(Mode::Quick);
    scanner.wait().await;
    wait_for(|| async { f.links().await.len() == 53 }).await;
    assert_eq!(f.mock.lock().unwrap().track_requests, 2);

    // 60 分たっていなければ Spotify を呼ばず、増えた曲に対応だけ付ける
    f.put("c/03.flac", Some("三曲目"));
    scanner.start(Mode::Quick);
    scanner.wait().await;
    let c = f.track_id("c/03.flac").await;
    wait_for(|| async { f.links().await["s-later"].0.as_deref() == Some(c.as_str()) }).await;
    assert_eq!(f.mock.lock().unwrap().track_requests, 2);
    assert_eq!(f.starred_at(&c).await, Some(ADDED_AT_MS));

    // 60 分たてば読み直す
    sqlx::query("UPDATE spotify_account SET fetched_at = ?")
        .bind(now_ms() - 61 * 60 * 1000)
        .execute(&f.pool)
        .await
        .unwrap();
    scanner.start(Mode::Quick);
    scanner.wait().await;
    wait_for(|| async { f.mock.lock().unwrap().track_requests == 4 }).await;
}

#[tokio::test]
async fn revoked_authorization_disconnects() {
    let f = Fixture::new().await;
    f.connect().await;
    // アクセストークンが切れ、リフレッシュトークンも取り消された
    f.mock.lock().unwrap().access_tokens.clear();
    f.mock.lock().unwrap().revoked.push("refresh-1".into());
    f.request("POST", "/api/spotify/sync", None).await;
    let status = f.wait_idle().await;
    assert!(
        status["lastSync"]["error"]
            .as_str()
            .unwrap()
            .contains("connect again")
    );
    assert_eq!(status["connected"], false);
}

#[tokio::test]
async fn expired_access_token_is_refreshed() {
    let f = Fixture::new().await;
    f.connect().await;
    f.mock.lock().unwrap().access_tokens.clear();
    f.request("POST", "/api/spotify/sync", None).await;
    let status = f.wait_idle().await;
    assert_eq!(status["lastSync"]["error"], Value::Null);
    // 使うたびに替わったリフレッシュトークンを保存する
    let (refresh,): (String,) = sqlx::query_as("SELECT refresh_token FROM spotify_account")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(refresh, "refresh-1-rotated");
}

#[tokio::test]
async fn pasted_url_is_checked() {
    let f = Fixture::new().await;
    let paste = |url: &str| json!({ "url": url });
    let (status, body) = f
        .request("POST", "/api/spotify/callback", Some(paste("not a url")))
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalidUrl"))
    );
    let (status, body) = f
        .request(
            "POST",
            "/api/spotify/callback",
            Some(paste(&format!(
                "{REDIRECT_URI}?code=good-code&state=forged"
            ))),
        )
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::BAD_REQUEST, Some("unknownState"))
    );
    let (status, body) = f
        .request(
            "POST",
            "/api/spotify/callback",
            Some(paste(&format!(
                "{REDIRECT_URI}?error=access_denied&state=x"
            ))),
        )
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::BAD_REQUEST, Some("denied"))
    );
    assert_eq!(f.status().await["connected"], false);
}

#[tokio::test]
async fn requires_session_and_json() {
    let f = Fixture::new().await;
    let res = f
        .router
        .clone()
        .oneshot(Request::get("/api/spotify").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = f
        .router
        .clone()
        .oneshot(
            Request::post("/api/spotify/sync")
                .header(header::COOKIE, format!("suisei_session={SESSION}"))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

async fn wait_for<F, Fut>(mut condition: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..200 {
        if condition().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition was not met");
}
