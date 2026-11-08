use std::time::Duration;

use rand::Rng;

use crate::domain::{cases, events, seed};
use crate::state::{AppState, Ingest, IngestEvent};

/// Simulates a collector appending events, so the live views have something to show.
pub fn spawn(st: &AppState, cfg: &Ingest) {
    if !cfg.enabled || cfg.per_second <= 0.0 {
        return;
    }
    let st = st.clone();
    let every = Duration::from_secs_f64(1.0 / cfg.per_second.min(100.0));
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(every);
        let mut record = 900_000;
        loop {
            tick.tick().await;
            let Ok(all) = cases::list(&st.pool).await else {
                continue;
            };
            if all.is_empty() {
                continue;
            }
            let (case_id, e) = {
                let mut rng = rand::rng();
                let case_id = all[rng.random_range(0..all.len())].id;
                record += 1;
                (case_id, seed::event(&mut rng, seed::now_ms(), record))
            };
            match events::insert(&st.pool, case_id, &e).await {
                Ok(event_id) => st.ingest.send(IngestEvent { case_id, event_id }),
                Err(err) => tracing::warn!(error = %err, "simulated ingest failed"),
            }
        }
    });
}
