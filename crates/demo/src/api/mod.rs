//! REST API under /api/v1. Every handler is registered through utoipa-axum, so the spec cannot drift from the routes.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use oc_kit::OcError;
use oc_kit::auth::CurrentUser;
use oc_kit::trash::SoftDelete;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::events::{self, Cursor, Event};
use crate::domain::notes::{self, Note, NoteInput, NotesStore};
use crate::domain::{cases, now, query};
use crate::state::AppState;

pub struct ApiError(OcError);

impl From<OcError> for ApiError {
    fn from(e: OcError) -> Self {
        ApiError(e)
    }
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let OcError::Internal(detail) = &self.0 {
            tracing::error!(%detail, "API error");
        }
        (
            self.0.status(),
            Json(ErrorBody {
                error: self.0.to_string(),
            }),
        )
            .into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(OpenApi)]
#[openapi(info(
    title = "Incident journal API",
    description = "Cases, events and notes. Authenticate with a session or `Authorization: Bearer <token>`."
))]
struct Doc;

pub fn router() -> axum::Router<AppState> {
    let (router, spec) = OpenApiRouter::with_openapi(Doc::openapi())
        .routes(routes!(list_cases))
        .routes(routes!(list_events))
        .routes(routes!(get_event))
        .routes(routes!(list_notes, create_note))
        .routes(routes!(get_note, update_note, delete_note))
        .split_for_parts();
    let mut spec = spec;
    spec.paths.paths = spec
        .paths
        .paths
        .into_iter()
        .map(|(k, v)| (format!("/api/v1{k}"), v))
        .collect();
    let json = serde_json::to_string(&spec).unwrap_or_default();
    axum::Router::new()
        .nest(
            "/api/v1",
            router.layer(oc_kit::security::rate_limit(600, 100)),
        )
        .route(
            "/api/v1/openapi.json",
            axum::routing::get(
                move || async move { ([("content-type", "application/json")], json) },
            ),
        )
}

/// List cases.
#[utoipa::path(get, path = "/cases", responses((status = 200, body = Vec<cases::Case>), (status = 401, body = ErrorBody)))]
async fn list_cases(State(st): State<AppState>, _u: CurrentUser) -> ApiResult<Vec<cases::Case>> {
    Ok(Json(cases::list(&st.pool).await?))
}

#[derive(Deserialize, IntoParams)]
struct EventsQuery {
    /// Search, e.g. `host:ws-01 -user:SYSTEM powershell`.
    #[serde(default)]
    q: String,
    /// `next_cursor` from the previous page.
    #[serde(default)]
    cursor: String,
    /// 1 to 500, default 100.
    limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct EventPage {
    pub items: Vec<Event>,
    pub next_cursor: Option<String>,
}

/// Search a case's events, newest first, with cursor pagination.
#[utoipa::path(get, path = "/cases/{id}/events", params(("id" = i64, Path), EventsQuery),
    responses((status = 200, body = EventPage), (status = 400, body = ErrorBody)))]
async fn list_events(
    State(st): State<AppState>,
    _u: CurrentUser,
    Path(id): Path<i64>,
    Query(eq): Query<EventsQuery>,
) -> ApiResult<EventPage> {
    cases::get(&st.pool, id).await?;
    let q = query::parse(&eq.q).map_err(|e| OcError::BadRequest(e.to_string()))?;
    let cursor = (!eq.cursor.is_empty())
        .then(|| Cursor::decode(&eq.cursor))
        .transpose()?;
    let page = events::search(
        &st.pool,
        id,
        &q,
        cursor,
        eq.limit.unwrap_or(100).clamp(1, 500),
    )
    .await?;
    Ok(Json(EventPage {
        items: page.items,
        next_cursor: page.next_cursor,
    }))
}

/// Get one event.
#[utoipa::path(get, path = "/events/{id}", params(("id" = i64, Path)), responses((status = 200, body = Event), (status = 404, body = ErrorBody)))]
async fn get_event(
    State(st): State<AppState>,
    _u: CurrentUser,
    Path(id): Path<i64>,
) -> ApiResult<Event> {
    Ok(Json(events::get(&st.pool, id).await?))
}

#[derive(Deserialize, IntoParams)]
struct NotesQuery {
    case_id: Option<i64>,
}

/// List notes, optionally for one case.
#[utoipa::path(get, path = "/notes", params(NotesQuery), responses((status = 200, body = Vec<Note>)))]
async fn list_notes(
    State(st): State<AppState>,
    _u: CurrentUser,
    Query(nq): Query<NotesQuery>,
) -> ApiResult<Vec<Note>> {
    Ok(Json(notes::list(&st.pool, nq.case_id, "", false).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct NoteBody {
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub case_id: Option<i64>,
}

impl NoteBody {
    fn input(self) -> NoteInput {
        NoteInput {
            case_id: self.case_id,
            title: self.title,
            body: self.body,
            tags: notes::parse_tags(&self.tags.join(",")),
        }
    }
}

/// Create a note.
#[utoipa::path(post, path = "/notes", request_body = NoteBody, responses((status = 201, body = Note), (status = 422, body = ErrorBody)))]
async fn create_note(
    State(st): State<AppState>,
    CurrentUser(u): CurrentUser,
    Json(b): Json<NoteBody>,
) -> Result<(StatusCode, Json<Note>), ApiError> {
    let id = notes::create(&st.pool, u.id, &b.input(), now()).await?;
    Ok((StatusCode::CREATED, Json(notes::get(&st.pool, id).await?)))
}

/// Get one note.
#[utoipa::path(get, path = "/notes/{id}", params(("id" = i64, Path)), responses((status = 200, body = Note), (status = 404, body = ErrorBody)))]
async fn get_note(
    State(st): State<AppState>,
    _u: CurrentUser,
    Path(id): Path<i64>,
) -> ApiResult<Note> {
    let n = notes::get(&st.pool, id).await?;
    if n.deleted_at.is_some() {
        return Err(OcError::NotFound.into());
    }
    Ok(Json(n))
}

/// Replace a note's title, body, tags and case.
#[utoipa::path(patch, path = "/notes/{id}", params(("id" = i64, Path)), request_body = NoteBody, responses((status = 200, body = Note)))]
async fn update_note(
    State(st): State<AppState>,
    _u: CurrentUser,
    Path(id): Path<i64>,
    Json(b): Json<NoteBody>,
) -> ApiResult<Note> {
    notes::update(&st.pool, id, &b.input(), now()).await?;
    Ok(Json(notes::get(&st.pool, id).await?))
}

/// Move a note to Trash; it can be restored for 30 days.
#[utoipa::path(delete, path = "/notes/{id}", params(("id" = i64, Path)), responses((status = 204)))]
async fn delete_note(
    State(st): State<AppState>,
    _u: CurrentUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    NotesStore(st.pool.clone()).soft_delete(id, now()).await?;
    Ok(StatusCode::NO_CONTENT)
}
