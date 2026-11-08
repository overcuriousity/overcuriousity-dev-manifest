#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::body::Body;
use axum::http::header;
use demo::domain::{cases, events, query};
use support::*;

#[tokio::test]
async fn every_rendered_page_passes_the_html_rules() {
    let mut a = app().await;
    let case = cases::list(&a.st.pool).await.unwrap()[0].id;
    let empty = cases::create(&a.st.pool, "Empty case", "", demo::domain::now()).await.unwrap();
    let note = first_note_id(&a).await;
    let event = events::search(&a.st.pool, case, &query::parse("").unwrap(), None, 1).await.unwrap().items[0].id;
    a.post(&format!("/events/{event}/pin"), "").await;
    let pages = [
        "/notes".to_owned(),
        format!("/notes/{note}"),
        "/notes/new".into(),
        format!("/notes/{note}/edit"),
        "/trash".into(),
        "/search".into(),
        "/search?q=acme".into(),
        "/cases".into(),
        "/account".into(),
        format!("/cases/{case}/timeline"),
        format!("/cases/{case}/timeline?q=host%3Aws-01"),
        format!("/cases/{case}/timeline?q=%22open"),
        format!("/cases/{empty}/timeline"),
        "/nope".into(),
    ];
    let fragments = [
        format!("/cases/{case}/timeline/rows"),
        format!("/cases/{case}/timeline/facets"),
        format!("/cases/{case}/timeline/histogram"),
        format!("/cases/{case}/timeline/new-count?after=0"),
        format!("/events/{event}/inspector"),
        format!("/cases/{case}/story"),
        "/auth/passkeys".into(),
    ];
    let mut findings = Vec::new();
    for uri in &pages {
        let html = body(a.get(uri).await).await;
        assert!(
            html.contains(r#"<meta name="htmx-config" content='{"allowEval":false"#),
            "{uri} lacks the htmx config"
        );
        findings.extend(
            oc_check::check_html_for(&html, "http://localhost:8080")
                .into_iter()
                .map(|f| format!("{uri}:{}: {}", f.line, f.message)),
        );
    }
    for uri in &fragments {
        let html = body(a.htmx_get(uri).await).await;
        findings.extend(
            oc_check::check_html_for(&html, "http://localhost:8080")
                .into_iter()
                .map(|f| format!("{uri}:{}: {}", f.line, f.message)),
        );
    }
    a.cookie.clear();
    let sign_in = body(
        a.send(a.req("GET", "/auth/sign-in").header(header::ACCEPT, "text/html").body(Body::empty()).unwrap()).await,
    )
    .await;
    findings.extend(
        oc_check::check_html_for(&sign_in, "http://localhost:8080")
            .into_iter()
            .map(|f| format!("sign-in:{}: {}", f.line, f.message)),
    );
    assert!(findings.is_empty(), "{}", findings.join("\n"));
}

#[tokio::test]
async fn the_pwa_manifest_and_service_worker_are_served_at_the_root() {
    let a = app().await;
    let m = a.text("/manifest.webmanifest").await;
    assert!(m.contains("\"start_url\":\"/notes\"") && m.contains("\"display\":\"standalone\""));
    let res = a.get("/sw.js").await;
    assert_eq!(res.headers()[header::CONTENT_TYPE], "text/javascript; charset=utf-8");
    let sw = body(res).await;
    assert!(sw.contains("/offline") && sw.contains("\"/api/\""));
    assert!(a.text("/offline").await.contains("You are offline."));
}
