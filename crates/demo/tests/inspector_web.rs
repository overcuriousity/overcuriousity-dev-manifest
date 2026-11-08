#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use demo::domain::{events, query};
use support::*;

async fn some_event(a: &App) -> events::Event {
    let case = demo::domain::cases::list(&a.st.pool).await.unwrap()[0].clone();
    let q = query::parse("event_id:4688").unwrap();
    events::search(&a.st.pool, case.id, &q, None, 1)
        .await
        .unwrap()
        .items
        .remove(0)
}

#[tokio::test]
async fn the_inspector_shows_every_field_with_copy_on_hashes_and_ids() {
    let a = app().await;
    let e = some_event(&a).await;
    let h = body(a.htmx_get(&format!("/events/{}/inspector", e.id)).await).await;
    for label in [
        "Host",
        "User",
        "Process",
        "Command line",
        "SHA-256",
        "Logon ID",
        "Source file",
        "Record",
    ] {
        assert!(h.contains(&format!("<dt>{label}</dt>")), "{label} missing");
    }
    assert!(h.contains(&format!("data-oc-copy=\"{}\"", e.sha256.unwrap())));
    assert!(h.contains("No verdict"));
}

#[tokio::test]
async fn setting_a_verdict_returns_an_undo_that_restores_the_previous_one() {
    let a = app().await;
    let e = some_event(&a).await;
    let res = body(
        a.post(&format!("/events/{}/verdict", e.id), "verdict=malicious")
            .await,
    )
    .await;
    assert!(res.contains("data-state=\"malicious\"") && res.contains("Marked as malicious"));
    assert!(
        res.contains(&format!("hx-post=\"/events/{}/verdict\"", e.id))
            && res.contains("{&#34;verdict&#34;:&#34;&#34;}"),
        "{res}"
    );
    assert_eq!(
        events::get(&a.st.pool, e.id)
            .await
            .unwrap()
            .verdict
            .as_deref(),
        Some("malicious")
    );
    a.post(&format!("/events/{}/verdict", e.id), "verdict=")
        .await;
    assert_eq!(events::get(&a.st.pool, e.id).await.unwrap().verdict, None);
}

#[tokio::test]
async fn pinned_events_appear_in_the_story_in_time_order_and_the_story_starts_unloaded() {
    let a = app().await;
    let case = demo::domain::cases::list(&a.st.pool).await.unwrap()[0].clone();
    let page = events::search(&a.st.pool, case.id, &query::parse("").unwrap(), None, 2)
        .await
        .unwrap();
    let (newer, older) = (&page.items[0], &page.items[1]);
    for e in [newer, older] {
        a.post(&format!("/events/{}/pin", e.id), "").await;
    }
    let story = body(a.htmx_get(&format!("/cases/{}/story", case.id)).await).await;
    let (i_old, i_new) = (
        story.find(&format!("data-event=\"{}\"", older.id)).unwrap(),
        story.find(&format!("data-event=\"{}\"", newer.id)).unwrap(),
    );
    assert!(i_old < i_new);
    let timeline = a.text(&format!("/cases/{}/timeline", case.id)).await;
    assert!(timeline.contains("<div id=\"pane-body\" class=\"oc-stack\"></div>"));
}
