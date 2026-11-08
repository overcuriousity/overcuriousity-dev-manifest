#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use oc_kit::auth::router;
use oc_kit::config::OcConfig;
use support::*;

async fn app(passkeys: bool) -> (tempfile::TempDir, Router) {
    let mut cfg = OcConfig::default();
    cfg.auth.passkeys = passkeys;
    let (d, st) = state_with(cfg).await;
    add_user(&st.pool, "ada", "pw-ada-123").await;
    let app = wrap(
        &st,
        Router::new()
            .nest("/auth", router(st.clone()))
            .with_state(st.clone()),
    )
    .await;
    (d, app)
}

fn json_post(uri: &str, body: &str, cookie: Option<&str>) -> Request<Body> {
    let mut r = Request::post(uri)
        .header(header::HOST, "localhost:8080")
        .header(header::ORIGIN, "http://localhost:8080")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        r = r.header(header::COOKIE, c);
    }
    r.body(Body::from(body.to_owned())).unwrap()
}

#[tokio::test]
async fn passkey_routes_are_absent_when_disabled() {
    let (_d, app) = app(false).await;
    assert_eq!(
        send(&app, json_post("/auth/passkeys/login/start", "", None))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn register_start_requires_sign_in() {
    let (_d, app) = app(true).await;
    assert_eq!(
        send(&app, json_post("/auth/passkeys/register/start", "", None))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let c = sign_in(&app, "ada", "pw-ada-123").await;
    let res = send(
        &app,
        json_post("/auth/passkeys/register/start", "", Some(&c)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body(res).await.contains("\"challenge\""));
}

#[tokio::test]
async fn login_start_returns_a_challenge_and_finish_without_a_ceremony_is_rejected() {
    let (_d, app) = app(true).await;
    let res = send(&app, json_post("/auth/passkeys/login/start", "", None)).await;
    assert_eq!(res.status(), StatusCode::OK);
    let has_cookie = res.headers().contains_key(header::SET_COOKIE);
    assert!(body(res).await.contains("\"challenge\"") && has_cookie);
    let fin = send(&app, json_post("/auth/passkeys/login/finish", "{}", None)).await;
    assert!(fin.status().is_client_error(), "{}", fin.status());
}
