mod fragments;
mod local;
mod tokens;

use axum::Router;
use sqlx::SqlitePool;

use crate::config::{AuthConfig, Deployment};
use crate::error::OcError;

pub use crate::identity::extract::{AdminUser, CurrentUser, MaybeUser};

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
        Ok(Self {
            pool,
            dep,
            cfg,
            app,
            branding,
            token_prefix,
        })
    }
}

/// Only same-origin absolute paths are followed after sign-in.
pub fn safe_next(next: &str) -> &str {
    let ok = next.starts_with('/')
        && !next.starts_with("//")
        && !next.starts_with("/\\")
        && !next.contains(['\r', '\n']);
    if ok { next } else { "/" }
}

/// Mount at `/auth`.
pub fn router(state: AuthState) -> Router<AuthState> {
    let _ = &state;
    Router::new().merge(local::routes()).merge(tokens::routes())
}

pub(crate) fn render_sign_in(
    st: &AuthState,
    next: &str,
    notice: Option<&str>,
) -> axum::response::Response {
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
    Page::new(
        "Sign in",
        Profile::Everyday,
        top,
        Html::from(form),
        crate::prefs::PrefsSource::Local,
    )
    .into_response()
}
