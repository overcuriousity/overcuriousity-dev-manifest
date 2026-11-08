#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use oc_kit::identity::tokens;
use support::*;
use time::OffsetDateTime;

#[tokio::test]
async fn a_minted_token_authenticates_its_user_and_is_stored_hashed() {
    let (_d, st) = state().await;
    let u = add_user(&st.pool, "ada", "pw-ada-123").await;
    let now = OffsetDateTime::now_utc();
    let (info, token) = tokens::mint(&st.pool, u.id, "laptop", "test", now)
        .await
        .unwrap();
    assert!(token.starts_with(&format!("test_{}.", info.id)));
    assert_eq!(
        tokens::verify(&st.pool, &token, "test", now)
            .await
            .unwrap()
            .unwrap()
            .id,
        u.id
    );
    let stored: String = sqlx::query_scalar("SELECT secret_hash FROM oc_api_tokens")
        .fetch_one(&st.pool)
        .await
        .unwrap();
    assert!(!token.contains(&stored) && stored.starts_with("$argon2id$"));
}

#[tokio::test]
async fn a_revoked_token_is_rejected_and_restore_reenables_it() {
    let (_d, st) = state().await;
    let u = add_user(&st.pool, "ada", "pw-ada-123").await;
    let now = OffsetDateTime::now_utc();
    let (info, token) = tokens::mint(&st.pool, u.id, "ci", "test", now)
        .await
        .unwrap();
    tokens::revoke(&st.pool, u.id, &info.id, now).await.unwrap();
    assert!(
        tokens::verify(&st.pool, &token, "test", now)
            .await
            .unwrap()
            .is_none()
    );
    tokens::restore(&st.pool, u.id, &info.id).await.unwrap();
    assert!(
        tokens::verify(&st.pool, &token, "test", now)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn malformed_or_foreign_tokens_are_rejected_without_panicking() {
    let (_d, st) = state().await;
    let now = OffsetDateTime::now_utc();
    let long = "test_".to_owned() + &"a".repeat(10_000);
    for t in [
        "",
        "test_",
        "test_x.",
        "other_abcdefghij.secret",
        "test_abcdefghij.",
        long.as_str(),
    ] {
        assert!(
            tokens::verify(&st.pool, t, "test", now)
                .await
                .unwrap()
                .is_none(),
            "{t:.20}"
        );
    }
}

#[tokio::test]
async fn a_user_cannot_revoke_another_users_token() {
    let (_d, st) = state().await;
    let a = add_user(&st.pool, "ada", "pw-ada-123").await;
    let b = add_user(&st.pool, "bob", "pw-bob-123").await;
    let now = OffsetDateTime::now_utc();
    let (info, token) = tokens::mint(&st.pool, a.id, "x", "test", now)
        .await
        .unwrap();
    assert!(tokens::revoke(&st.pool, b.id, &info.id, now).await.is_err());
    assert!(
        tokens::verify(&st.pool, &token, "test", now)
            .await
            .unwrap()
            .is_some()
    );
}
