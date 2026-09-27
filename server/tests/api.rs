use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use suisei::{AppState, Credentials};
use tower::ServiceExt;

const AUTH: &str = "u=inaba&p=sesame";

fn app() -> axum::Router {
    suisei::router(AppState {
        credentials: Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: None,
        },
    })
}

async fn send(req: Request<Body>) -> (StatusCode, String) {
    let res = app().oneshot(req).await.unwrap();
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
async fn unimplemented_endpoint_returns_generic_error() {
    let res = get_json(&format!("/rest/getArtists.view?{AUTH}&f=json")).await;
    assert_eq!(res["status"], "failed");
    assert_eq!(res["error"]["code"], 0);
    assert_eq!(res["error"]["message"], "not implemented: getArtists");
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
async fn outside_rest_is_not_found() {
    let (status, _) = get("/").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
