use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;
use tower_sessions::Session;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use super::{AuthState, safe_next};
use crate::error::OcError;
use crate::identity::extract::CurrentUser;
use crate::identity::users;
use crate::security::rate_limit;
use crate::session::USER_KEY;
use crate::ui::{Button, Html, HxAttrs, Notice, PasskeyItem, PasskeyList, date};

const REG_KEY: &str = "oc.passkey.reg";
const AUTH_KEY: &str = "oc.passkey.auth";

pub(crate) fn build(
    st: &crate::config::Deployment,
    app_name: &str,
) -> Result<Arc<Webauthn>, OcError> {
    let w = WebauthnBuilder::new(&st.rp_id, &st.public_url)
        .map_err(OcError::internal)?
        .rp_name(app_name)
        .build()
        .map_err(OcError::internal)?;
    Ok(Arc::new(w))
}

pub(super) fn routes() -> Router<AuthState> {
    Router::new()
        .route("/passkeys", get(list))
        .route("/passkeys/register/start", post(register_start))
        .route("/passkeys/register/finish", post(register_finish))
        .route(
            "/passkeys/login/start",
            post(login_start).layer(rate_limit(30, 10)),
        )
        .route(
            "/passkeys/login/finish",
            post(login_finish).layer(rate_limit(30, 10)),
        )
        .route("/passkeys/{id}/delete", post(delete))
        .route("/passkeys/{id}/restore", post(restore))
}

fn user_handle(id: i64) -> Uuid {
    Uuid::from_u64_pair(0, id.unsigned_abs())
}

fn webauthn(st: &AuthState) -> Result<&Webauthn, OcError> {
    st.webauthn.as_deref().ok_or(OcError::NotFound)
}

fn bad(e: impl std::fmt::Display) -> OcError {
    tracing::info!(error = %e, "passkey ceremony rejected");
    OcError::BadRequest("The passkey could not be verified. Try again.".into())
}

async fn stored(pool: &SqlitePool, user_id: i64) -> Result<Vec<(i64, Passkey)>, OcError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", credential FROM oc_passkeys WHERE user_id = ? AND removed_at IS NULL"#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| serde_json::from_str(&r.credential).ok().map(|p| (r.id, p)))
        .collect())
}

pub async fn list_items(pool: &SqlitePool, user_id: i64) -> Result<Vec<PasskeyItem>, OcError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", name, last_used_at AS "last_used_at: OffsetDateTime" FROM oc_passkeys
           WHERE user_id = ? AND removed_at IS NULL ORDER BY created_at"#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| PasskeyItem {
            id: r.id,
            name: r.name,
            last_used: r.last_used_at.map(date),
        })
        .collect())
}

async fn list_fragment(st: &AuthState, user_id: i64) -> Result<Html, OcError> {
    Ok(Html::from(PasskeyList {
        passkeys: list_items(&st.pool, user_id).await?,
        enabled: st.webauthn.is_some(),
    }))
}

async fn list(State(st): State<AuthState>, CurrentUser(u): CurrentUser) -> Result<Html, OcError> {
    webauthn(&st)?;
    list_fragment(&st, u.id).await
}

async fn register_start(
    State(st): State<AuthState>,
    session: Session,
    CurrentUser(u): CurrentUser,
) -> Result<Response, OcError> {
    let w = webauthn(&st)?;
    let exclude = stored(&st.pool, u.id)
        .await?
        .into_iter()
        .map(|(_, p)| p.cred_id().clone())
        .collect();
    let (challenge, state) = w
        .start_passkey_registration(
            user_handle(u.id),
            &u.username,
            &u.display_name,
            Some(exclude),
        )
        .map_err(OcError::internal)?;
    session
        .insert(REG_KEY, state)
        .await
        .map_err(OcError::internal)?;
    Ok(Json(challenge).into_response())
}

#[derive(Deserialize)]
struct NameQuery {
    #[serde(default)]
    name: String,
}

async fn register_finish(
    State(st): State<AuthState>,
    session: Session,
    CurrentUser(u): CurrentUser,
    Query(q): Query<NameQuery>,
    Json(cred): Json<RegisterPublicKeyCredential>,
) -> Result<Html, OcError> {
    let w = webauthn(&st)?;
    let state: PasskeyRegistration = session
        .remove(REG_KEY)
        .await
        .map_err(OcError::internal)?
        .ok_or_else(|| bad("no ceremony"))?;
    let passkey = w.finish_passkey_registration(&cred, &state).map_err(bad)?;
    let json = serde_json::to_string(&passkey).map_err(OcError::internal)?;
    let name = if q.name.trim().is_empty() {
        "Passkey".to_owned()
    } else {
        q.name.trim().chars().take(80).collect()
    };
    let now = OffsetDateTime::now_utc();
    sqlx::query!(
        "INSERT INTO oc_passkeys (user_id, name, credential, created_at) VALUES (?, ?, ?, ?)",
        u.id,
        name,
        json,
        now
    )
    .execute(&st.pool)
    .await?;
    list_fragment(&st, u.id).await
}

