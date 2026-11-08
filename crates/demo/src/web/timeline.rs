use askama::Template;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use oc_kit::OcError;
use oc_kit::prefs::Density;
use oc_kit::ui::{
    Button, CopyButton, Empty, Html, HxAttrs, Icon, IconButton, Notice, Page, Pill, Profile,
    RowActions, SearchBox, StatusBar, TopBar,
};
use serde::Deserialize;

use super::{Ctx, render};
use crate::domain::events::{self, Cursor, Event, Facet};
use crate::domain::query::{self, Query as Q, QueryError, Term};
use crate::domain::{cases, from_ms};
use crate::state::{AppState, BRANDING};

pub const PAGE_SIZE: i64 = 100;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cases/{id}/timeline", get(page))
        .route("/cases/{id}/timeline/rows", get(rows))
        .route("/cases/{id}/timeline/facets", get(facets))
}

#[derive(Deserialize, Default)]
pub struct TimelineQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub undo: Option<String>,
}

pub fn timeline_url(case_id: i64, q: &str) -> String {
    format!("/cases/{case_id}/timeline?q={}", urlencode(q))
}

pub fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b))
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn shell(url: &str) -> HxAttrs {
    super::notes::shell_get(url)
}

pub fn ts(ms: &i64) -> String {
    let f = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    from_ms(*ms).format(&f).unwrap_or_default()
}

pub fn verdict(e: &Event) -> Html {
    render(&super::inspector::Verdict {
        event_id: e.id,
        state: e.verdict.clone().unwrap_or_default(),
        oob: true,
    })
}

pub fn level_class(level: &str) -> &'static str {
    match level {
        "error" => "danger",
        "warning" => "warning",
        _ => "none",
    }
}

pub fn level_icon(level: &str) -> Html {
    let icon = match level {
        "error" => Icon::Danger,
        "warning" => Icon::Warning,
        _ => Icon::Info,
    };
    icon.sized(oc_kit::ui::IconSize::Row)
}

#[derive(Template)]
#[template(path = "timeline/rows.html")]
pub struct Rows<'a> {
    pub case_id: i64,
    pub events: &'a [Event],
    pub q: String,
    pub next: Option<String>,
}

impl Rows<'_> {
    fn actions(&self, e: &Event) -> RowActions {
        let host = Term::Field {
            field: "host",
            value: e.host.clone(),
            negate: false,
        };
        let not_host = Term::Field {
            field: "host",
            value: e.host.clone(),
            negate: true,
        };
        let parsed = query::parse(&self.q).unwrap_or_default();
        let pin_label = if e.pinned_at.is_some() {
            "Unpin"
        } else {
            "Pin to story"
        };
        let pin_url = format!(
            "/events/{}/{}",
            e.id,
            if e.pinned_at.is_some() {
                "unpin"
            } else {
                "pin"
            }
        );
        RowActions {
            actions: vec![
                IconButton::new(Icon::FilterTo, format!("Filter to {}", e.host))
                    .shortcut("F")
                    .row(true)
                    .attrs(shell(&timeline_url(
                        self.case_id,
                        &parsed.with(host).to_string(),
                    ))),
                IconButton::new(Icon::Exclude, format!("Exclude {}", e.host))
                    .shortcut("X")
                    .row(true)
                    .attrs(shell(&timeline_url(
                        self.case_id,
                        &parsed.with(not_host).to_string(),
                    ))),
                IconButton::new(Icon::Pin, pin_label)
                    .shortcut("P")
                    .row(true)
                    .pressed(e.pinned_at.is_some())
                    .attrs(
                        HxAttrs::post(pin_url)
                            .target("closest tr")
                            .swap("outerHTML"),
                    ),
            ],
        }
    }

    fn copy(&self, value: &str, label: &str) -> CopyButton {
        CopyButton::new(value, label)
    }

    fn next_url(&self) -> String {
        match &self.next {
            Some(c) => format!(
                "/cases/{}/timeline/rows?q={}&cursor={c}",
                self.case_id,
                urlencode(&self.q)
            ),
            None => String::new(),
        }
    }
}

