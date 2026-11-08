use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use oc_kit::OcError;
use oc_kit::trash::{SoftDelete, undo_notice};
use oc_kit::ui::{Button, Empty, Fact, Facts, Html, HxAttrs, Icon, Notice, SearchBox, Select};
use serde::Deserialize;

use super::{Ctx, everyday, paragraphs, render, when};
use crate::domain::notes::{self, Note, NoteInput, NotesStore};
use crate::domain::{cases, now};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notes", get(index).post(create))
        .route("/notes/new", get(new_form))
        .route("/notes/{id}", get(show).post(update))
        .route("/notes/{id}/edit", get(edit_form))
        .route("/notes/{id}/delete", post(delete))
        .route("/notes/{id}/restore", post(restore))
        .route("/trash", get(trash))
        .route("/trash/{id}/restore", post(restore))
        .route("/search", get(search))
}

/// Swaps the whole shell, so a button can move to another view without a full reload.
pub fn shell_get(url: &str) -> HxAttrs {
    HxAttrs::get(url).target("#oc-shell").swap("outerHTML").with("hx-select", "#oc-shell").with("hx-push-url", "true")
}

#[derive(Template)]
#[template(path = "notes/list.html")]
struct NoteList<'a> {
    notes: &'a [Note],
    current: Option<i64>,
}

impl NoteList<'_> {
    fn is_current(&self, n: &Note) -> bool {
        self.current == Some(n.id)
    }

    fn meta(&self, n: &Note) -> String {
        let mut parts = vec![when(&n.updated_at)];
        if let Some(c) = &n.case_name {
            parts.push(c.clone());
        }
        parts.join(" · ")
    }
}

#[derive(Template)]
#[template(path = "notes/page.html")]
struct NotesPage {
    new_button: Option<Button>,
    search: SearchBox,
    list: Html,
    detail: Html,
    related: Option<Html>,
    focus: &'static str,
}

#[derive(Template)]
#[template(path = "notes/detail.html")]
struct Detail<'a> {
    note: &'a Note,
    paras: Vec<String>,
    facts: Facts,
    edit: Button,
    delete: Button,
}

#[derive(Template)]
#[template(path = "notes/related.html")]
struct Related<'a> {
    notes: Vec<&'a Note>,
}

#[derive(Template)]
#[template(path = "notes/form.html")]
struct NoteForm {
    action: String,
    heading: &'static str,
    title: String,
    body: String,
    tags: String,
    case: Select,
    notice: Option<Notice>,
    cancel: String,
}

#[derive(Template)]
#[template(path = "notes/trash.html")]
struct Trash<'a> {
    notes: &'a [Note],
    empty: Empty,
}

#[derive(Template)]
#[template(path = "notes/search.html")]
struct SearchPage<'a> {
    search: SearchBox,
    q: &'a str,
    notes: Vec<Note>,
    cases: Vec<cases::Case>,
}

#[derive(Deserialize, Default)]
struct ListQuery {
    #[serde(default)]
    q: String,
}

fn search_box(q: &str) -> SearchBox {
    SearchBox {
        name: "q".into(),
        value: q.to_owned(),
        label: "Filter notes by title".into(),
        attrs: HxAttrs::get("/notes")
            .target("#oc-note-list")
            .swap("outerHTML")
            .with("hx-select", "#oc-note-list")
            .with("hx-trigger", "input changed delay:250ms from:find input, search")
            .with("hx-push-url", "true"),
    }
}

fn new_button() -> Button {
    Button::primary("New note").icon(Icon::New).class("oc-fab").attrs(shell_get("/notes/new"))
}

async fn page(
    st: &AppState,
    ctx: &Ctx,
    filter: &str,
    current: Option<&Note>,
    detail: Html,
    focus: &'static str,
) -> Result<oc_kit::ui::Page, OcError> {
    page_with(st, ctx, filter, current, detail, focus, true).await
}

#[allow(clippy::too_many_arguments)]
async fn page_with(
    st: &AppState,
    ctx: &Ctx,
    filter: &str,
    current: Option<&Note>,
    detail: Html,
    focus: &'static str,
    offer_new: bool,
) -> Result<oc_kit::ui::Page, OcError> {
    let all = notes::list(&st.pool, None, filter, false).await?;
    let list = render(&NoteList { notes: &all, current: current.map(|n| n.id) });
    let related = current.and_then(|n| {
        let rel: Vec<&Note> =
            all.iter().filter(|o| o.id != n.id && o.case_id.is_some() && o.case_id == n.case_id).collect();
        (!rel.is_empty()).then(|| render(&Related { notes: rel }))
    });
    let body = render(&NotesPage {
        new_button: offer_new.then(new_button),
        search: search_box(filter),
        list,
        detail,
        related,
        focus,
    });
    let title = current.map_or("Notes".to_owned(), |n| n.title.clone());
    Ok(everyday(ctx, "/notes", &title, body))
}

