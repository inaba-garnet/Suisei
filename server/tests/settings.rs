//! Web から変える設定（docs/server.md の「設定」）。

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use suisei::db;
use suisei::settings::Store;
use suisei::{AppState, Credentials};
use tower::ServiceExt;

const SESSION: &str = "session-token";

async fn app() -> (axum::Router, suisei::db::Pool) {
    let pool = db::open_in_memory().await.unwrap();
    db::session::create(&pool, &hex::encode(Sha256::digest(SESSION)), 0)
        .await
        .unwrap();
    // 最終利用日時が古すぎるとセッションが切れるので、今の時刻にする
    sqlx::query("UPDATE session SET last_used_at = strftime('%s','now') * 1000")
        .execute(&pool)
        .await
        .unwrap();
    let settings = Store::load(pool.clone()).await.unwrap();
    let router = suisei::router(AppState {
        credentials: Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: None,
        },
        db: pool.clone(),
        scanner: suisei::scan::Scanner::new(
            pool.clone(),
            "/nonexistent".into(),
            Default::default(),
        ),
        now_playing: Default::default(),
        cache_dir: "/nonexistent".into(),
        ffmpeg: "ffmpeg".into(),
        dev: false,
        throttle: Default::default(),
        trust_forwarded_for: false,
        spotify: suisei::spotify::Spotify::new(pool.clone(), settings.clone(), Default::default()),
        settings,
    });
    (router, pool)
}

async fn send(router: &axum::Router, method: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri("/api/settings")
        .header(header::COOKIE, format!("suisei_session={SESSION}"));
    if body.is_some() {
        req = req.header(header::CONTENT_TYPE, "application/json");
    }
    let req = req
        .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn defaults_and_saved_values() {
    let (router, pool) = app().await;
    let (status, body) = send(&router, "GET", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "scanInterval": 3600,
            "backupInterval": 86400,
            "backupKeep": 7,
            "splitCharacters": true,
            "spotifyClientId": null,
        })
    );

    let changed = json!({
        "scanInterval": 0,
        "backupInterval": 43200,
        "backupKeep": 3,
        "splitCharacters": false,
        "spotifyClientId": "0123456789abcdef0123456789abcdef",
    });
    let (status, body) = send(&router, "PUT", Some(changed.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, changed);

    // 再起動しても残る
    let stored = Store::load(pool).await.unwrap().get();
    assert_eq!(stored.scan_interval, Duration::ZERO);
    assert_eq!(stored.backup_interval, Duration::from_secs(43200));
    assert!(!stored.split_characters);
}

#[tokio::test]
async fn rejects_invalid_values() {
    let (router, _) = app().await;
    let (_, base) = send(&router, "GET", None).await;
    for (field, value) in [
        ("scanInterval", json!(10)),
        ("backupInterval", json!(60)),
        ("backupKeep", json!(1000)),
        ("spotifyClientId", json!("https://example.com")),
    ] {
        let mut body = base.clone();
        body[field] = value;
        let (status, error) = send(&router, "PUT", Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(error["field"], field);
    }
    let (_, after) = send(&router, "GET", None).await;
    assert_eq!(after, base);
}

#[tokio::test]
async fn requires_session() {
    let (router, _) = app().await;
    let res = router
        .oneshot(Request::get("/api/settings").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
