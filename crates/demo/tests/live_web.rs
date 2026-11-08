#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::time::Duration;

use axum::body::Body;
use demo::domain::{cases, events, seed};
use demo::state::IngestEvent;
use http_body_util::BodyExt;
use support::*;

async fn ids(a: &App) -> (i64, i64) {
    let all = cases::list(&a.st.pool).await.unwrap();
    (all[0].id, all[1].id)
}

async fn add(a: &App, case: i64, host: &str) -> i64 {
    let mut e = seed::event(&mut rand::rng(), seed::now_ms(), 1);
    e.host = host.to_owned();
    let id = events::insert(&a.st.pool, case, &e).await.unwrap();
    a.st.ingest.send(IngestEvent { case_id: case, event_id: id });
    id
}

#[tokio::test]
async fn the_live_stream_emits_ingest_for_its_case_only() {
    let a = app().await;
    let (mine, other) = ids(&a).await;
    let res = a.send(a.req("GET", &format!("/cases/{mine}/live")).body(Body::empty()).unwrap()).await;
    let mut body = res.into_body();
    tokio::time::sleep(Duration::from_millis(50)).await;
    add(&a, other, "ws-99").await;
    add(&a, mine, "ws-98").await;
    let mut text = String::new();
    let _ = tokio::time::timeout(Duration::from_millis(300), async {
        while let Some(Ok(f)) = body.frame().await {
            if let Some(d) = f.data_ref() {
                text.push_str(&String::from_utf8_lossy(d));
            }
        }
    })
    .await;
    assert_eq!(text.matches("event: ingest").count(), 1, "{text}");
}

#[tokio::test]
async fn new_count_and_show_respect_the_cursor_and_the_query() {
    let a = app().await;
    let (mine, _) = ids(&a).await;
    let after = events::max_id(&a.st.pool, mine).await.unwrap();
    add(&a, mine, "ws-98").await;
    let last = add(&a, mine, "ws-97").await;
    let n = body(a.htmx_get(&format!("/cases/{mine}/timeline/new-count?after={after}&q=host%3Aws-98")).await).await;
    assert!(n.contains("1 new event"), "{n}");
    let none = body(a.htmx_get(&format!("/cases/{mine}/timeline/new-count?after={last}&q=")).await).await;
    assert!(none.trim().is_empty());
    let shown = body(a.htmx_get(&format!("/cases/{mine}/timeline/new?after={after}&q=")).await).await;
    assert_eq!(shown.matches("data-oc-row").count(), 2);
    assert!(shown.contains(&format!("id=\"after\" name=\"after\" value=\"{last}\"")));
}

#[tokio::test]
async fn the_histogram_is_an_accessible_svg_without_literal_colours() {
    let a = app().await;
    let (mine, _) = ids(&a).await;
    let svg = body(a.htmx_get(&format!("/cases/{mine}/timeline/histogram?q=")).await).await;
    assert!(
        svg.contains("<svg")
            && svg.contains("role=\"img\"")
            && svg.contains("aria-label=\"")
            && svg.contains("oc-hist-bar")
    );
    assert!(oc_check::check_html(&svg).is_empty(), "{:?}", oc_check::check_html(&svg));
}

#[tokio::test]
async fn a_resync_reload_moves_the_new_events_cursor() {
    let a = app().await;
    let (mine, _) = ids(&a).await;
    let last = add(&a, mine, "ws-96").await;
    let rows = body(a.htmx_get(&format!("/cases/{mine}/timeline/rows?q=")).await).await;
    assert!(
        rows.contains(&format!("id=\"after\" name=\"after\" value=\"{last}\" hx-swap-oob=\"true\"")),
        "{}",
        &rows[rows.len().saturating_sub(400)..]
    );
}