async fn login_start(State(st): State<AuthState>, session: Session) -> Result<Response, OcError> {
    let w = webauthn(&st)?;
    let (challenge, state) = w
        .start_discoverable_authentication()
        .map_err(OcError::internal)?;
    session
        .insert(AUTH_KEY, state)
        .await
        .map_err(OcError::internal)?;
    Ok(Json(challenge).into_response())
}

#[derive(Deserialize)]
struct NextQuery {
    #[serde(default)]
    next: String,
}

async fn login_finish(
    State(st): State<AuthState>,
    session: Session,
    Query(q): Query<NextQuery>,
    Json(cred): Json<PublicKeyCredential>,
) -> Result<Response, OcError> {
    let w = webauthn(&st)?;
    let state: DiscoverableAuthentication = session
        .remove(AUTH_KEY)
        .await
        .map_err(OcError::internal)?
        .ok_or_else(|| bad("no ceremony"))?;
    let (handle, cred_id) = w.identify_discoverable_authentication(&cred).map_err(bad)?;
    let user_id = i64::try_from(handle.as_u64_pair().1).map_err(bad)?;
    let user = users::by_id(&st.pool, user_id)
        .await?
        .ok_or_else(|| bad("unknown user"))?;
    let (id, mut passkey) = stored(&st.pool, user.id)
        .await?
        .into_iter()
        .find(|(_, p)| AsRef::<[u8]>::as_ref(p.cred_id()) == cred_id)
        .ok_or_else(|| bad("unknown credential"))?;
    let result = w
        .finish_discoverable_authentication_v2(&cred, state, &passkey)
        .map_err(bad)?;
    passkey.update_credential(&result);
    let json = serde_json::to_string(&passkey).map_err(OcError::internal)?;
    let now = OffsetDateTime::now_utc();
    sqlx::query!(
        "UPDATE oc_passkeys SET credential = ?, last_used_at = ? WHERE id = ?",
        json,
        now,
        id
    )
    .execute(&st.pool)
    .await?;
    session.cycle_id().await.map_err(OcError::internal)?;
    session
        .insert(USER_KEY, user.id)
        .await
        .map_err(OcError::internal)?;
    Ok(Json(serde_json::json!({ "next": safe_next(&q.next) })).into_response())
}

async fn delete(
    State(st): State<AuthState>,
    CurrentUser(u): CurrentUser,
    Path(id): Path<i64>,
) -> Result<Html, OcError> {
    let now = OffsetDateTime::now_utc();
    let hour_ago = now - time::Duration::hours(1);
    sqlx::query!(
        "DELETE FROM oc_passkeys WHERE removed_at IS NOT NULL AND removed_at < ?",
        hour_ago
    )
    .execute(&st.pool)
    .await?;
    let name = sqlx::query_scalar!(
        "SELECT name FROM oc_passkeys WHERE id = ? AND user_id = ? AND removed_at IS NULL",
        id,
        u.id
    )
    .fetch_optional(&st.pool)
    .await?
    .ok_or(OcError::NotFound)?;
    sqlx::query!(
        "UPDATE oc_passkeys SET removed_at = ? WHERE id = ?",
        now,
        id
    )
    .execute(&st.pool)
    .await?;
    let undo = Button::default("Undo").attrs(
        HxAttrs::post(format!("/auth/passkeys/{id}/restore"))
            .target("#oc-passkey-list")
            .swap("outerHTML"),
    );
    Ok(Notice::success(format!("Removed passkey '{name}'"))
        .action(undo)
        .autohide(crate::trash::UNDO_MS)
        .oob())
}

async fn restore(
    State(st): State<AuthState>,
    CurrentUser(u): CurrentUser,
    Path(id): Path<i64>,
) -> Result<Html, OcError> {
    sqlx::query!(
        "UPDATE oc_passkeys SET removed_at = NULL WHERE id = ? AND user_id = ?",
        id,
        u.id
    )
    .execute(&st.pool)
    .await?;
    list_fragment(&st, u.id).await
}
