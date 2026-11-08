#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::Router;
use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::routing::get;
use oc_kit::auth::{CurrentUser, resource_metadata_router, router};
use oc_kit::config::{OcConfig, OidcConfig, Secret};
use support::fake_idp::{self, Idp};
use support::*;

async fn app(
    idp: Option<&Idp>,
    auto: bool,
) -> (tempfile::TempDir, oc_kit::auth::AuthState, Router) {
    let mut cfg = OcConfig::default();
    cfg.auth.oidc = idp.map(|i| OidcConfig {
        issuer: i.issuer.parse().unwrap(),
        client_id: "app".into(),
        client_secret: Secret::new("s"),
        scopes: vec!["openid".into()],
        auto_provision: auto,
        display_name: "Sign in with SSO".into(),
    });
    let (d, st) = state_with(cfg).await;
    let r = Router::new()
        .route(
            "/me",
            get(|CurrentUser(u): CurrentUser| async move { u.display_name }),
        )
        .nest("/auth", router(st.clone()))
        .merge(resource_metadata_router())
        .with_state(st.clone());
    let app = wrap(&st, r).await;
    (d, st, app)
}

fn query(url: &str, key: &str) -> String {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
        .unwrap()
}

async fn start(app: &Router) -> (String, String, String) {
    let res = send(
        app,
        req_get("/auth/oidc/start?next=/me")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let loc = res.headers()[header::LOCATION].to_str().unwrap().to_owned();
    (loc, cookie(&res), String::new())
}

#[tokio::test]
async fn the_start_route_redirects_with_pkce_and_state() {
    let idp = fake_idp::start().await;
    let (_d, _st, app) = app(Some(&idp), false).await;
    let (loc, _, _) = start(&app).await;
    assert!(loc.starts_with(&format!("{}/authorize?", idp.issuer)));
    assert_eq!(query(&loc, "code_challenge_method"), "S256");
    assert_eq!(
        query(&loc, "redirect_uri"),
        "http://localhost:8080/auth/oidc/callback"
    );
}

async fn callback(app: &Router, sub: &str) -> axum::response::Response {
    let (loc, c, _) = start(app).await;
    let (state, nonce) = (query(&loc, "state"), query(&loc, "nonce"));
    let uri = format!("/auth/oidc/callback?state={state}&code={sub}:{nonce}");
    send(
        app,
        req_get(&uri)
            .header(header::COOKIE, c)
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

#[tokio::test]
async fn a_valid_callback_provisions_and_signs_in_when_allowed() {
    let idp = fake_idp::start().await;
    let (_d, _st, app) = app(Some(&idp), true).await;
    let res = callback(&app, "sub-1").await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER, "{:?}", body(res).await);
    assert_eq!(res.headers()[header::LOCATION], "/me");
    let me = send(
        &app,
        req_get("/me")
            .header(header::COOKIE, cookie(&res))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(body(me).await, "Test Person");
}

#[tokio::test]
async fn an_unknown_subject_without_auto_provision_sees_the_ask_an_admin_notice() {
    let idp = fake_idp::start().await;
    let (_d, _st, app) = app(Some(&idp), false).await;
    let res = callback(&app, "sub-2").await;
    assert!(
        body(res)
            .await
            .contains("No account for this sign-in. Ask an admin to add you.")
    );
}

#[tokio::test]
async fn a_jwt_bearer_from_the_issuer_authenticates_and_a_foreign_one_does_not() {
    let idp = fake_idp::start().await;
    let (_d, st, app) = app(Some(&idp), false).await;
    let u = add_user(&st.pool, "ada", "pw-ada-123").await;
    sqlx::query("INSERT INTO oc_oidc_identities (issuer, subject, user_id) VALUES (?, 'sub-9', ?)")
        .bind(&idp.issuer)
        .bind(u.id)
        .execute(&st.pool)
        .await
        .unwrap();
    let good = idp.id_token("sub-9", "n", "app");
    let r = send(
        &app,
        req_get("/me")
            .header(header::AUTHORIZATION, format!("Bearer {good}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(body(r).await, "ada");
    let foreign = Idp {
        issuer: "http://evil.example".into(),
    }
    .id_token("sub-9", "n", "app");
    let r = send(
        &app,
        req_get("/me")
            .header(header::AUTHORIZATION, format!("Bearer {foreign}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert!(
        r.headers()[header::WWW_AUTHENTICATE]
            .to_str()
            .unwrap()
            .contains("resource_metadata=")
    );
}

#[tokio::test]
async fn oidc_routes_and_resource_metadata_are_absent_without_config() {
    let (_d, _st, app) = app(None, false).await;
    for uri in ["/auth/oidc/start", "/.well-known/oauth-protected-resource"] {
        assert_eq!(
            send(&app, req_get(uri).body(Body::empty()).unwrap())
                .await
                .status(),
            StatusCode::NOT_FOUND,
            "{uri}"
        );
    }
}
