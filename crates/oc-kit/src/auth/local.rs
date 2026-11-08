use axum::Form;
use axum::Router;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use tower_sessions::Session;

use super::{AuthState, safe_next};
use crate::error::OcError;
use crate::identity::users;
use crate::security::rate_limit;
use crate::session::USER_KEY;

#[derive(Deserialize)]
pub(super) struct SignInForm {
    username: String,
    password: String,
    #[serde(default)]
    next: String,
}

#[derive(Deserialize)]
pub(super) struct NextQuery {
    #[serde(default)]
    next: String,
}

pub(super) fn routes() -> Router<AuthState> {
    Router::new()
        .route(
            "/sign-in",
            get(sign_in_page).merge(post(sign_in).layer(rate_limit(10, 5))),
        )
        .route("/sign-out", post(sign_out))
}

async fn sign_in_page(State(st): State<AuthState>, Query(q): Query<NextQuery>) -> Response {
    super::render_sign_in(&st, safe_next(&q.next), None)
}

async fn sign_in(
    State(st): State<AuthState>,
    session: Session,
    Form(f): Form<SignInForm>,
) -> Result<Response, OcError> {
    if !st.cfg.local {
        return Err(OcError::NotFound);
    }
    let Some(user) = users::verify_password(&st.pool, f.username.trim(), &f.password).await? else {
        return Ok(super::render_sign_in(
            &st,
            safe_next(&f.next),
            Some("Wrong username or password."),
        ));
    };
    session.cycle_id().await.map_err(OcError::internal)?;
    session
        .insert(USER_KEY, user.id)
        .await
        .map_err(OcError::internal)?;
    Ok(Redirect::to(safe_next(&f.next)).into_response())
}

async fn sign_out(session: Session) -> Result<Response, OcError> {
    session.flush().await.map_err(OcError::internal)?;
    Ok(Redirect::to("/auth/sign-in").into_response())
}