fn empty_detail() -> Html {
    Html::from(Empty::new("Select a note to read it, or start a new one."))
}

async fn index(State(st): State<AppState>, ctx: Ctx, Query(q): Query<ListQuery>) -> Result<Response, OcError> {
    Ok(page(&st, &ctx, &q.q, None, empty_detail(), "list").await?.into_response())
}

fn detail(note: &Note, origin: &str) -> Html {
    let mut facts = vec![
        Fact::new("ID", note.id.to_string()).mono().copy(),
        Fact::new("Author", note.author.clone()),
        Fact::new("Created", when(&note.created_at)).mono(),
        Fact::new("Updated", when(&note.updated_at)).mono(),
    ];
    if let (Some(id), Some(name)) = (note.case_id, &note.case_name) {
        facts.push(Fact::new("Case", name.clone()).href(format!("/cases/{id}/timeline")));
    }
    let url = format!("{origin}/notes/{}", note.id);
    facts.push(Fact::new("Link", url.clone()).mono().copy().href(format!("/notes/{}", note.id)));
    let delete = Button::danger("Delete").icon(Icon::Delete).attrs(
        HxAttrs::post(format!("/notes/{}/delete", note.id))
            .target("#oc-shell")
            .swap("outerHTML")
            .with("hx-select", "#oc-shell")
            .with("hx-select-oob", "#oc-notices")
            .with("hx-push-url", "/notes"),
    );
    render(&Detail {
        note,
        paras: paragraphs(&note.body),
        facts: Facts { rows: facts },
        edit: Button::default("Edit").icon(Icon::Edit).attrs(shell_get(&format!("/notes/{}/edit", note.id))),
        delete,
    })
}

async fn show(State(st): State<AppState>, ctx: Ctx, Path(id): Path<i64>) -> Result<Response, OcError> {
    let note = notes::get(&st.pool, id).await?;
    if note.deleted_at.is_some() {
        return Err(OcError::NotFound);
    }
    let d = detail(&note, &st.auth.dep.origin);
    Ok(page(&st, &ctx, "", Some(&note), d, "detail").await?.into_response())
}

#[derive(Deserialize)]
struct NoteFormData {
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    case_id: String,
}

impl NoteFormData {
    fn input(&self) -> NoteInput {
        NoteInput {
            case_id: self.case_id.parse().ok(),
            title: self.title.clone(),
            body: self.body.replace("\r\n", "\n"),
            tags: notes::parse_tags(&self.tags),
        }
    }
}

async fn case_select(st: &AppState, selected: Option<i64>) -> Result<Select, OcError> {
    let mut options = vec![(String::new(), "No case".to_owned())];
    options.extend(cases::list(&st.pool).await?.into_iter().map(|c| (c.id.to_string(), c.name)));
    Ok(Select {
        name: "case_id".into(),
        label: "Case".into(),
        options,
        selected: selected.map(|i| i.to_string()).unwrap_or_default(),
        attrs: HxAttrs::new(),
    })
}

async fn form_page(st: &AppState, ctx: &Ctx, form: NoteForm) -> Result<oc_kit::ui::Page, OcError> {
    page_with(st, ctx, "", None, render(&form), "detail", false).await
}

async fn new_form(State(st): State<AppState>, ctx: Ctx) -> Result<Response, OcError> {
    let form = NoteForm {
        action: "/notes".into(),
        heading: "New note",
        title: String::new(),
        body: String::new(),
        tags: String::new(),
        case: case_select(&st, None).await?,
        notice: None,
        cancel: "/notes".into(),
    };
    Ok(form_page(&st, &ctx, form).await?.into_response())
}

