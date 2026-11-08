#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::Router;
use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::routing::get;
use oc_kit::auth::router;
use oc_kit::prefs::{PagePrefs, prefs_router};
use support::*;

async fn app() -> (tempfile::TempDir, Router) {
    let (d, st) = state().await;
    add_user(&st.pool, "ada", "pw-ada-123").await;
    let r = Router::new()
        .route(
            "/p",
            get(|PagePrefs(p, src): PagePrefs| async move {
                format!("{} {} {}", p.theme.as_str(), p.density.as_str(), src.as_str())
            }),
        )
        .nest("/auth", router(st.clone()))
        .nest("/_oc/prefs", prefs_router())
        .with_state(st.clone());
    (d, wrap(&st, r).await)
}

fn post(body: &str, cookie: Option<&str>) -> axum::http::Request<Body> {
    let mut r = post_form("/_oc/prefs", body);
    if let Some(c) = cookie {
        r.headers_mut().insert(header::COOKIE, c.parse().unwrap());
    }
    r
}

#[tokio::test]
async fn a_signed_in_users_theme_follows_them_to_a_new_session() {
    let (_d, app) = app().await;
    let c = sign_in(&app, "ada", "pw-ada-123").await;
    assert_eq!(send(&app, post("theme=dark&density=compact", Some(&c))).await.status(), StatusCode::NO_CONTENT);
    let c2 = sign_in(&app, "ada", "pw-ada-123").await;
    let r = send(&app, req_get("/p").header(header::COOKIE, c2).body(Body::empty()).unwrap()).await;
    assert_eq!(body(r).await, "dark compact user");
}

#[tokio::test]
async fn signed_out_pages_use_local_preferences() {
    let (_d, app) = app().await;
    let r = send(&app, req_get("/p").body(Body::empty()).unwrap()).await;
    assert_eq!(body(r).await, "system default local");
}

#[tokio::test]
async fn unknown_values_are_rejected_and_sign_in_is_required() {
    let (_d, app) = app().await;
    assert_eq!(send(&app, post("theme=dark", None)).await.status(), StatusCode::UNAUTHORIZED);
    let c = sign_in(&app, "ada", "pw-ada-123").await;
    let r = send(&app, post("theme=neon", Some(&c))).await;
    assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body(r).await, "Unknown theme.");
}
