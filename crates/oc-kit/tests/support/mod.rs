#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;

use axum::Router;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, Response, header};
use oc_kit::auth::{AuthState, Branding};
use oc_kit::config::OcConfig;
use oc_kit::security::{Limits, secure};
use sqlx::SqlitePool;
use tower::ServiceExt;

pub const BRANDING: Branding = Branding {
    app_name: "Test",
    logo_svg: "<svg></svg>",
};

pub async fn state() -> (tempfile::TempDir, AuthState) {
    state_with(OcConfig::default()).await
}

pub async fn state_with(cfg: OcConfig) -> (tempfile::TempDir, AuthState) {
    let dir = tempfile::tempdir().unwrap();
    let pool = oc_kit::db::connect(dir.path(), "t.sqlite").await.unwrap();
    oc_kit::db::migrate_kit(&pool).await.unwrap();
    let dep = cfg.deployment().unwrap();
    let st = AuthState::new(pool, dep, cfg.auth.clone(), "test", BRANDING, "test")
        .await
        .unwrap();
    (dir, st)
}

pub async fn wrap(st: &AuthState, router: Router) -> Router {
    let sessions = oc_kit::session::session_layer(&st.pool, st.app, &st.dep)
        .await
        .unwrap();
    secure(
        router.layer(sessions),
        &st.dep,
        &OcConfig::default().trusted_proxies,
        Limits::default(),
    )
    .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 5000))))
}

pub fn req_get(uri: &str) -> axum::http::request::Builder {
    Request::get(uri).header(header::HOST, "localhost:8080")
}

pub fn post_form(uri: &str, body: &str) -> Request<Body> {
    Request::post(uri)
        .header(header::HOST, "localhost:8080")
        .header(header::ORIGIN, "http://localhost:8080")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

pub fn cookie(res: &Response<Body>) -> String {
    res.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

pub async fn send(app: &Router, req: Request<Body>) -> Response<Body> {
    app.clone().oneshot(req).await.unwrap()
}

pub async fn body(res: Response<Body>) -> String {
    String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

pub async fn add_user(pool: &SqlitePool, name: &str, pw: &str) -> oc_kit::identity::User {
    oc_kit::identity::users::create(
        pool,
        name,
        name,
        false,
        Some(pw),
        time::OffsetDateTime::now_utc(),
    )
    .await
    .unwrap()
}

/// Signs in and returns the session cookie pair `name=value`.
pub async fn sign_in(app: &Router, user: &str, pw: &str) -> String {
    let res = send(
        app,
        post_form(
            "/auth/sign-in",
            &format!("username={user}&password={pw}&next=/"),
        ),
    )
    .await;
    assert_eq!(res.status(), 303, "sign-in failed");
    cookie(&res)
}
