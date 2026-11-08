#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::http::StatusCode;
use support::*;

async fn case_id(a: &App, name: &str) -> i64 {
    demo::domain::cases::list(&a.st.pool).await.unwrap().into_iter().find(|c| c.name == name).unwrap().id
}

#[tokio::test]
async fn the_timeline_renders_header_dividers_tiers_and_an_actions_header_for_screen_readers() {
    let a = app().await;
    let id = case_id(&a, "Lab ws-07").await;
    let page = a.text(&format!("/cases/{id}/timeline")).await;
    assert!(page.contains("<th class=\"oc-col-actions\"><span class=\"oc-sr-only\">Actions</span></th>"));
    for tier in ["tier-2", "tier-3", "tier-4"] {
        assert!(page.contains(&format!("<th class=\"{tier}")), "{tier} header missing");
    }
    assert!(page.contains("oc-app--workbench") && page.contains("data-oc-rows"));
}

#[tokio::test]
async fn filter_terms_render_as_pills_and_removing_one_offers_undo() {
    let a = app().await;
    let id = case_id(&a, "Lab ws-07").await;
    let page = a.text(&format!("/cases/{id}/timeline?q=host%3Aws-01")).await;
    assert!(page.contains("class=\"oc-pill\">host:ws-01"));
    let removed = a.text(&format!("/cases/{id}/timeline?q=&undo=host%3Aws-01")).await;
    assert!(removed.contains("Removed filter host:ws-01") && removed.contains("q=host%3Aws-01"));
}

#[tokio::test]
async fn infinite_scroll_fragments_do_not_repeat_rows() {
    let a = app().await;
    let id = case_id(&a, "Lab ws-07").await;
    let first = body(a.htmx_get(&format!("/cases/{id}/timeline/rows")).await).await;
    let cursor = first.split("cursor=").nth(1).unwrap().split('"').next().unwrap().replace("&amp;", "&");
    let second = body(a.htmx_get(&format!("/cases/{id}/timeline/rows?cursor={cursor}")).await).await;
    let ids = |s: &str| {
        s.match_indices("data-event=\"")
            .map(|(i, _)| s[i + 12..].split('"').next().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    let (a1, b1) = (ids(&first), ids(&second));
    assert_eq!(a1.len(), 100);
    assert!(b1.iter().all(|x| !a1.contains(x)));
}

#[tokio::test]
async fn a_hostile_query_returns_a_400_with_one_line() {
    let a = app().await;
    let id = case_id(&a, "Lab ws-07").await;
    let res = a.htmx_get(&format!("/cases/{id}/timeline/rows?q=user%3A%22x")).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(res).await, "A quote is not closed.");
    let page = a.text(&format!("/cases/{id}/timeline?q=%27%3B+DROP+TABLE+events%3B+--")).await;
    assert!(page.contains("No events match this search. Remove a filter to see more."));
}

#[tokio::test]
async fn an_empty_case_shows_an_empty_state_with_the_reason() {
    let a = app().await;
    let id = demo::domain::cases::create(&a.st.pool, "Empty", "", demo::domain::now()).await.unwrap();
    assert!(
        a.text(&format!("/cases/{id}/timeline"))
            .await
            .contains("No events in this case yet. New events appear here as they are ingested.")
    );
}
