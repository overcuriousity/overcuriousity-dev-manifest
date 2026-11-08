//! App-issued API tokens: `{prefix}_{id}.{secret}`. The id is a public lookup key; only an argon2 hash of the secret is stored.

use rand::Rng;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use super::{User, password};
use crate::error::OcError;

const ID_LEN: usize = 10;
const SECRET_LEN: usize = 52;
const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";

#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenInfo {
    pub id: String,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
}

fn random(len: usize) -> String {
    let mut rng = rand::rng();
    (0..len)
        .map(|_| char::from(ALPHABET[rng.random_range(0..ALPHABET.len())]))
        .collect()
}

fn parse<'a>(token: &'a str, prefix: &str) -> Option<(&'a str, &'a str)> {
    let rest = token.strip_prefix(prefix)?.strip_prefix('_')?;
    let (id, secret) = rest.split_once('.')?;
    let valid = |s: &str, len: usize| s.len() == len && s.bytes().all(|b| ALPHABET.contains(&b));
    (valid(id, ID_LEN) && valid(secret, SECRET_LEN)).then_some((id, secret))
}

/// Returns the token text once; it cannot be recovered later.
pub async fn mint(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    prefix: &str,
    now: OffsetDateTime,
) -> Result<(TokenInfo, String), OcError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(OcError::Unprocessable(
            "Give the token a name of up to 80 characters.".into(),
        ));
    }
    let id = random(ID_LEN);
    let secret = random(SECRET_LEN);
    let hash = password::hash(&secret)?;
    sqlx::query!(
        "INSERT INTO oc_api_tokens (id, user_id, name, secret_hash, created_at) VALUES (?, ?, ?, ?, ?)",
        id,
        user_id,
        name,
        hash,
        now
    )
    .execute(pool)
    .await?;
    let token = format!("{prefix}_{id}.{secret}");
    Ok((
        TokenInfo {
            id,
            name: name.to_owned(),
            created_at: now,
            last_used_at: None,
            revoked_at: None,
        },
        token,
    ))
}

pub async fn verify(
    pool: &SqlitePool,
    token: &str,
    prefix: &str,
    now: OffsetDateTime,
) -> Result<Option<User>, OcError> {
    let Some((id, secret)) = parse(token, prefix) else {
        return Ok(None);
    };
    let row = sqlx::query!(
        r#"SELECT t.secret_hash, u.id AS "user_id!", u.username, u.display_name, u.is_admin AS "is_admin: bool"
           FROM oc_api_tokens t JOIN oc_users u ON u.id = t.user_id
           WHERE t.id = ? AND t.revoked_at IS NULL AND u.disabled_at IS NULL"#,
        id
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    if !password::verify(secret, &row.secret_hash) {
        return Ok(None);
    }
    let stale = now - time::Duration::minutes(1);
    sqlx::query!(
        "UPDATE oc_api_tokens SET last_used_at = ? WHERE id = ? AND (last_used_at IS NULL OR last_used_at < ?)",
        now,
        id,
        stale
    )
    .execute(pool)
    .await?;
    Ok(Some(User {
        id: row.user_id,
        username: row.username,
        display_name: row.display_name,
        is_admin: row.is_admin,
    }))
}

pub async fn list(pool: &SqlitePool, user_id: i64) -> Result<Vec<TokenInfo>, OcError> {
    Ok(sqlx::query_as!(
        TokenInfo,
        r#"SELECT id AS "id!", name, created_at AS "created_at: OffsetDateTime", last_used_at AS "last_used_at: OffsetDateTime",
                  revoked_at AS "revoked_at: OffsetDateTime"
           FROM oc_api_tokens WHERE user_id = ? AND revoked_at IS NULL ORDER BY created_at DESC"#,
        user_id
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &SqlitePool, user_id: i64, id: &str) -> Result<TokenInfo, OcError> {
    sqlx::query_as!(
        TokenInfo,
        r#"SELECT id AS "id!", name, created_at AS "created_at: OffsetDateTime", last_used_at AS "last_used_at: OffsetDateTime",
                  revoked_at AS "revoked_at: OffsetDateTime"
           FROM oc_api_tokens WHERE user_id = ? AND id = ?"#,
        user_id,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(OcError::NotFound)
}

pub async fn revoke(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
    now: OffsetDateTime,
) -> Result<(), OcError> {
    let r = sqlx::query!(
        "UPDATE oc_api_tokens SET revoked_at = ? WHERE id = ? AND user_id = ?",
        now,
        id,
        user_id
    )
    .execute(pool)
    .await?;
    if r.rows_affected() == 0 {
        Err(OcError::NotFound)
    } else {
        Ok(())
    }
}

pub async fn restore(pool: &SqlitePool, user_id: i64, id: &str) -> Result<(), OcError> {
    let r = sqlx::query!(
        "UPDATE oc_api_tokens SET revoked_at = NULL WHERE id = ? AND user_id = ?",
        id,
        user_id
    )
    .execute(pool)
    .await?;
    if r.rows_affected() == 0 {
        Err(OcError::NotFound)
    } else {
        Ok(())
    }
}
