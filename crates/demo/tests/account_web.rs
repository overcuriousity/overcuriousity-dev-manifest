#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::http::StatusCode;
use support::*;

#[tokio::test]
async fn the_account_page_offers_appearance_passkeys_and_tokens() {
    let a = app().await;
    let page = a.text("/account").await;
    for needle in [
        "data-oc-pref-select=\"theme\"",
        "data-oc-pref-select=\"density\"",
        "id=\"oc-passkey-list\"",
        "id=\"oc-token-list\"",
        "hx-post=\"/auth/tokens\"",
    ] {
        assert!(page.contains(needle), "{needle}");
    }
}

#[tokio::test]
async fn a_minted_token_is_shown_once_with_a_copy_button_and_revoke_offers_undo() {
    let a = app().await;
    let minted = body(a.post("/auth/tokens", "name=ci").await).await;
    assert!(minted.contains("Shown once. Copy it now.") && minted.contains("data-oc-copy=\"demo_"));
    let id = minted
        .split("data-oc-copy=\"demo_")
        .nth(1)
        .unwrap()
        .split('.')
        .next()
        .unwrap()
        .to_owned();
    assert!(!a.text("/account").await.contains("data-oc-copy=\"demo_"));
    let revoked = body(a.post(&format!("/auth/tokens/{id}/revoke"), "").await).await;
    assert!(
        revoked.contains("Revoked token &#39;ci&#39;") || revoked.contains("Revoked token 'ci'"),
        "{revoked}"
    );
    assert!(revoked.contains(&format!("/auth/tokens/{id}/restore")));
}

#[tokio::test]
async fn purge_requires_the_exact_case_name() {
    let a = app().await;
    let case = demo::domain::cases::list(&a.st.pool).await.unwrap()[0].clone();
    let res = a
        .post(&format!("/cases/{}/purge", case.id), "name=nope")
        .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body(res).await, "Type the case name exactly.");
    let res = a
        .post(
            &format!("/cases/{}/purge", case.id),
            &format!("name={}", case.name.replace(' ', "+")),
        )
        .await;
    assert_eq!(res.headers()["hx-redirect"], "/cases");
    assert!(demo::domain::cases::get(&a.st.pool, case.id).await.is_err());
}

#[tokio::test]
async fn unknown_paths_render_a_one_line_404_with_a_way_back() {
    let a = app().await;
    let res = a.get("/nope").await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let page = body(res).await;
    assert!(
        page.contains("Page not found.")
            && page.contains("href=\"/notes\">Back to notes</a>")
            && page.contains("<html")
    );
}
