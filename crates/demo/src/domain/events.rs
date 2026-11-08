use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Serialize;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use time::OffsetDateTime;

use super::OcError;
use super::query::{Query, Term};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Event {
    pub id: i64,
    pub case_id: i64,
    pub ts: i64,
    pub host: String,
    pub user: String,
    pub event_id: i64,
    pub level: String,
    pub source: String,
    pub process: String,
    pub pid: Option<i64>,
    pub parent: Option<String>,
    pub command_line: Option<String>,
    pub record: i64,
    pub integrity: Option<String>,
    pub logon_id: Option<String>,
    pub source_file: String,
    pub sha256: Option<String>,
    pub message: String,
    pub verdict: Option<String>,
    pub pinned_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Default)]
pub struct NewEvent {
    pub ts: i64,
    pub host: String,
    pub user: String,
    pub event_id: i64,
    pub level: String,
    pub source: String,
    pub process: String,
    pub pid: Option<i64>,
    pub parent: Option<String>,
    pub command_line: Option<String>,
    pub record: i64,
    pub integrity: Option<String>,
    pub logon_id: Option<String>,
    pub source_file: String,
    pub sha256: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub ts: i64,
    pub id: i64,
}

impl Cursor {
    pub fn encode(self) -> String {
        URL_SAFE_NO_PAD.encode(format!("{}|{}", self.ts, self.id))
    }

    pub fn decode(s: &str) -> Result<Cursor, OcError> {
        let bad = || {
            OcError::BadRequest(
                "That page link is no longer valid. Start from the first page.".into(),
            )
        };
        let raw = URL_SAFE_NO_PAD.decode(s).map_err(|_| bad())?;
        let text = String::from_utf8(raw).map_err(|_| bad())?;
        let (ts, id) = text.split_once('|').ok_or_else(bad)?;
        Ok(Cursor {
            ts: ts.parse().map_err(|_| bad())?,
            id: id.parse().map_err(|_| bad())?,
        })
    }
}

pub const VERDICTS: [&str; 3] = ["benign", "suspicious", "malicious"];
pub const FACET_FIELDS: [&str; 4] = ["host", "user", "event_id", "level"];

/// Field names never reach SQL from user input; they map through this fixed table.
fn column(field: &str) -> Option<&'static str> {
    Some(match field {
        "host" => "host",
        "user" => "user",
        "event_id" => "event_id",
        "level" => "level",
        "source" => "source",
        "process" => "process",
        _ => return None,
    })
}

fn push_filter(qb: &mut QueryBuilder<'_, Sqlite>, case_id: i64, q: &Query) {
    qb.push(" WHERE case_id = ").push_bind(case_id);
    for term in &q.terms {
        match term {
            Term::Text(s) => {
                qb.push(" AND instr(lower(message), lower(")
                    .push_bind(s.clone())
                    .push(")) > 0");
            }
            Term::Field {
                field: "verdict",
                value,
                negate,
            } => {
                let not = if *negate { "NOT " } else { "" };
                if value == "none" {
                    qb.push(format!(" AND {not}(verdict IS NULL)"));
                } else {
                    qb.push(format!(" AND {not}(verdict IS NOT NULL AND verdict = "))
                        .push_bind(value.clone())
                        .push(")");
                }
            }
            Term::Field {
                field: "pinned",
                value,
                negate,
            } => {
                let yes = (value == "yes") != *negate;
                qb.push(if yes {
                    " AND pinned_at IS NOT NULL"
                } else {
                    " AND pinned_at IS NULL"
                });
            }
            Term::Field {
                field,
                value,
                negate,
            } => {
                let Some(col) = column(field) else { continue };
                let not = if *negate { "NOT " } else { "" };
                if col == "event_id" {
                    qb.push(format!(" AND {not}(event_id = "))
                        .push_bind(value.parse::<i64>().unwrap_or(-1))
                        .push(")");
                } else {
                    qb.push(format!(" AND {not}(lower({col}) = lower("))
                        .push_bind(value.clone())
                        .push("))");
                }
            }
        }
    }
}

