use serde::Serialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use super::OcError;

#[derive(Debug, Clone, Serialize)]
pub struct Case {
    pub id: i64,
    pub name: String,
    pub summary: String,
    pub events: i64,
    pub notes: i64,
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Case>, OcError> {
    Ok(sqlx::query_as!(
        Case,
        r#"SELECT c.id AS "id!", c.name, c.summary,
                  (SELECT COUNT(*) FROM events e WHERE e.case_id = c.id) AS "events!: i64",
                  (SELECT COUNT(*) FROM notes n WHERE n.case_id = c.id AND n.deleted_at IS NULL) AS "notes!: i64"
           FROM cases c ORDER BY c.created_at DESC, c.id DESC"#
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Case, OcError> {
    sqlx::query_as!(
        Case,
        r#"SELECT c.id AS "id!", c.name, c.summary,
                  (SELECT COUNT(*) FROM events e WHERE e.case_id = c.id) AS "events!: i64",
                  (SELECT COUNT(*) FROM notes n WHERE n.case_id = c.id AND n.deleted_at IS NULL) AS "notes!: i64"
           FROM cases c WHERE c.id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(OcError::NotFound)
}

pub async fn create(
    pool: &SqlitePool,
    name: &str,
    summary: &str,
    now: OffsetDateTime,
) -> Result<i64, OcError> {
    Ok(sqlx::query_scalar!(
        "INSERT INTO cases (name, summary, created_at) VALUES (?, ?, ?) RETURNING id",
        name,
        summary,
        now
    )
    .fetch_one(pool)
    .await?)
}

pub async fn set_summary(pool: &SqlitePool, id: i64, summary: &str) -> Result<(), OcError> {
    if summary.chars().count() > 4000 {
        return Err(OcError::Unprocessable(
            "Keep the summary under 4000 characters.".into(),
        ));
    }
    sqlx::query!("UPDATE cases SET summary = ? WHERE id = ?", summary, id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Irreversible: the case, its events and its notes are deleted, not moved to Trash.
pub async fn purge(pool: &SqlitePool, id: i64, typed_name: &str) -> Result<(), OcError> {
    let case = get(pool, id).await?;
    if typed_name != case.name {
        return Err(OcError::Unprocessable("Type the case name exactly.".into()));
    }
    sqlx::query!("DELETE FROM cases WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(())
}
