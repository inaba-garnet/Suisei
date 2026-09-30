//! Web クライアントのログインと、Cookie のセッションでの認証（docs/server.md）。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use suisei::db::{self, Pool};
use suisei::{AppState, Credentials};
use tower::ServiceExt;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const HOUR_MS: i64 = 60 * 60 * 1000;

struct App {
    router: Router,
    db: Pool,
}

impl App {
    async fn new() -> Self {
        let db = db::open_in_memory().await.unwrap();
        let router = suisei::router(AppState {
            credentials: Credentials {
                user: "inaba".into(),
                password: "sesame".into(),
                api_key: None,
            },
            db: db.clone(),
            scanner: suisei::scan::Scanner::new(
                db.clone(),
                "/nonexistent".into(),
                Default::default(),
            ),
            now_playing: Default::default(),
            cache_dir: "/nonexistent".into(),
            ffmpeg: "ffmpeg".into(),
            dev: false,
        });
        Self { router, db }
    }

    async fn send(&self, req: Request<Body>) -> Res {
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let set_cookie = res
            .headers()
            .get(header::SET_COOKIE)
            .map(|v| v.to_str().unwrap().to_owned());
        let body = res.into_body().collect().await.unwrap().to_bytes();
        Res {
            status,
            set_cookie,
            body: String::from_utf8(body.to_vec()).unwrap(),
        }
    }