async fn edit_form(State(st): State<AppState>, ctx: Ctx, Path(id): Path<i64>) -> Result<Response, OcError> {
    let n = notes::get(&st.pool, id).await?;
    let form = NoteForm {
        action: format!("/notes/{id}"),
        heading: "Edit note",
        title: n.title.clone(),
        body: n.body.clone(),
        tags: n.tags.join(", "),
        case: case_select(&st, n.case_id).await?,
        notice: None,
        cancel: format!("/notes/{id}"),
    };
    Ok(form_page(&st, &ctx, form).await?.into_response())
}

async fn rejected(
    st: &AppState,
    ctx: &Ctx,
    f: &NoteFormData,
    action: String,
    cancel: String,
    e: OcError,
) -> Result<Response, OcError> {
    let form = NoteForm {
        action,
        heading: "Fix the note",
        title: f.title.clone(),
        body: f.body.clone(),
        tags: f.tags.clone(),
        case: case_select(st, f.case_id.parse().ok()).await?,
        notice: Some(Notice::danger(e.to_string())),
        cancel,
    };
    Ok((StatusCode::UNPROCESSABLE_ENTITY, form_page(st, ctx, form).await?).into_response())
}

async fn create(State(st): State<AppState>, ctx: Ctx, Form(f): Form<NoteFormData>) -> Result<Response, OcError> {
    match notes::create(&st.pool, ctx.user.id, &f.input(), now()).await {
        Ok(id) => Ok(Redirect::to(&format!("/notes/{id}")).into_response()),
        Err(e @ OcError::Unprocessable(_)) => rejected(&st, &ctx, &f, "/notes".into(), "/notes".into(), e).await,
        Err(e) => Err(e),
    }
}

async fn update(
    State(st): State<AppState>,
    ctx: Ctx,
    Path(id): Path<i64>,
    Form(f): Form<NoteFormData>,
) -> Result<Response, OcError> {
    match notes::update(&st.pool, id, &f.input(), now()).await {
        Ok(()) => Ok(Redirect::to(&format!("/notes/{id}")).into_response()),
        Err(e @ OcError::Unprocessable(_)) => {
            rejected(&st, &ctx, &f, format!("/notes/{id}"), format!("/notes/{id}"), e).await
        }
        Err(e) => Err(e),
    }
}

async fn delete(State(st): State<AppState>, ctx: Ctx, Path(id): Path<i64>) -> Result<Response, OcError> {
    let note = notes::get(&st.pool, id).await?;
    NotesStore(st.pool.clone()).soft_delete(id, now()).await?;
    let notice = undo_notice(&format!("Deleted '{}'", note.title), HxAttrs::post(format!("/notes/{id}/restore")));
    let mut p = page(&st, &ctx, "", None, empty_detail(), "list").await?;
    p.notices = Html::from(notice);
    Ok(p.into_response())
}

async fn restore(State(st): State<AppState>, _ctx: Ctx, Path(id): Path<i64>) -> Result<Response, OcError> {
    NotesStore(st.pool.clone()).restore(id).await?;
    let mut h = HeaderMap::new();
    h.insert("hx-redirect", HeaderValue::from_str(&format!("/notes/{id}")).map_err(OcError::internal)?);
    Ok((h, StatusCode::OK).into_response())
}

async fn trash(State(st): State<AppState>, ctx: Ctx) -> Result<Response, OcError> {
    let deleted = notes::list(&st.pool, None, "", true).await?;
    let body =
        render(&Trash { notes: &deleted, empty: Empty::new("Trash is empty. Deleted notes stay here for 30 days.") });
    Ok(everyday(&ctx, "/notes", "Trash", body).into_response())
}

async fn search(State(st): State<AppState>, ctx: Ctx, Query(q): Query<ListQuery>) -> Result<Response, OcError> {
    let term = q.q.trim();
    let (found, matching) = if term.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let lower = term.to_lowercase();
        let all_cases = cases::list(&st.pool).await?;
        (
            notes::list(&st.pool, None, term, false).await?,
            all_cases.into_iter().filter(|c| c.name.to_lowercase().contains(&lower)).collect(),
        )
    };
    let search = SearchBox {
        name: "q".into(),
        value: term.to_owned(),
        label: "Search notes and cases".into(),
        attrs: HxAttrs::get("/search")
            .target("#oc-search-results")
            .swap("outerHTML")
            .with("hx-select", "#oc-search-results")
            .with("hx-trigger", "input changed delay:250ms from:find input, search")
            .with("hx-push-url", "true"),
    };
    let body = render(&SearchPage { search, q: term, notes: found, cases: matching });
    Ok(everyday(&ctx, "/search", "Search", body).into_response())
}
