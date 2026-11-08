use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use tower_sessions::Session;

use super::User;
use super::users;
use crate::auth::AuthState;
use crate::error::OcError;
use crate::session::USER_KEY;

#[derive(Debug, Clone)]
pub struct CurrentUser(pub User);

#[derive(Debug, Clone)]
pub struct MaybeUser(pub Option<User>);

#[derive(Debug, Clone)]
pub struct AdminUser(pub User);

pub enum AuthRejection {
    SignIn { next: String },
    Unauthorized { bearer_hint: Option<String> },
    Forbidden,
    Error(OcError),
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        match self {
            Self::SignIn { next } => Redirect::to(&format!(
                "/auth/sign-in?next={}",
                urlencoding::encode(&next)
            ))
            .into_response(),
            Self::Unauthorized { bearer_hint } => {
                let www = bearer_hint.unwrap_or_else(|| "Bearer".into());
                (
                    StatusCode::UNAUTHORIZED,
                    [
                        (header::WWW_AUTHENTICATE, www),
                        (header::CONTENT_TYPE, "application/json".into()),
                    ],
                    r#"{"error":"Sign in first."}"#,
                )
                    .into_response()
            }
            Self::Forbidden => OcError::Forbidden.into_response(),
            Self::Error(e) => e.into_response(),
        }
    }
}

fn wants_html(parts: &Parts) -> bool {
    parts
        .headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|a| a.contains("text/html"))
        && !parts.headers.contains_key("hx-request")
}

fn bearer(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

/// Session first, then `Authorization: Bearer`.
pub async fn resolve_user(parts: &mut Parts, st: &AuthState) -> Result<Option<User>, OcError> {
    if let Some(token) = bearer(parts).map(str::to_owned) {
        return crate::identity::verify_bearer(st, &token).await;
    }
    let Ok(session) = Session::from_request_parts(parts, st).await else {
        return Ok(None);
    };
    let id: Option<i64> = session.get(USER_KEY).await.map_err(OcError::internal)?;
    match id {
        Some(id) => users::by_id(&st.pool, id).await,
        None => Ok(None),
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let st = AuthState::from_ref(state);
        if let Some(u) = parts.extensions.get::<CurrentUser>() {
            return Ok(u.clone());
        }
        match resolve_user(parts, &st)
            .await
            .map_err(AuthRejection::Error)?
        {
            Some(u) => {
                parts.extensions.insert(CurrentUser(u.clone()));
                Ok(CurrentUser(u))
            }
            None if wants_html(parts) => Err(AuthRejection::SignIn {
                next: parts
                    .uri
                    .path_and_query()
                    .map(|p| p.as_str().to_owned())
                    .unwrap_or_else(|| "/".into()),
            }),
            None => Err(AuthRejection::Unauthorized {
                bearer_hint: crate::identity::www_authenticate(&st),
            }),
        }
    }
}

impl<S> FromRequestParts<S> for MaybeUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let st = AuthState::from_ref(state);
        Ok(MaybeUser(
            resolve_user(parts, &st)
                .await
                .map_err(AuthRejection::Error)?,
        ))
    }
}

impl<S> FromRequestParts<S> for AdminUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let CurrentUser(u) = CurrentUser::from_request_parts(parts, state).await?;
        if u.is_admin {
            Ok(AdminUser(u))
        } else {
            Err(AuthRejection::Forbidden)
        }
    }
}
