use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Default,
    Compact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Prefs {
    pub theme: Theme,
    pub density: Density,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PrefsSource {
    User,
    #[default]
    Local,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "system" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }
}

impl Density {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Compact => "compact",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "default" => Some(Self::Default),
            "compact" => Some(Self::Compact),
            _ => None,
        }
    }
}

impl PrefsSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Local => "local",
        }
    }
}

use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::routing::post;
use axum::{Form, Router};
use sqlx::SqlitePool;

use crate::auth::AuthState;
use crate::error::OcError;
use crate::identity::extract::{CurrentUser, resolve_user};

pub async fn load(pool: &SqlitePool, user_id: i64) -> Result<Prefs, OcError> {
    let row = sqlx::query!("SELECT theme, density FROM oc_preferences WHERE user_id = ?", user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row
        .map(|r| Prefs {
            theme: Theme::parse(&r.theme).unwrap_or_default(),
            density: Density::parse(&r.density).unwrap_or_default(),
        })
        .unwrap_or_default())
}

pub async fn save(pool: &SqlitePool, user_id: i64, p: Prefs) -> Result<(), OcError> {
    let (theme, density) = (p.theme.as_str(), p.density.as_str());
    sqlx::query!(
        "INSERT INTO oc_preferences (user_id, theme, density) VALUES (?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET theme = excluded.theme, density = excluded.density",
        user_id,
        theme,
        density
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Deserialize)]
struct PrefsForm {
    theme: Option<String>,
    density: Option<String>,
}

/// Mount at `/_oc/prefs`.
pub fn prefs_router() -> Router<AuthState> {
    Router::new().route("/", post(update))
}

async fn update(
    State(st): State<AuthState>,
    CurrentUser(u): CurrentUser,
    Form(f): Form<PrefsForm>,
) -> Result<StatusCode, OcError> {
    let mut p = load(&st.pool, u.id).await?;
    if let Some(t) = f.theme {
        p.theme = Theme::parse(&t).ok_or_else(|| OcError::Unprocessable("Unknown theme.".into()))?;
    }
    if let Some(d) = f.density {
        p.density = Density::parse(&d).ok_or_else(|| OcError::Unprocessable("Unknown density.".into()))?;
    }
    save(&st.pool, u.id, p).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The preferences to render a page with: the user's when signed in, defaults (and `localStorage`) otherwise.
pub struct PagePrefs(pub Prefs, pub PrefsSource);

impl<S> FromRequestParts<S> for PagePrefs
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = OcError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let st = AuthState::from_ref(state);
        let user = match parts.extensions.get::<CurrentUser>() {
            Some(u) => Some(u.0.clone()),
            None => resolve_user(parts, &st).await?,
        };
        match user {
            Some(u) => Ok(PagePrefs(load(&st.pool, u.id).await?, PrefsSource::User)),
            None => Ok(PagePrefs(Prefs::default(), PrefsSource::Local)),
        }
    }
}