pub async fn search(
    pool: &SqlitePool,
    case_id: i64,
    q: &Query,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<Page<Event>, OcError> {
    let limit = limit.clamp(1, 500);
    let mut qb = QueryBuilder::new("SELECT * FROM events");
    push_filter(&mut qb, case_id, q);
    if let Some(c) = cursor {
        qb.push(" AND (ts < ")
            .push_bind(c.ts)
            .push(" OR (ts = ")
            .push_bind(c.ts)
            .push(" AND id < ")
            .push_bind(c.id)
            .push("))");
    }
    qb.push(" ORDER BY ts DESC, id DESC LIMIT ")
        .push_bind(limit + 1);
    let mut items: Vec<Event> = qb.build_query_as().fetch_all(pool).await?;
    let more = items.len() as i64 > limit;
    items.truncate(usize::try_from(limit).unwrap_or(0));
    let next_cursor = more
        .then(|| items.last().map(|e| Cursor { ts: e.ts, id: e.id }.encode()))
        .flatten();
    Ok(Page { items, next_cursor })
}

/// Events newer than the newest one the client has seen, newest first.
pub async fn newer_than(
    pool: &SqlitePool,
    case_id: i64,
    q: &Query,
    after_id: i64,
    limit: i64,
) -> Result<Vec<Event>, OcError> {
    let mut qb = QueryBuilder::new("SELECT * FROM events");
    push_filter(&mut qb, case_id, q);
    qb.push(" AND id > ")
        .push_bind(after_id)
        .push(" ORDER BY ts DESC, id DESC LIMIT ")
        .push_bind(limit);
    Ok(qb.build_query_as().fetch_all(pool).await?)
}

pub async fn count(
    pool: &SqlitePool,
    case_id: i64,
    q: &Query,
    after_id: Option<i64>,
) -> Result<i64, OcError> {
    let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM events");
    push_filter(&mut qb, case_id, q);
    if let Some(a) = after_id {
        qb.push(" AND id > ").push_bind(a);
    }
    Ok(qb.build_query_scalar().fetch_one(pool).await?)
}

pub async fn max_id(pool: &SqlitePool, case_id: i64) -> Result<i64, OcError> {
    Ok(sqlx::query_scalar!(
        r#"SELECT COALESCE(MAX(id), 0) AS "m!: i64" FROM events WHERE case_id = ?"#,
        case_id
    )
    .fetch_one(pool)
    .await?)
}

#[derive(Debug, Clone, Serialize)]
pub struct Facet {
    pub field: &'static str,
    pub values: Vec<(String, i64)>,
}

pub async fn facets(pool: &SqlitePool, case_id: i64, q: &Query) -> Result<Vec<Facet>, OcError> {
    let mut out = Vec::new();
    for field in FACET_FIELDS {
        let Some(col) = column(field) else { continue };
        let mut qb = QueryBuilder::new(format!(
            "SELECT CAST({col} AS TEXT) AS v, COUNT(*) AS n FROM events"
        ));
        push_filter(&mut qb, case_id, q);
        qb.push(" GROUP BY v ORDER BY n DESC, v LIMIT 8");
        let values: Vec<(String, i64)> = qb.build_query_as().fetch_all(pool).await?;
        out.push(Facet { field, values });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct Histogram {
    pub start: i64,
    pub end: i64,
    pub buckets: Vec<u32>,
}

pub async fn histogram(
    pool: &SqlitePool,
    case_id: i64,
    q: &Query,
    n: u16,
) -> Result<Histogram, OcError> {
    let n = i64::from(n.max(1));
    let mut qb = QueryBuilder::new("SELECT MIN(ts), MAX(ts) FROM events");
    push_filter(&mut qb, case_id, q);
    let (start, end): (Option<i64>, Option<i64>) = qb.build_query_as().fetch_one(pool).await?;
    let (Some(start), Some(end)) = (start, end) else {
        return Ok(Histogram {
            start: 0,
            end: 0,
            buckets: vec![0; usize::try_from(n).unwrap_or(1)],
        });
    };
    let width = ((end - start) / n).max(1);
    let mut qb = QueryBuilder::new("SELECT MIN((ts - ");
    qb.push_bind(start)
        .push(") / ")
        .push_bind(width)
        .push(", ")
        .push_bind(n - 1)
        .push(") AS b, COUNT(*) FROM events");
    push_filter(&mut qb, case_id, q);
    qb.push(" GROUP BY b");
    let rows: Vec<(i64, i64)> = qb.build_query_as().fetch_all(pool).await?;
    let mut buckets = vec![0u32; usize::try_from(n).unwrap_or(1)];
    for (b, c) in rows {
        if let Some(slot) = usize::try_from(b).ok().and_then(|b| buckets.get_mut(b)) {
            *slot = u32::try_from(c).unwrap_or(u32::MAX);
        }
    }
    Ok(Histogram {
        start,
        end,
        buckets,
    })
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Event, OcError> {
    sqlx::query_as::<_, Event>("SELECT * FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(OcError::NotFound)
}

/// Returns the previous verdict so the caller can offer Undo.
pub async fn set_verdict(
    pool: &SqlitePool,
    id: i64,
    verdict: Option<&str>,
) -> Result<Option<String>, OcError> {
    if verdict.is_some_and(|v| !VERDICTS.contains(&v)) {
        return Err(OcError::Unprocessable(
            "Choose benign, suspicious or malicious.".into(),
        ));
    }
    let previous = get(pool, id).await?.verdict;
    sqlx::query!("UPDATE events SET verdict = ? WHERE id = ?", verdict, id)
        .execute(pool)
        .await?;
    Ok(previous)
}

pub async fn set_pinned(
    pool: &SqlitePool,
    id: i64,
    pinned: bool,
    now: OffsetDateTime,
) -> Result<(), OcError> {
    let at = pinned.then_some(now);
    let r = sqlx::query!("UPDATE events SET pinned_at = ? WHERE id = ?", at, id)
        .execute(pool)
        .await?;
    if r.rows_affected() == 0 {
        Err(OcError::NotFound)
    } else {
        Ok(())
    }
}

pub async fn pinned(pool: &SqlitePool, case_id: i64) -> Result<Vec<Event>, OcError> {
    Ok(sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE case_id = ? AND pinned_at IS NOT NULL ORDER BY ts, id",
    )
    .bind(case_id)
    .fetch_all(pool)
    .await?)
}

pub async fn insert<'e, X: sqlx::SqliteExecutor<'e>>(
    ex: X,
    case_id: i64,
    e: &NewEvent,
) -> Result<i64, OcError> {
    Ok(sqlx::query_scalar!(
        "INSERT INTO events (case_id, ts, host, user, event_id, level, source, process, pid, parent, command_line, record,
                             integrity, logon_id, source_file, sha256, message)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id AS \"id!\"",
        case_id,
        e.ts,
        e.host,
        e.user,
        e.event_id,
        e.level,
        e.source,
        e.process,
        e.pid,
        e.parent,
        e.command_line,
        e.record,
        e.integrity,
        e.logon_id,
        e.source_file,
        e.sha256,
        e.message
    )
    .fetch_one(ex)
    .await?)
}
