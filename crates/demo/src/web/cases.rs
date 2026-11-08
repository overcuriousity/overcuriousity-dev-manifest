use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use oc_kit::OcError;
use oc_kit::ui::{Empty, HxAttrs, TypedConfirm};
use serde::Deserialize;

use super::{Ctx, everyday, render};
use crate::domain::cases::{self, Case};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cases", get(index))
        .route("/cases/{id}/purge", post(purge))
}

#[derive(Template)]
#[template(path = "cases/list.html")]
struct CaseList {
    cases: Vec<Case>,
    empty: Empty,
}

impl CaseList {
    fn confirm(&self, c: &Case) -> TypedConfirm {
        TypedConfirm {
            id: format!("purge-{}", c.id),
            title: "Purge case".into(),
            text: "This deletes the case, its events and notes permanently.".into(),
            name_to_type: c.name.clone(),
            action_label: "Purge case".into(),
            post: HxAttrs::post(format!("/cases/{}/purge", c.id)),
        }
    }
}

async fn index(State(st): State<AppState>, ctx: Ctx) -> Result<Response, OcError> {
    let body = render(&CaseList {
        cases: cases::list(&st.pool).await?,
        empty: Empty::new("No cases yet. Cases appear here when data is ingested."),
    });
    Ok(everyday(&ctx, "/cases", "Cases", body).into_response())
}

#[derive(Deserialize)]
struct PurgeForm {
    #[serde(default)]
    name: String,
}

async fn purge(
    State(st): State<AppState>,
    _ctx: Ctx,
    Path(id): Path<i64>,
    Form(f): Form<PurgeForm>,
) -> Result<Response, OcError> {
    match cases::purge(&st.pool, id, &f.name).await {
        Ok(()) => {
            let mut h = HeaderMap::new();
            h.insert("hx-redirect", HeaderValue::from_static("/cases"));
            Ok((h, StatusCode::OK).into_response())
        }
        Err(e @ OcError::Unprocessable(_)) => {
            Ok((StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response())
        }
        Err(e) => Err(e),
    }
}
