use askama::Template;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use oc_kit::OcError;
use oc_kit::ui::{Button, Empty, Fact, Facts, Html, HxAttrs, Notice};
use serde::Deserialize;

use super::timeline::{Rows, ts};
use super::{Ctx, render};
use crate::domain::events::{self, Event};
use crate::domain::{cases, now, query};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/events/{id}/inspector", get(inspector))
        .route("/events/{id}/verdict", post(verdict))
        .route("/events/{id}/pin", post(pin))
        .route("/events/{id}/unpin", post(unpin))
        .route("/cases/{id}/story", get(story))
        .route("/cases/{id}/summary", post(summary))
}

#[derive(Template)]
#[template(path = "timeline/verdict.html")]
pub struct Verdict {
    pub event_id: i64,
    pub state: String,
    pub oob: bool,
}

#[derive(Template)]
#[template(path = "timeline/inspector.html")]
struct Inspector<'a> {
    e: &'a Event,
    verdict: Html,
    facts: Facts,
    notice: Option<Notice>,
}

impl Inspector<'_> {
    fn button(&self, value: &str, label: &str) -> Button {
        Button::default(label).attrs(
            HxAttrs::post(format!("/events/{}/verdict", self.e.id))
                .vals(format!("{{\"verdict\":\"{value}\"}}"))
                .target(format!("#verdict-{}", self.e.id))
                .swap("outerHTML")
                .with(
                    "data-oc-optimistic",
                    format!("#verdict-{0}, #verdict-cell-{0} .oc-verdict", self.e.id),
                )
                .with("data-oc-optimistic-state", value),
        )
    }
}

#[derive(Template)]
#[template(path = "timeline/story.html")]
struct Story<'a> {
    case_id: i64,
    summary: &'a str,
    pinned: &'a [Event],
    empty: Empty,
    notice: Option<Notice>,
}

impl Story<'_> {
    fn time(&self, e: &Event) -> String {
        ts(&e.ts)
    }
}

fn facts(e: &Event) -> Facts {
    let opt = |v: &Option<String>| v.clone().unwrap_or_default();
    let mut rows = vec![
        Fact::new("Time", ts(&e.ts)).mono().copy(),
        Fact::new("Host", e.host.clone()).copy(),
        Fact::new("User", e.user.clone()).copy(),
        Fact::new("Event ID", e.event_id.to_string()).mono(),
        Fact::new("Level", e.level.clone()),
        Fact::new("Source", e.source.clone()),
        Fact::new("Process", e.process.clone()),
        Fact::new("PID", e.pid.map(|p| p.to_string()).unwrap_or_default()).mono(),
        Fact::new("Parent", opt(&e.parent)),
    ];
    if e.command_line.is_some() {
        rows.push(
            Fact::new("Command line", opt(&e.command_line))
                .mono()
                .copy(),
        );
    }
    if e.sha256.is_some() {
        rows.push(Fact::new("SHA-256", opt(&e.sha256)).mono().copy());
    }
    rows.extend([
        Fact::new("Integrity", opt(&e.integrity)),
        Fact::new("Logon ID", opt(&e.logon_id)).mono().copy(),
        Fact::new("Source file", e.source_file.clone())
            .mono()
            .copy(),
        Fact::new("Record", e.record.to_string()).mono(),
        Fact::new("Event", format!("#{}", e.id)).mono().copy(),
    ]);
    Facts { rows }
}

fn verdict_html(e: &Event, oob: bool) -> Html {
    render(&Verdict {
        event_id: e.id,
        state: e.verdict.clone().unwrap_or_default(),
        oob,
    })
}

async fn inspector(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
) -> Result<Response, OcError> {
    let e = events::get(&st.pool, id).await?;
    let notice = e.pinned_at.map(|_| Notice::info("Pinned to the story"));
    Ok(render(&Inspector {
        e: &e,
        verdict: verdict_html(&e, false),
        facts: facts(&e),
        notice,
    })
    .into_response())
}

#[derive(Deserialize)]
struct VerdictForm {
    #[serde(default)]
    verdict: String,
}

