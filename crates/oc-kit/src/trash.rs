//! Reversible deletes: act at once, offer Undo for a few seconds, keep the item in Trash for 30 days.

use std::future::Future;
use std::time::Duration;

use time::OffsetDateTime;

use crate::error::OcError;
use crate::ui::{Button, HxAttrs, Notice};

pub const TRASH_RETENTION: time::Duration = time::Duration::days(30);
pub const UNDO_MS: u32 = 6000;

/// Storage stays with the app; the kit only drives the lifecycle.
pub trait SoftDelete {
    fn soft_delete(&self, id: i64, now: OffsetDateTime) -> impl Future<Output = Result<(), OcError>> + Send;
    fn restore(&self, id: i64) -> impl Future<Output = Result<(), OcError>> + Send;
    fn purge_older_than(&self, cutoff: OffsetDateTime) -> impl Future<Output = Result<u64, OcError>> + Send;
}

pub fn undo_notice(text: &str, restore: HxAttrs) -> Notice {
    Notice::success(text).action(Button::default("Undo").attrs(restore)).autohide(UNDO_MS)
}

pub fn spawn_purger<S: SoftDelete + Send + Sync + 'static>(store: S, every: Duration) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(every);
        loop {
            tick.tick().await;
            match store.purge_older_than(OffsetDateTime::now_utc() - TRASH_RETENTION).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(purged = n, "emptied expired Trash items"),
                Err(e) => tracing::warn!(error = %e, "Trash purge failed"),
            }
        }
    });
}
