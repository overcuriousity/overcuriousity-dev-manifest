use serde::Serialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use super::OcError;
use oc_kit::trash::SoftDelete;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Note {
    pub id: i64,
    pub case_id: Option<i64>,
    pub case_name: Option<String>,
    pub author: String,
    pub title: String,
    pub body: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NoteInput {
    pub case_id: Option<i64>,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
}

impl NoteInput {
    fn validate(&self) -> Result<(), OcError> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err(OcError::Unprocessable("Give the note a title.".into()));
        }
        if title.chars().count() > 200 {
            return Err(OcError::Unprocessable(
                "Keep the title under 200 characters.".into(),
            ));
        }
        if self.body.chars().count() > 20_000 {
            return Err(OcError::Unprocessable(
                "Keep the note under 20000 characters.".into(),
            ));
        }
        Ok(())
    }
}

pub fn parse_tags(s: &str) -> Vec<String> {
    let mut tags: Vec<String> = s
        .split(',')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && t.chars().count() <= 40)
        .collect();
    tags.sort();
    tags.dedup();
    tags.truncate(12);
    tags
}

struct Row {
    id: i64,
    case_id: Option<i64>,
    case_name: Option<String>,
    author: String,
    title: String,
    body: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    deleted_at: Option<OffsetDateTime>,
}

async fn tags_of(pool: &SqlitePool, id: i64) -> Result<Vec<String>, OcError> {
    Ok(sqlx::query_scalar!(
        "SELECT tag FROM note_tags WHERE note_id = ? ORDER BY tag",
        id
    )
    .fetch_all(pool)
    .await?)
}

async fn hydrate(pool: &SqlitePool, rows: Vec<Row>) -> Result<Vec<Note>, OcError> {
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let tags = tags_of(pool, r.id).await?;
        out.push(Note {
            id: r.id,
            case_id: r.case_id,
            case_name: r.case_name,
            author: r.author,
            title: r.title,
            body: r.body,
            created_at: r.created_at,
            updated_at: r.updated_at,
            deleted_at: r.deleted_at,
            tags,
        });
    }
    Ok(out)
}

/// `filter` matches the title, case-insensitively.
pub async fn list(
    pool: &SqlitePool,
    case_id: Option<i64>,
    filter: &str,
    deleted: bool,
) -> Result<Vec<Note>, OcError> {
    let rows = sqlx::query_as!(
        Row,
        r#"SELECT n.id AS "id!", n.case_id, c.name AS "case_name?", u.display_name AS author, n.title, n.body,
                  n.created_at AS "created_at: OffsetDateTime", n.updated_at AS "updated_at: OffsetDateTime",
                  n.deleted_at AS "deleted_at: OffsetDateTime"
           FROM notes n JOIN oc_users u ON u.id = n.author_id LEFT JOIN cases c ON c.id = n.case_id
           WHERE (?1 IS NULL OR n.case_id = ?1)
             AND (?2 = '' OR instr(lower(n.title), lower(?2)) > 0)
             AND ((?3 AND n.deleted_at IS NOT NULL) OR (NOT ?3 AND n.deleted_at IS NULL))
           ORDER BY COALESCE(n.deleted_at, n.updated_at) DESC, n.id DESC"#,
        case_id,
        filter,
        deleted
    )
    .fetch_all(pool)
    .await?;
    hydrate(pool, rows).await
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Note, OcError> {
    let rows = sqlx::query_as!(
        Row,
        r#"SELECT n.id AS "id!", n.case_id, c.name AS "case_name?", u.display_name AS author, n.title, n.body,
                  n.created_at AS "created_at: OffsetDateTime", n.updated_at AS "updated_at: OffsetDateTime",
                  n.deleted_at AS "deleted_at: OffsetDateTime"
           FROM notes n JOIN oc_users u ON u.id = n.author_id LEFT JOIN cases c ON c.id = n.case_id
           WHERE n.id = ?"#,
        id
    )
    .fetch_all(pool)
    .await?;
    hydrate(pool, rows).await?.pop().ok_or(OcError::NotFound)
}

async fn set_tags(pool: &SqlitePool, id: i64, tags: &[String]) -> Result<(), OcError> {
    sqlx::query!("DELETE FROM note_tags WHERE note_id = ?", id)
        .execute(pool)
        .await?;
    for t in tags {
        sqlx::query!(
            "INSERT OR IGNORE INTO note_tags (note_id, tag) VALUES (?, ?)",
            id,
            t
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn create(
    pool: &SqlitePool,
    author_id: i64,
    input: &NoteInput,
    now: OffsetDateTime,
) -> Result<i64, OcError> {
    input.validate()?;
    let title = input.title.trim();
    let id = sqlx::query_scalar!(
        "INSERT INTO notes (case_id, author_id, title, body, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id AS \"id!\"",
        input.case_id,
        author_id,
        title,
        input.body,
        now,
        now
    )
    .fetch_one(pool)
    .await?;
    set_tags(pool, id, &input.tags).await?;
    Ok(id)
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &NoteInput,
    now: OffsetDateTime,
) -> Result<(), OcError> {
    input.validate()?;
    let title = input.title.trim();
    let r = sqlx::query!(
        "UPDATE notes SET case_id = ?, title = ?, body = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
        input.case_id,
        title,
        input.body,
        now,
        id
    )
    .execute(pool)
    .await?;
    if r.rows_affected() == 0 {
        return Err(OcError::NotFound);
    }
    set_tags(pool, id, &input.tags).await
}

#[derive(Clone)]
pub struct NotesStore(pub SqlitePool);

impl SoftDelete for NotesStore {
    async fn soft_delete(&self, id: i64, now: OffsetDateTime) -> Result<(), OcError> {
        let r = sqlx::query!(
            "UPDATE notes SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL",
            now,
            id
        )
        .execute(&self.0)
        .await?;
        if r.rows_affected() == 0 {
            Err(OcError::NotFound)
        } else {
            Ok(())
        }
    }

    async fn restore(&self, id: i64) -> Result<(), OcError> {
        let r = sqlx::query!(
            "UPDATE notes SET deleted_at = NULL WHERE id = ? AND deleted_at IS NOT NULL",
            id
        )
        .execute(&self.0)
        .await?;
        if r.rows_affected() == 0 {
            Err(OcError::NotFound)
        } else {
            Ok(())
        }
    }

    async fn purge_older_than(&self, cutoff: OffsetDateTime) -> Result<u64, OcError> {
        Ok(sqlx::query!(
            "DELETE FROM notes WHERE deleted_at IS NOT NULL AND deleted_at < ?",
            cutoff
        )
        .execute(&self.0)
        .await?
        .rows_affected())
    }
}