async fn verdict(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
    Form(f): Form<VerdictForm>,
) -> Result<Response, OcError> {
    let new = (!f.verdict.is_empty()).then_some(f.verdict.as_str());
    let previous = events::set_verdict(&st.pool, id, new).await?;
    let e = events::get(&st.pool, id).await?;
    let undo = Button::default("Undo").attrs(
        HxAttrs::post(format!("/events/{id}/verdict"))
            .vals(format!(
                "{{\"verdict\":\"{}\"}}",
                previous.unwrap_or_default()
            ))
            .target(format!("#verdict-{id}"))
            .swap("outerHTML"),
    );
    let text = match new {
        Some(v) => format!("Marked as {v}"),
        None => "Verdict cleared".to_owned(),
    };
    let notice = Notice::success(text)
        .action(undo)
        .autohide(oc_kit::trash::UNDO_MS)
        .oob();
    Ok(Html::join([verdict_html(&e, false), cell(&e), notice]).into_response())
}

/// The row's verdict cell, swapped out of band so the table agrees with the inspector.
fn cell(e: &Event) -> Html {
    Html(format!(
        "<td class=\"oc-verdict-cell\" id=\"verdict-cell-{}\" hx-swap-oob=\"true\">{}</td>",
        e.id,
        render(&Verdict {
            event_id: e.id,
            state: e.verdict.clone().unwrap_or_default(),
            oob: true
        })
    ))
}

fn current_query(headers: &HeaderMap) -> String {
    headers
        .get("hx-current-url")
        .and_then(|v| v.to_str().ok())
        .and_then(|u| url_query(u, "q"))
        .unwrap_or_default()
}

fn url_query(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| decode(v))
    })
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                match u8::from_str_radix(
                    std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                    16,
                ) {
                    Ok(b) => {
                        out.push(b);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn set_pin(
    st: &AppState,
    headers: &HeaderMap,
    id: i64,
    pinned: bool,
) -> Result<Response, OcError> {
    events::set_pinned(&st.pool, id, pinned, now()).await?;
    let e = events::get(&st.pool, id).await?;
    let q = query::parse(&current_query(headers))
        .unwrap_or_default()
        .to_string();
    let row = render(&Rows {
        case_id: e.case_id,
        events: std::slice::from_ref(&e),
        q,
        next: None,
    });
    let story = story_html(st, e.case_id, None).await?;
    Ok(Html::join([
        row,
        Html(format!(
            "<div id=\"pane-body\" class=\"oc-stack\" hx-swap-oob=\"true\">{story}</div>"
        )),
    ])
    .into_response())
}

async fn pin(
    State(st): State<AppState>,
    _ctx: Ctx,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, OcError> {
    set_pin(&st, &headers, id, true).await
}

async fn unpin(
    State(st): State<AppState>,
    _ctx: Ctx,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, OcError> {
    set_pin(&st, &headers, id, false).await
}

async fn story_html(st: &AppState, case_id: i64, notice: Option<Notice>) -> Result<Html, OcError> {
    let case = cases::get(&st.pool, case_id).await?;
    let pinned = events::pinned(&st.pool, case_id).await?;
    Ok(render(&Story {
        case_id,
        summary: &case.summary,
        pinned: &pinned,
        empty: Empty::new("Nothing pinned yet. Pin events with P to build the story."),
        notice,
    }))
}

async fn story(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
) -> Result<Response, OcError> {
    Ok(story_html(&st, id, None).await?.into_response())
}

#[derive(Deserialize)]
struct SummaryForm {
    #[serde(default)]
    summary: String,
}

async fn summary(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
    Form(f): Form<SummaryForm>,
) -> Result<Response, OcError> {
    let notice = match cases::set_summary(&st.pool, id, f.summary.trim()).await {
        Ok(()) => Notice::success("Summary saved"),
        Err(e @ OcError::Unprocessable(_)) => Notice::danger(e.to_string()),
        Err(e) => return Err(e),
    };
    Ok(story_html(&st, id, Some(notice)).await?.into_response())
}
