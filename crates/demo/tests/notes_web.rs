#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::http::{StatusCode, header};
use support::*;

#[tokio::test]
async fn creating_a_note_shows_it_in_the_list_and_on_its_page() {
    let a = app().await;
    let res = a
        .post(
            "/notes",
            "title=Firewall+export&case_id=&tags=network&body=First+line%0A%0ASecond",
        )
        .await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let loc = res.headers()[header::LOCATION].to_str().unwrap().to_owned();
    assert!(a.text("/notes").await.contains("Firewall export"));
    let page = a.text(&loc).await;
    assert!(page.contains("<p>First line</p>") && page.contains("<p>Second</p>"));
}

#[tokio::test]
async fn deleting_returns_an_undo_notice_and_restore_brings_the_note_back() {
    let a = app().await;
    let id = first_note_id(&a).await;
    let res = a.post(&format!("/notes/{id}/delete"), "").await;
    let page = body(res).await;
    assert!(
        page.contains("data-oc-autohide=\"6000\"")
            && page.contains(&format!("/notes/{id}/restore"))
    );
    assert!(
        a.text("/trash")
            .await
            .contains(&format!("/trash/{id}/restore"))
    );
    let res = a.post(&format!("/notes/{id}/restore"), "").await;
    assert_eq!(res.headers()["hx-redirect"], format!("/notes/{id}"));
    assert!(
        !a.text("/trash")
            .await
            .contains(&format!("/trash/{id}/restore"))
    );
}

#[tokio::test]
async fn the_empty_trash_says_why() {
    let a = app().await;
    assert!(
        a.text("/trash")
            .await
            .contains("Trash is empty. Deleted notes stay here for 30 days.")
    );
}

#[tokio::test]
async fn the_detail_page_has_copy_buttons_and_one_primary_button() {
    let a = app().await;
    let id = first_note_id(&a).await;
    for uri in [
        "/notes".to_owned(),
        format!("/notes/{id}"),
        "/notes/new".to_owned(),
        "/trash".to_owned(),
    ] {
        let page = a.text(&uri).await;
        assert!(
            page.matches("oc-btn--primary").count() <= 1,
            "{uri} has more than one primary button"
        );
    }
    let page = a.text(&format!("/notes/{id}")).await;
    assert!(
        page.contains(&format!("data-oc-copy=\"{id}\""))
            && page.contains(&format!(
                "data-oc-copy=\"http://localhost:8080/notes/{id}\""
            ))
    );
}

#[tokio::test]
async fn an_invalid_note_gets_a_one_line_notice() {
    let a = app().await;
    let res = a.post("/notes", "title=&case_id=&tags=&body=x").await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body(res).await.contains("Give the note a title."));
}