#[derive(Template)]
#[template(path = "timeline/facets.html")]
pub struct Facets<'a> {
    pub case_id: i64,
    pub q: &'a Q,
    pub facets: &'a [Facet],
}

impl Facets<'_> {
    fn add(&self, field: &str, value: &str) -> HxAttrs {
        let Some(field) = query::FIELDS.iter().find(|f| **f == field).copied() else {
            return HxAttrs::new();
        };
        shell(&timeline_url(
            self.case_id,
            &self
                .q
                .with(Term::Field {
                    field,
                    value: value.to_owned(),
                    negate: false,
                })
                .to_string(),
        ))
    }

    fn title(field: &str) -> &'static str {
        match field {
            "host" => "Host",
            "user" => "User",
            "event_id" => "Event ID",
            _ => "Level",
        }
    }
}

#[derive(Template)]
#[template(path = "timeline/page.html")]
struct Main<'a> {
    case_id: i64,
    pills: Vec<Pill>,
    error: Option<Notice>,
    matched: i64,
    rows: Html,
    empty: Option<Empty>,
    after: i64,
    q: &'a str,
}

fn pills(case_id: i64, q: &Q) -> Vec<Pill> {
    q.terms
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let rest = q.without(i).to_string();
            let url = format!(
                "/cases/{case_id}/timeline?q={}&undo={}",
                urlencode(&rest),
                urlencode(&q.to_string())
            );
            Pill {
                text: t.to_string(),
                exclude: matches!(t, Term::Field { negate: true, .. }),
                remove: shell(&url),
            }
        })
        .collect()
}

fn parse_or_error(raw: &str) -> (Q, Option<QueryError>) {
    match query::parse(raw) {
        Ok(q) => (q, None),
        Err(e) => (Q::default(), Some(e)),
    }
}

pub const SHORTCUTS: [(&str, &str); 11] = [
    ("?", "Show shortcuts"),
    ("/", "Focus search"),
    ("Esc", "Close the open layer"),
    ("j", "Next event"),
    ("k", "Previous event"),
    ("Enter", "Open the selected event"),
    ("F", "Filter to the selected event's host"),
    ("X", "Exclude the selected event's host"),
    ("P", "Pin or unpin the selected event"),
    ("S", "Show or hide the story"),
    ("\\", "Show or hide filters"),
];

pub fn workbench(ctx: &Ctx, case: &cases::Case, q: &str, body: Html) -> Page {
    let mut top = TopBar::new(BRANDING, "/notes", ctx.prefs, Some(ctx.user.clone()));
    top.breadcrumb = vec![
        ("/cases".into(), "Cases".into()),
        (format!("/cases/{}/timeline", case.id), case.name.clone()),
    ];
    top.search = Some(SearchBox {
        name: "q".into(),
        value: q.to_owned(),
        label: "Search events, for example host:ws-01 -user:SYSTEM powershell".into(),
        attrs: shell(&format!("/cases/{}/timeline", case.id)).with("hx-trigger", "submit"),
    });
    let mut page = Page::new(case.name.clone(), Profile::Workbench, top, body, ctx.source);
    page.extra_css = vec![super::demo_css()];
    page.rail = vec![
        IconButton::new(Icon::Facets, "Show or hide filters")
            .shortcut("\\")
            .attrs(HxAttrs::new().with("data-oc-toggle", "facets")),
        IconButton::new(Icon::Story, "Show or hide the story")
            .shortcut("S")
            .pressed(false)
            .attrs(
                HxAttrs::get(format!("/cases/{}/story", case.id))
                    .target("#pane-body")
                    .with("data-oc-toggle", "pane")
                    .with("hx-trigger", "click once"),
            ),
        IconButton::new(Icon::Notes, "Notes of this case").attrs(super::notes::shell_get("/notes")),
    ];
    page.shortcuts = SHORTCUTS.to_vec();
    page.pane = Some(Html(
        "<div id=\"pane-body\" class=\"oc-stack\"></div>".into(),
    ));
    page.inspector = Some(Html::from(Empty::new(
        "Select an event to see its details.",
    )));
    page
}

