use sqlx::SqlitePool;
use time::OffsetDateTime;

use super::password;
use crate::error::OcError;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub is_admin: bool,
}

pub async fn create(
    pool: &SqlitePool,
    username: &str,
    display_name: &str,
    is_admin: bool,
    password: Option<&str>,
    now: OffsetDateTime,
) -> Result<User, OcError> {
    let hash = password.map(password::hash).transpose()?;
    let id = sqlx::query_scalar!(
        "INSERT INTO oc_users (username, display_name, is_admin, password_hash, created_at) VALUES (?, ?, ?, ?, ?) RETURNING id",
        username,
        display_name,
        is_admin,
        hash,
        now
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(d) if d.is_unique_violation() => OcError::Conflict(format!("The username {username} is taken.")),
        e => e.into(),
    })?;
    Ok(User {
        id,
        username: username.to_owned(),
        display_name: display_name.to_owned(),
        is_admin,
    })
}

pub async fn by_id(pool: &SqlitePool, id: i64) -> Result<Option<User>, OcError> {
    Ok(sqlx::query_as!(
        User,
        r#"SELECT id AS "id!", username, display_name, is_admin AS "is_admin: bool" FROM oc_users WHERE id = ? AND disabled_at IS NULL"#,
        id
    )
    .fetch_optional(pool)
    .await?)
}

pub async fn by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>, OcError> {
    Ok(sqlx::query_as!(
        User,
        r#"SELECT id AS "id!", username, display_name, is_admin AS "is_admin: bool" FROM oc_users WHERE username = ? AND disabled_at IS NULL"#,
        username
    )
    .fetch_optional(pool)
    .await?)
}

pub async fn verify_password(
    pool: &SqlitePool,
    username: &str,
    secret: &str,
) -> Result<Option<User>, OcError> {
    let row = sqlx::query!(
        r#"SELECT id AS "id!", username, display_name, is_admin AS "is_admin: bool", password_hash FROM oc_users WHERE username = ? AND disabled_at IS NULL"#,
        username
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        password::verify(secret, password::dummy_hash());
        return Ok(None);
    };
    let ok = row
        .password_hash
        .as_deref()
        .is_some_and(|h| password::verify(secret, h));
    Ok(ok.then_some(User {
        id: row.id,
        username: row.username,
        display_name: row.display_name,
        is_admin: row.is_admin,
    }))
}

pub async fn set_password(pool: &SqlitePool, user_id: i64, secret: &str) -> Result<(), OcError> {
    let hash = password::hash(secret)?;
    sqlx::query!(
        "UPDATE oc_users SET password_hash = ? WHERE id = ?",
        hash,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn disable(pool: &SqlitePool, user_id: i64, now: OffsetDateTime) -> Result<(), OcError> {
    sqlx::query!(
        "UPDATE oc_users SET disabled_at = ? WHERE id = ?",
        now,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn count(pool: &SqlitePool) -> Result<i64, OcError> {
    Ok(sqlx::query_scalar!("SELECT COUNT(*) FROM oc_users")
        .fetch_one(pool)
        .await?)
}

/// Identities are keyed by `(issuer, subject)` only; e-mail addresses are never used to match accounts.
pub async fn find_or_provision_oidc(
    pool: &SqlitePool,
    issuer: &str,
    subject: &str,
    display_name: &str,
    preferred_username: Option<&str>,
    auto_provision: bool,
    now: OffsetDateTime,
) -> Result<Option<User>, OcError> {
    let existing = sqlx::query_scalar!(
        "SELECT user_id FROM oc_oidc_identities WHERE issuer = ? AND subject = ?",
        issuer,
        subject
    )
    .fetch_optional(pool)
    .await?;
    if let Some(id) = existing {
        return by_id(pool, id).await;
    }
    if !auto_provision {
        return Ok(None);
    }
    let base = preferred_username.unwrap_or(subject);
    let mut username = base.to_owned();
    for n in 2.. {
        if by_username(pool, &username).await?.is_none() {
            break;
        }
        username = format!("{base}-{n}");
    }
    let user = create(pool, &username, display_name, false, None, now).await?;
    sqlx::query!(
        "INSERT INTO oc_oidc_identities (issuer, subject, user_id) VALUES (?, ?, ?)",
        issuer,
        subject,
        user.id
    )
    .execute(pool)
    .await?;
    Ok(Some(user))
}
