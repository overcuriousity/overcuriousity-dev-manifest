use askama::Template;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use oc_kit::OcError;
use oc_kit::sse::{SseEvent, sse_response};
use oc_kit::ui::Html;
use serde::Deserialize;

use super::timeline::{PAGE_SIZE, Rows, ts, urlencode};
use super::{Ctx, render};
use crate::domain::events;
use crate::domain::query;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cases/{id}/live", get(live))
        .route("/cases/{id}/timeline/histogram", get(histogram))
        .route("/cases/{id}/timeline/new-count", get(new_count))
        .route("/cases/{id}/timeline/new", get(new_rows))
}

async fn live(State(st): State<AppState>, _ctx: Ctx, Path(id): Path<i64>) -> Response {
    sse_response(st.ingest.subscribe(), move |e| {
        (e.case_id == id).then(|| SseEvent { name: "ingest", data: String::new() })
    })
}

#[derive(Deserialize)]
struct LiveQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    after: i64,
}

const W: f64 = 1000.0;
const H: f64 = 40.0;

#[derive(Template)]
#[template(path = "timeline/histogram.html")]
struct Histogram {
    label: String,
    bars: Vec<(f64, f64, f64, f64)>,
    window: Option<(f64, f64)>,
    start: String,
    end: String,
}

async fn histogram(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
    Query(lq): Query<LiveQuery>,
) -> Result<Response, OcError> {
    let Ok(q) = query::parse(&lq.q) else {
        return Ok((StatusCode::BAD_REQUEST, "Fix the search to see the histogram.").into_response());
    };
    let h = events::histogram(&st.pool, id, &q, 60).await?;
    let total: u32 = h.buckets.iter().sum();
    let max = f64::from(h.buckets.iter().copied().max().unwrap_or(0).max(1));
    let n = h.buckets.len().max(1) as f64;
    let bw = W / n;
    let bars = h
        .buckets
        .iter()
        .enumerate()
        .filter(|(_, c)| **c > 0)
        .map(|(i, c)| {
            let bh = (f64::from(*c) / max * H).max(1.0);
            (i as f64 * bw + 1.0, H - bh, (bw - 2.0).max(1.0), bh)
        })
        .collect();
    let visible = events::search(&st.pool, id, &q, None, PAGE_SIZE).await?.items;
    let span = (h.end - h.start).max(1) as f64;
    let window = match (visible.last(), visible.first()) {
        (Some(old), Some(new)) if h.end > h.start => {
            let x = (old.ts - h.start) as f64 / span * W;
            Some((x, ((new.ts - old.ts) as f64 / span * W).max(2.0)))
        }
        _ => None,
    };
    let (start, end) = (ts(&h.start), ts(&h.end));
    let label = if total == 0 {
        "No events to chart.".to_owned()
    } else {
        format!("{total} events between {start} and {end}; the marked range is on screen.")
    };
    Ok(render(&Histogram { label, bars, window, start, end }).into_response())
}

async fn new_count(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
    Query(lq): Query<LiveQuery>,
) -> Result<Response, OcError> {
    let Ok(q) = query::parse(&lq.q) else {
        return Ok(Html::default().into_response());
    };
    let n = events::count(&st.pool, id, &q, Some(lq.after)).await?;
    if n == 0 {
        return Ok(Html::default().into_response());
    }
    let label = if n == 1 { "1 new event".to_owned() } else { format!("{n} new events") };
    let url = format!("/cases/{id}/timeline/new?after={}&q={}", lq.after, urlencode(&q.to_string()));
    Ok(Html::raw(format!(
        "<span class=\"oc-row\"><span>{label}</span><button type=\"button\" class=\"oc-btn\" id=\"oc-ingest-show\" hx-get=\"{url}\" hx-target=\"#oc-rows\" hx-swap=\"afterbegin\">Show</button></span>"
    ))
    .into_response())
}

async fn new_rows(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
    Query(lq): Query<LiveQuery>,
) -> Result<Response, OcError> {
    let Ok(q) = query::parse(&lq.q) else {
        return Ok(Html::default().into_response());
    };
    let newer = events::newer_than(&st.pool, id, &q, lq.after, 500).await?;
    let max = events::max_id(&st.pool, id).await?.max(lq.after);
    let rows = render(&Rows { case_id: id, events: &newer, q: q.to_string(), next: None });
    Ok(Html::join([rows, reset_cursor(max)]).into_response())
}

/// Out-of-band updates for a table response: the newest id seen, and an empty new-events notice.
/// Wrapped in <template> because the response is parsed in a table context.
pub fn reset_cursor(after: i64) -> Html {
    Html::raw(format!(
        "<template><input type=\"hidden\" id=\"after\" name=\"after\" value=\"{after}\" hx-swap-oob=\"true\"></template>\
         <template><div id=\"oc-ingest-notice\" hx-swap-oob=\"innerHTML\"></div></template>"
    ))
}