async fn page(
    State(st): State<AppState>,
    ctx: Ctx,
    Path(id): Path<i64>,
    Query(tq): Query<TimelineQuery>,
) -> Result<Response, OcError> {
    let case = cases::get(&st.pool, id).await?;
    let (q, error) = parse_or_error(&tq.q);
    let page = events::search(&st.pool, id, &q, None, PAGE_SIZE).await?;
    let matched = events::count(&st.pool, id, &q, None).await?;
    let after = events::max_id(&st.pool, id).await?;
    let fac = events::facets(&st.pool, id, &q).await?;
    let rows = render(&Rows {
        case_id: id,
        events: &page.items,
        q: q.to_string(),
        next: page.next_cursor.clone(),
    });
    let empty = page.items.is_empty().then(|| {
        if case.events == 0 {
            Empty::new("No events in this case yet. New events appear here as they are ingested.")
        } else {
            Empty::new("No events match this search. Remove a filter to see more.")
        }
    });
    let qs = q.to_string();
    let body = render(&Main {
        case_id: id,
        pills: pills(id, &q),
        error: error.map(|e| Notice::danger(e.to_string())),
        matched,
        rows,
        empty,
        after,
        q: &qs,
    });
    let mut p = workbench(&ctx, &case, &tq.q, body);
    p.facets = Some(render(&Facets {
        case_id: id,
        q: &q,
        facets: &fac,
    }));
    let density = if ctx.prefs.density == Density::Compact {
        "Compact"
    } else {
        "Default size"
    };
    p.statusbar = Some(StatusBar {
        items: vec![
            Html(format!(
                "<span id=\"oc-match-count\">{matched} of {} events</span>",
                case.events
            )),
            Html::from(oc_kit::ui::Link::data("/cases", "All cases")),
            Html(format!(
                "<span class=\"oc-status oc-status--success\" id=\"oc-live-state\">{}<span>Live</span></span>",
                Icon::Success.sized(oc_kit::ui::IconSize::Row)
            )),
            Html(density.to_owned()),
        ],
    });
    if let Some(prev) = tq.undo.filter(|u| *u != tq.q) {
        let removed = match query::parse(&prev) {
            Ok(p) => p
                .terms
                .into_iter()
                .find(|t| !q.terms.contains(t))
                .map(|t| t.to_string())
                .unwrap_or_default(),
            Err(_) => String::new(),
        };
        let undo = Button::default("Undo").attrs(shell(&timeline_url(id, &prev)));
        p.notices = Html::from(
            Notice::success(format!("Removed filter {removed}"))
                .action(undo)
                .autohide(oc_kit::trash::UNDO_MS),
        );
    }
    Ok(p.into_response())
}

fn bad_query(e: QueryError) -> Response {
    (StatusCode::BAD_REQUEST, e.to_string()).into_response()
}

async fn rows(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
    Query(tq): Query<TimelineQuery>,
) -> Result<Response, OcError> {
    let q = match query::parse(&tq.q) {
        Ok(q) => q,
        Err(e) => return Ok(bad_query(e)),
    };
    let cursor = tq.cursor.as_deref().map(Cursor::decode).transpose()?;
    let page = events::search(&st.pool, id, &q, cursor, PAGE_SIZE).await?;
    Ok(render(&Rows {
        case_id: id,
        events: &page.items,
        q: q.to_string(),
        next: page.next_cursor,
    })
    .into_response())
}

async fn facets(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
    Query(tq): Query<TimelineQuery>,
) -> Result<Response, OcError> {
    let q = match query::parse(&tq.q) {
        Ok(q) => q,
        Err(e) => return Ok(bad_query(e)),
    };
    let fac = events::facets(&st.pool, id, &q).await?;
    Ok(render(&Facets {
        case_id: id,
        q: &q,
        facets: &fac,
    })
    .into_response())
}
