#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use demo::domain::events::{self, Cursor};
use demo::domain::notes::{self, NotesStore};
use demo::domain::query::parse;
use demo::domain::{cases, now, seed};
use oc_kit::trash::{SoftDelete, TRASH_RETENTION};

async fn seeded() -> (tempfile::TempDir, sqlx::SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = demo::db::open(dir.path()).await.unwrap();
    assert!(
        seed::ensure_demo_user(&pool, now())
            .await
            .unwrap()
            .is_some()
    );
    seed::seed(&pool, now()).await.unwrap();
    (dir, pool)
}

#[tokio::test]
async fn seed_is_deterministic_and_creates_the_demo_user_once() {
    let (_a, p1) = seeded().await;
    let (_b, p2) = seeded().await;
    let sig = |p: sqlx::SqlitePool| async move {
        sqlx::query_scalar::<_, String>("SELECT group_concat(host || user || ts, ',') FROM (SELECT * FROM events ORDER BY id LIMIT 50)")
            .fetch_one(&p)
            .await
            .unwrap()
    };
    assert_eq!(sig(p1.clone()).await, sig(p2).await);
    assert!(seed::ensure_demo_user(&p1, now()).await.unwrap().is_none());
    assert!(!seed::seed(&p1, now()).await.unwrap());
}

#[tokio::test]
async fn keyset_pagination_returns_every_event_exactly_once() {
    let (_d, pool) = seeded().await;
    let case = cases::list(&pool)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.name == "Lab ws-07")
        .unwrap();
    let q = parse("").unwrap();
    let (mut seen, mut cursor) = (HashSet::new(), None);
    loop {
        let page = events::search(&pool, case.id, &q, cursor, 333)
            .await
            .unwrap();
        for e in &page.items {
            assert!(seen.insert(e.id), "event {} repeated", e.id);
        }
        match page.next_cursor {
            Some(c) => cursor = Some(Cursor::decode(&c).unwrap()),
            None => break,
        }
    }
    assert_eq!(seen.len() as i64, case.events);
}

#[tokio::test]
async fn facet_counts_match_the_filtered_result_and_hostile_values_are_bound() {
    let (_d, pool) = seeded().await;
    let case = cases::list(&pool).await.unwrap()[0].clone();
    let q = parse("level:warning").unwrap();
    let total = events::count(&pool, case.id, &q, None).await.unwrap();
    let facets = events::facets(&pool, case.id, &q).await.unwrap();
    let level = facets.iter().find(|f| f.field == "level").unwrap();
    assert_eq!(level.values, vec![("warning".to_owned(), total)]);
    let hostile = parse(r#"user:"x' OR 1=1 --""#).unwrap();
    assert_eq!(
        events::count(&pool, case.id, &hostile, None).await.unwrap(),
        0
    );
}

#[tokio::test]
async fn a_deleted_note_can_be_restored_within_30_days() {
    let (_d, pool) = seeded().await;
    let note = notes::list(&pool, None, "", false).await.unwrap()[0].clone();
    let store = NotesStore(pool.clone());
    store
        .soft_delete(note.id, now() - time::Duration::days(29))
        .await
        .unwrap();
    assert_eq!(
        store
            .purge_older_than(now() - TRASH_RETENTION)
            .await
            .unwrap(),
        0
    );
    assert!(
        notes::list(&pool, None, "", true)
            .await
            .unwrap()
            .iter()
            .any(|n| n.id == note.id)
    );
    store.restore(note.id).await.unwrap();
    assert!(
        notes::list(&pool, None, "", false)
            .await
            .unwrap()
            .iter()
            .any(|n| n.id == note.id)
    );
}

#[tokio::test]
async fn purging_a_case_needs_the_exact_name_and_removes_events_and_notes() {
    let (_d, pool) = seeded().await;
    let case = cases::list(&pool)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.name == "ACME breach")
        .unwrap();
    assert!(cases::purge(&pool, case.id, "acme breach").await.is_err());
    cases::purge(&pool, case.id, "ACME breach").await.unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE case_id = ?")
        .bind(case.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let notes_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE case_id = ?")
        .bind(case.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((left, notes_left), (0, 0));
}
