use axum::extract::FromRef;
use oc_kit::assets::AssetSet;
use oc_kit::auth::{AuthState, Branding};
use oc_kit::sse::Broadcaster;
use serde::Deserialize;
use sqlx::SqlitePool;

pub const BRANDING: Branding = Branding {
    app_name: "Incident journal",
    logo_svg: include_str!("../assets/logo.svg"),
};
pub const APP: &str = "demo";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
    pub ingest: Ingest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Ingest {
    pub enabled: bool,
    pub per_second: f64,
}

impl Default for Ingest {
    fn default() -> Self {
        Self {
            enabled: true,
            per_second: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IngestEvent {
    pub case_id: i64,
    pub event_id: i64,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub auth: AuthState,
    pub ingest: Broadcaster<IngestEvent>,
    pub assets: &'static AssetSet,
}

impl FromRef<AppState> for AuthState {
    fn from_ref(s: &AppState) -> AuthState {
        s.auth.clone()
    }
}