    async fn login(&self, username: &str, password: &str) -> Res {
        let body = serde_json::json!({ "username": username, "password": password });
        self.send(
            Request::post("/api/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    /// ログインして、Cookie の値を返す。
    async fn session(&self) -> String {
        let res = self.login("inaba", "sesame").await;
        assert_eq!(res.status, StatusCode::NO_CONTENT);
        cookie_value(&res.set_cookie.unwrap())
    }

    async fn me(&self, token: &str) -> Res {
        self.send(
            Request::get("/api/me")
                .header(header::COOKIE, format!("suisei_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    /// Subsonic API を Cookie 付きで呼び、応答の中身を返す。
    async fn rest(&self, uri: &str, token: &str) -> Value {
        let res = self
            .send(
                Request::get(uri)
                    .header(header::COOKIE, format!("a=1; suisei_session={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(res.status, StatusCode::OK);
        serde_json::from_str::<Value>(&res.body).unwrap()["subsonic-response"].clone()
    }

    /// 最終利用日時を `ago_ms` だけ前にしたセッションを DB に直接作る。
    async fn insert_session(&self, token: &str, ago_ms: i64) {
        db::session::create(&self.db, &hash(token), now_ms() - ago_ms)
            .await
            .unwrap();
    }

    async fn last_used_at(&self, token: &str) -> Option<i64> {
        db::session::last_used_at(&self.db, &hash(token))
            .await
            .unwrap()
    }
}

struct Res {
    status: StatusCode,
    set_cookie: Option<String>,
    body: String,
}

fn cookie_value(set_cookie: &str) -> String {
    let pair = set_cookie.split(';').next().unwrap();
    pair.strip_prefix("suisei_session=").unwrap().to_owned()
}

fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[tokio::test]
async fn login_sets_session_cookie() {
    let app = App::new().await;
    let res = app.login("inaba", "sesame").await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let cookie = res.set_cookie.unwrap();
    assert!(cookie.contains("; Path=/"), "{cookie}");
    assert!(cookie.contains("; Max-Age=2592000"), "{cookie}");
    assert!(cookie.contains("; HttpOnly"), "{cookie}");
    assert!(cookie.contains("; SameSite=Strict"), "{cookie}");
    assert!(!cookie.contains("Secure"), "{cookie}");
    // DB には値そのものではなくハッシュを持つ
    let token = cookie_value(&cookie);
    assert_eq!(token.len(), 64);
    assert!(app.last_used_at(&token).await.is_some());
    let raw = db::session::last_used_at(&app.db, &token).await.unwrap();
    assert_eq!(raw, None);
}

#[tokio::test]
async fn login_behind_https_sets_secure() {
    let app = App::new().await;
    let res = app
        .send(
            Request::post("/api/login")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-forwarded-proto", "https")
                .body(Body::from(r#"{"username":"inaba","password":"sesame"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.set_cookie.unwrap().ends_with("; Secure"));
}

#[tokio::test]
async fn wrong_login_is_rejected() {
    let app = App::new().await;
    for (user, password) in [("inaba", "wrong"), ("someone", "sesame")] {
        let res = app.login(user, password).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
        assert_eq!(res.set_cookie, None);
    }
}

/// 別のサイトのフォームから送らせない。
#[tokio::test]
async fn login_and_logout_require_json() {
    let app = App::new().await;
    let res = app
        .send(
            Request::post("/api/login")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("username=inaba&password=sesame"))
                .unwrap(),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let token = app.session().await;
    let res = app
        .send(
            Request::post("/api/logout")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, format!("suisei_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(app.me(&token).await.status, StatusCode::OK);
}

#[tokio::test]
async fn me_returns_user_and_renews_cookie() {
    let app = App::new().await;
    let token = app.session().await;
    let res = app.me(&token).await;
    assert_eq!(res.status, StatusCode::OK);
    let body: Value = serde_json::from_str(&res.body).unwrap();
    assert_eq!(body["username"], "inaba");
    assert_eq!(body["dev"], false);
    let cookie = res.set_cookie.unwrap();
    assert_eq!(cookie_value(&cookie), token);
    assert!(cookie.contains("; Max-Age=2592000"), "{cookie}");
}

#[tokio::test]
async fn me_without_session_is_401() {
    let app = App::new().await;
    let res = app
        .send(Request::get("/api/me").body(Body::empty()).unwrap())
        .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    let res = app.me("unknown").await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    // 無効な Cookie は消させる
    assert!(res.set_cookie.unwrap().contains("; Max-Age=0"));
}

#[tokio::test]
async fn logout_ends_session() {
    let app = App::new().await;
    let token = app.session().await;
    let res = app
        .send(
            Request::post("/api/logout")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, format!("suisei_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.set_cookie.unwrap().contains("; Max-Age=0"));
    assert_eq!(app.me(&token).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(app.last_used_at(&token).await, None);
}

#[tokio::test]
async fn subsonic_accepts_session_cookie() {
    let app = App::new().await;
    let token = app.session().await;
    let res = app.rest("/rest/ping?f=json", &token).await;
    assert_eq!(res["status"], "ok");
}

#[tokio::test]
async fn subsonic_rejects_invalid_session_with_40() {
    let app = App::new().await;
    let res = app.rest("/rest/ping?f=json", "unknown").await;
    assert_eq!(res["error"]["code"], 40);
}

/// 認証の引数があれば、Cookie は見ない。
#[tokio::test]
async fn subsonic_params_take_precedence_over_cookie() {
    let app = App::new().await;
    let res = app
        .rest("/rest/ping?f=json&u=inaba&p=sesame", "unknown")
        .await;
    assert_eq!(res["status"], "ok");

    let token = app.session().await;
    let res = app.rest("/rest/ping?f=json&u=inaba&p=wrong", &token).await;
    assert_eq!(res["error"]["code"], 40);
    let res = app.rest("/rest/ping?f=json&u=inaba", &token).await;
    assert_eq!(res["error"]["code"], 10);
}

#[tokio::test]
async fn session_expires_after_30_days_unused() {
    let app = App::new().await;
    app.insert_session("old", 30 * DAY_MS).await;
    assert_eq!(app.me("old").await.status, StatusCode::UNAUTHORIZED);
    let res = app.rest("/rest/ping?f=json", "old").await;
    assert_eq!(res["error"]["code"], 40);

    app.insert_session("recent", 29 * DAY_MS).await;
    assert_eq!(app.me("recent").await.status, StatusCode::OK);
}

/// 最終利用日時の書き込みは 1 時間に一度まで。
#[tokio::test]
async fn last_used_is_updated_at_most_hourly() {
    let app = App::new().await;
    app.insert_session("fresh", HOUR_MS / 2).await;
    let before = app.last_used_at("fresh").await.unwrap();
    app.rest("/rest/ping?f=json", "fresh").await;
    assert_eq!(app.last_used_at("fresh").await, Some(before));

    app.insert_session("stale", 2 * HOUR_MS).await;
    let before = app.last_used_at("stale").await.unwrap();
    app.rest("/rest/ping?f=json", "stale").await;
    assert!(app.last_used_at("stale").await.unwrap() >= before + 2 * HOUR_MS - 1000);
}

#[tokio::test]
async fn login_deletes_expired_sessions() {
    let app = App::new().await;
    app.insert_session("old", 31 * DAY_MS).await;
    app.insert_session("recent", DAY_MS).await;
    app.session().await;
    assert_eq!(app.last_used_at("old").await, None);
    assert!(app.last_used_at("recent").await.is_some());
}
