use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use suisei::{AppState, Credentials};
use tower::ServiceExt;

const AUTH: &str = "u=inaba&p=sesame";

async fn app() -> axum::Router {
    let db = suisei::db::open_in_memory().await.unwrap();
    suisei::router(AppState {
        credentials: Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: None,
        },
        db: db.clone(),
        scanner: suisei::scan::Scanner::new(db, "/nonexistent".into()),
        now_playing: Default::default(),
        cache_dir: "/nonexistent".into(),
    })
}

async fn send(req: Request<Body>) -> (StatusCode, String) {
    let res = app().await.oneshot(req).await.unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

async fn get(uri: &str) -> (StatusCode, String) {
    send(Request::get(uri).body(Body::empty()).unwrap()).await
}

async fn get_json(uri: &str) -> Value {
    let (status, body) = get(uri).await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_str::<Value>(&body).unwrap()["subsonic-response"].clone()
}

#[tokio::test]
async fn ping_json() {
    let res = get_json(&format!("/rest/ping?{AUTH}&f=json")).await;
    assert_eq!(res["status"], "ok");
    assert_eq!(res["openSubsonic"], true);
}

#[tokio::test]
async fn ping_view_xml() {
    let (status, body) = get(&format!("/rest/ping.view?{AUTH}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(r#"<subsonic-response openSubsonic="true""#));
    assert!(body.contains(r#"status="ok""#));
    assert!(body.contains(r#"xmlns="http://subsonic.org/restapi""#));
}

#[tokio::test]
async fn wrong_password_is_rejected() {
    let res = get_json("/rest/ping?u=inaba&p=wrong&f=json").await;
    assert_eq!(res["status"], "failed");
    assert_eq!(res["error"]["code"], 40);
}

#[tokio::test]
async fn unimplemented_endpoint_requires_auth() {
    let res = get_json("/rest/getArtists?f=json").await;
    assert_eq!(res["error"]["code"], 10);
}

#[tokio::test]
async fn pending_endpoint_returns_generic_error() {
    let res = get_json(&format!(
        "/rest/savePlayQueue.view?{AUTH}&f=json&id=tr-00000000"
    ))
    .await;
    assert_eq!(res["status"], "failed");
    assert_eq!(res["error"]["code"], 0);
    assert_eq!(res["error"]["message"], "not implemented: savePlayQueue");
}

#[tokio::test]
async fn unsupported_list_is_empty() {
    let res = get_json(&format!("/rest/getPodcasts?{AUTH}&f=json")).await;
    assert_eq!(res["status"], "ok");
    assert_eq!(res["podcasts"], serde_json::json!({}));
}

#[tokio::test]
async fn unsupported_action_is_501() {
    let (status, _) = get(&format!("/rest/createShare?{AUTH}&f=json&id=tr-00000000")).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn unknown_endpoint_is_404() {
    let (status, _) = get(&format!("/rest/getNothing?{AUTH}&f=json")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Symfonium は失敗すると削除を再送し続けるので、成功を返す。
#[tokio::test]
async fn delete_bookmark_succeeds() {
    let res = get_json(&format!(
        "/rest/deleteBookmark?{AUTH}&f=json&id=tr-00000000"
    ))
    .await;
    assert_eq!(res["status"], "ok");
    let res = get_json(&format!("/rest/deleteBookmark?{AUTH}&f=json")).await;
    assert_eq!(res["error"]["code"], 10);
}

#[tokio::test]
async fn external_info_is_empty() {
    let res = get_json(&format!(
        "/rest/getArtistInfo2?{AUTH}&f=json&id=ar-00000000"
    ))
    .await;
    assert_eq!(res["status"], "ok");
    assert_eq!(res["artistInfo2"], serde_json::json!({}));
    let res = get_json(&format!("/rest/getTopSongs?{AUTH}&f=json")).await;
    assert_eq!(res["error"]["code"], 10);
}

#[tokio::test]
async fn users_lists_the_only_user() {
    let res = get_json(&format!("/rest/getUsers?{AUTH}&f=json")).await;
    assert_eq!(res["users"]["user"][0]["username"], "inaba");
}

#[tokio::test]
async fn extensions_without_auth() {
    let res = get_json("/rest/getOpenSubsonicExtensions?f=json").await;
    assert_eq!(res["status"], "ok");
    assert_eq!(res["openSubsonicExtensions"][0]["name"], "formPost");
}

#[tokio::test]
async fn form_post() {
    let req = Request::post("/rest/ping")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!("{AUTH}&f=json")))
        .unwrap();
    let (status, body) = send(req).await;
    assert_eq!(status, StatusCode::OK);
    let res: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(res["subsonic-response"]["status"], "ok");
}

#[tokio::test]
async fn root_is_reachable() {
    let (status, _) = get("/").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn outside_rest_is_not_found() {
    let (status, _) = get("/server/xml.server.php").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn database_is_created_and_migrated() {
    let dir = std::env::temp_dir().join(format!("suisei-test-{}", std::process::id()));
    let pool = suisei::db::open(&dir).await.unwrap();
    // 二回目に開いても、適用済みのマイグレーションで失敗しない
    drop(pool);
    suisei::db::open(&dir).await.unwrap();
    assert!(dir.join("suisei.db").exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn foreign_keys_are_enforced() {
    let pool = suisei::db::open_in_memory().await.unwrap();
    let res = sqlx::query(
        "INSERT INTO track_artist (track_id, position, artist_id, credited_name) VALUES ('tr-00000000', 0, 'ar-00000000', 'x')",
    )
    .execute(&pool)
    .await;
    assert!(res.is_err(), "存在しない曲とアーティストを参照できてしまう");
}
