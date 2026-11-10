#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::Router;
use axum::http::{StatusCode, header};
use axum::routing::get;
use oc_kit::auth::{CurrentUser, router};
use support::*;

async fn app() -> (tempfile::TempDir, oc_kit::auth::AuthState, Router) {
    let (d, st) = state().await;
    add_user(&st.pool, "ada", "pw-ada-123").await;
    let r = Router::new()
        .route("/private", get(|CurrentUser(u): CurrentUser| async move { u.username }))
        .nest("/auth", router(st.clone()))
        .with_state(st.clone());
    let app = wrap(&st, r).await;
    (d, st, app)
}

#[tokio::test]
async fn signing_in_sets_a_host_prefixed_lax_httponly_secure_cookie() {
    let (_d, _st, app) = app().await;
    let res = send(&app, post_form("/auth/sign-in", "username=ada&password=pw-ada-123&next=/private")).await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    assert_eq!(res.headers()[header::LOCATION], "/private");
    let c = res.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(c.starts_with("__Host-test-session="), "{c}");
    for attr in ["HttpOnly", "Secure", "SameSite=Lax", "Path=/"] {
        assert!(c.contains(attr), "{attr} missing in {c}");
    }
    let r =
        send(&app, req_get("/private").header(header::COOKIE, cookie(&res)).body(axum::body::Body::empty()).unwrap())
            .await;
    assert_eq!(body(r).await, "ada");
}

#[tokio::test]
async fn protected_pages_redirect_html_and_reject_json() {
    let (_d, _st, app) = app().await;
    let html =
        send(&app, req_get("/private").header(header::ACCEPT, "text/html").body(axum::body::Body::empty()).unwrap())
            .await;
    assert_eq!(html.status(), StatusCode::SEE_OTHER);
    assert_eq!(html.headers()[header::LOCATION], "/auth/sign-in?next=%2Fprivate");
    let json = send(
        &app,
        req_get("/private").header(header::ACCEPT, "application/json").body(axum::body::Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(json.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body(json).await, r#"{"error":"Sign in first."}"#);
}

#[tokio::test]
async fn next_never_redirects_off_site_and_wrong_passwords_get_one_line() {
    let (_d, _st, app) = app().await;
    let res = send(&app, post_form("/auth/sign-in", "username=ada&password=pw-ada-123&next=//evil.example")).await;
    assert_eq!(res.headers()[header::LOCATION], "/");
    let bad = send(&app, post_form("/auth/sign-in", "username=ada&password=wrong&next=/")).await;
    assert_eq!(bad.status(), StatusCode::OK);
    assert!(body(bad).await.contains("Wrong username or password."));
}

#[tokio::test]
async fn signing_out_ends_the_session() {
    let (_d, _st, app) = app().await;
    let c = sign_in(&app, "ada", "pw-ada-123").await;
    let mut out = post_form("/auth/sign-out", "");
    out.headers_mut().insert(header::COOKIE, c.parse().unwrap());
    assert_eq!(send(&app, out).await.status(), StatusCode::SEE_OTHER);
    let r = send(&app, req_get("/private").header(header::COOKIE, c).body(axum::body::Body::empty()).unwrap()).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_eleventh_rapid_sign_in_attempt_is_rate_limited() {
    let (_d, _st, app) = app().await;
    let mut last = StatusCode::OK;
    for _ in 0..11 {
        last = send(&app, post_form("/auth/sign-in", "username=ada&password=wrong&next=/")).await.status();
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn next_rejects_control_characters_and_backslashes_that_browsers_turn_into_other_hosts() {
    use oc_kit::auth::safe_next;
    for bad in ["/\t/evil.example", "/\\evil.example", "/x\\y", "/\u{1}", "/ /evil", "/\u{7f}"] {
        assert_eq!(safe_next(bad), "/", "{bad:?}");
    }
    assert_eq!(safe_next("/cases/1/timeline?q=host%3Aws-01#x"), "/cases/1/timeline?q=host%3Aws-01#x");
}

#[tokio::test]
async fn a_tab_in_next_does_not_redirect_off_site() {
    let (_d, _st, app) = app().await;
    let res = send(&app, post_form("/auth/sign-in", "username=ada&password=pw-ada-123&next=/%09/evil.example")).await;
    assert_eq!(res.headers()[header::LOCATION], "/");
}

#[tokio::test]
async fn an_htmx_request_without_a_session_is_sent_to_sign_in() {
    let (_d, _st, app) = app().await;
    let r = req_get("/private")
        .header("hx-request", "true")
        .header("hx-current-url", "http://localhost:8080/cases/1/timeline?q=x")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = send(&app, r).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(res.headers()["hx-redirect"], "/auth/sign-in?next=%2Fcases%2F1%2Ftimeline%3Fq%3Dx");
}

#[tokio::test]
async fn plain_http_uses_an_unprefixed_cookie_without_secure_and_turns_passkeys_off() {
    let cfg = oc_kit::config::OcConfig {
        public_url: Some("http://notes.lan".parse().unwrap()),
        allow_plain_http: true,
        ..oc_kit::config::OcConfig::default()
    };
    let (_d, st) = state_with(cfg).await;
    assert!(st.webauthn.is_none() && !st.cfg.passkeys);
    add_user(&st.pool, "ada", "pw-ada-123").await;
    let app = wrap(&st, Router::new().nest("/auth", router(st.clone())).with_state(st.clone())).await;
    let req = axum::http::Request::post("/auth/sign-in")
        .header(header::HOST, "notes.lan")
        .header(header::ORIGIN, "http://notes.lan")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(axum::body::Body::from("username=ada&password=pw-ada-123&next=/"))
        .unwrap();
    let res = send(&app, req).await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let c = res.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(c.starts_with("test-session=") && c.contains("HttpOnly") && !c.contains("Secure"), "{c}");
}
