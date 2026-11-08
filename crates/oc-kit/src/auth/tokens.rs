use axum::Form;
use axum::Router;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde::Deserialize;
use time::OffsetDateTime;

use super::AuthState;
use crate::error::OcError;
use crate::identity::extract::CurrentUser;
use crate::identity::tokens;

#[derive(Deserialize)]
struct NewToken {
    name: String,
}

pub(super) fn routes() -> Router<AuthState> {
    Router::new()
        .route("/tokens", post(mint))
        .route("/tokens/{id}/revoke", post(revoke))
        .route("/tokens/{id}/restore", post(restore))
}

async fn mint(
    State(st): State<AuthState>,
    CurrentUser(u): CurrentUser,
    Form(f): Form<NewToken>,
) -> Result<Response, OcError> {
    let (info, token) = tokens::mint(&st.pool, u.id, &f.name, st.token_prefix, OffsetDateTime::now_utc()).await?;
    Ok(super::fragments::token_minted(&info, &token))
}

async fn revoke(
    State(st): State<AuthState>,
    CurrentUser(u): CurrentUser,
    Path(id): Path<String>,
) -> Result<Response, OcError> {
    let info = tokens::get(&st.pool, u.id, &id).await?;
    tokens::revoke(&st.pool, u.id, &id, OffsetDateTime::now_utc()).await?;
    Ok(super::fragments::token_revoked(&info))
}

async fn restore(
    State(st): State<AuthState>,
    CurrentUser(u): CurrentUser,
    Path(id): Path<String>,
) -> Result<Response, OcError> {
    tokens::restore(&st.pool, u.id, &id).await?;
    let all = tokens::list(&st.pool, u.id).await?;
    Ok(super::fragments::token_list(&all).into_response())
}
