#![allow(clippy::unwrap_used, clippy::expect_used)]

use oc_kit::db;
use oc_kit::identity::users;
use time::OffsetDateTime;

async fn pool() -> (tempfile::TempDir, sqlx::SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = db::connect(dir.path(), "t.sqlite").await.unwrap();
    db::migrate_kit(&pool).await.unwrap();
    (dir, pool)
}

#[tokio::test]
async fn a_created_user_can_sign_in_with_their_password() {
    let (_d, pool) = pool().await;
    let now = OffsetDateTime::now_utc();
    let u = users::create(&pool, "Ada", "Ada L.", true, Some("correct horse"), now)
        .await
        .unwrap();
    let signed_in = users::verify_password(&pool, "ada", "correct horse")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(signed_in.id, u.id);
    assert!(signed_in.is_admin);
}

#[tokio::test]
async fn a_wrong_password_an_unknown_user_and_a_disabled_user_all_return_none() {
    let (_d, pool) = pool().await;
    let now = OffsetDateTime::now_utc();
    let u = users::create(&pool, "bob", "Bob", false, Some("pw-123456"), now)
        .await
        .unwrap();
    assert!(
        users::verify_password(&pool, "bob", "nope")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        users::verify_password(&pool, "nobody", "pw-123456")
            .await
            .unwrap()
            .is_none()
    );
    users::disable(&pool, u.id, now).await.unwrap();
    assert!(
        users::verify_password(&pool, "bob", "pw-123456")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn oidc_identities_match_on_issuer_and_subject_and_provision_only_when_allowed() {
    let (_d, pool) = pool().await;
    let now = OffsetDateTime::now_utc();
    let none =
        users::find_or_provision_oidc(&pool, "https://idp", "sub-1", "Cy", Some("cy"), false, now)
            .await
            .unwrap();
    assert!(none.is_none());
    let made =
        users::find_or_provision_oidc(&pool, "https://idp", "sub-1", "Cy", Some("cy"), true, now)
            .await
            .unwrap()
            .unwrap();
    let again = users::find_or_provision_oidc(
        &pool,
        "https://idp",
        "sub-1",
        "Other",
        Some("x"),
        false,
        now,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(made.id, again.id);
    let other_issuer =
        users::find_or_provision_oidc(&pool, "https://other", "sub-1", "Cy", Some("cy"), true, now)
            .await
            .unwrap()
            .unwrap();
    assert_ne!(other_issuer.id, made.id);
}

#[tokio::test]
async fn kit_and_app_migrations_coexist_in_one_database() {
    let (_d, pool) = pool().await;
    let mut app =
        sqlx::migrate::Migrator::new(std::path::Path::new("tests/fixtures/app_migrations"))
            .await
            .unwrap();
    app.set_ignore_missing(true);
    app.run(&pool).await.unwrap();
    db::migrate_kit(&pool).await.unwrap();
    app.run(&pool).await.unwrap();
}
