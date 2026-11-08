mod fragments;
mod local;
pub mod oidc;
pub mod passkeys;
mod tokens;

use axum::Router;
use sqlx::SqlitePool;

use crate::config::{AuthConfig, Deployment};
use crate::error::OcError;

pub use crate::identity::extract::{AdminUser, CurrentUser, MaybeUser};
pub use oidc::resource_metadata_router;

#[derive(Debug, Clone, Copy)]
pub struct Branding {
    pub app_name: &'static str,
    pub logo_svg: &'static str,
}

#[derive(Clone)]
pub struct AuthState {
    pub pool: SqlitePool,
    pub dep: Deployment,
    pub cfg: AuthConfig,
    pub app: &'static str,
    pub branding: Branding,
    pub token_prefix: &'static str,
    pub webauthn: Option<std::sync::Arc<webauthn_rs::Webauthn>>,
    pub oidc: Option<std::sync::Arc<oidc::Oidc>>,
}

impl AuthState {
    pub async fn new(
        pool: SqlitePool,
        dep: Deployment,
        cfg: AuthConfig,
        app: &'static str,
        branding: Branding,
        token_prefix: &'static str,
    ) -> Result<Self, OcError> {
        let webauthn = if cfg.passkeys { Some(passkeys::build(&dep, branding.app_name)?) } else { None };
        let oidc = cfg.oidc.as_ref().map(|o| oidc::Oidc::new(o, &dep)).transpose()?;
        Ok(Self { pool, dep, cfg, app, branding, token_prefix, webauthn, oidc })
    }
}

/// Only same-origin absolute paths are followed after sign-in.
/// Browsers drop tabs and newlines and read `\` as `/` while parsing URLs, so any of them could turn
/// a local path into `//other.host`.
pub fn safe_next(next: &str) -> &str {
    let ok =
        next.starts_with('/') && !next.starts_with("//") && !next.bytes().any(|b| b <= 0x20 || b == 0x7f || b == b'\\');
    if ok { next } else { "/" }
}

/// Mount at `/auth`.
pub fn router(state: AuthState) -> Router<AuthState> {
    let mut r = Router::new().merge(local::routes()).merge(tokens::routes());
    if state.oidc.is_some() {
        r = r.merge(oidc::routes());
    }
    if state.webauthn.is_some() { r.merge(passkeys::routes()) } else { r }
}

pub(crate) fn render_sign_in(st: &AuthState, next: &str, notice: Option<&str>) -> axum::response::Response {
    use crate::ui::{Html, Notice, Page, Profile, SignInForm, TopBar};
    use axum::response::IntoResponse;
    let form = SignInForm {
        local: st.cfg.local,
        passkeys: st.cfg.passkeys,
        oidc_label: st.cfg.oidc.as_ref().map(|o| o.display_name.clone()),
        notice: notice.map(Notice::danger),
        next: next.to_owned(),
    };
    let top = TopBar::new(st.branding, "/", crate::prefs::Prefs::default(), None);
    Page::new("Sign in", Profile::Everyday, top, Html::from(form), crate::prefs::PrefsSource::Local).into_response()
}
