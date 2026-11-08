#![allow(clippy::unwrap_used, clippy::expect_used)]

use oc_kit::OcError;
use oc_kit::trash::{SoftDelete, TRASH_RETENTION, undo_notice};
use oc_kit::ui::{Html, HxAttrs};
use sqlx::SqlitePool;
use time::OffsetDateTime;

struct Things(SqlitePool);

impl SoftDelete for Things {
    async fn soft_delete(&self, id: i64, now: OffsetDateTime) -> Result<(), OcError> {
        sqlx::query("UPDATE things SET deleted_at = ? WHERE id = ?").bind(now).bind(id).execute(&self.0).await?;
        Ok(())
    }
    async fn restore(&self, id: i64) -> Result<(), OcError> {
        sqlx::query("UPDATE things SET deleted_at = NULL WHERE id = ?").bind(id).execute(&self.0).await?;
        Ok(())
    }
    async fn purge_older_than(&self, cutoff: OffsetDateTime) -> Result<u64, OcError> {
        Ok(sqlx::query("DELETE FROM things WHERE deleted_at < ?").bind(cutoff).execute(&self.0).await?.rows_affected())
    }
}

async fn things() -> Things {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE things (id INTEGER PRIMARY KEY, deleted_at TEXT)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO things (id) VALUES (1), (2)").execute(&pool).await.unwrap();
    Things(pool)
}

async fn live(t: &Things) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM things WHERE deleted_at IS NULL").fetch_one(&t.0).await.unwrap()
}

#[tokio::test]
async fn a_deleted_item_can_be_restored_within_30_days() {
    let t = things().await;
    let now = OffsetDateTime::now_utc();
    t.soft_delete(1, now - time::Duration::days(29)).await.unwrap();
    assert_eq!(t.purge_older_than(now - TRASH_RETENTION).await.unwrap(), 0);
    t.restore(1).await.unwrap();
    assert_eq!(live(&t).await, 2);
}

#[tokio::test]
async fn items_older_than_30_days_are_purged_and_younger_ones_are_kept() {
    let t = things().await;
    let now = OffsetDateTime::now_utc();
    t.soft_delete(1, now - time::Duration::days(31)).await.unwrap();
    t.soft_delete(2, now - time::Duration::days(1)).await.unwrap();
    assert_eq!(t.purge_older_than(now - TRASH_RETENTION).await.unwrap(), 1);
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM things").fetch_one(&t.0).await.unwrap();
    assert_eq!(left, 1);
}

#[test]
fn the_undo_notice_names_the_item_and_autohides_after_6s() {
    let h = Html::from(undo_notice("Deleted 'ACME notes'", HxAttrs::post("/notes/1/restore"))).0;
    assert!(h.contains("Deleted &#39;ACME notes&#39;") || h.contains("Deleted 'ACME notes'"), "{h}");
    assert!(
        h.contains("data-oc-autohide=\"6000\"") && h.contains("hx-post=\"/notes/1/restore\"") && h.contains(">Undo<")
    );
}
