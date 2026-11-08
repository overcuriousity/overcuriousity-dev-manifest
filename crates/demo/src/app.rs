use axum::Router;
use axum::extract::State;
use axum::response::Redirect;
use axum::routing::get;
use oc_kit::OcError;
use oc_kit::assets::{app_assets, asset_router};
use oc_kit::auth::{AuthState, resource_metadata_router};
use oc_kit::config::OcConfig;
use oc_kit::security::{Limits, secure};
use oc_kit::sse::Broadcaster;
use rust_embed::RustEmbed;

use crate::state::{APP, AppState, BRANDING};
use crate::web;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct DemoFiles;

pub async fn build(cfg: &OcConfig, pool: sqlx::SqlitePool) -> Result<(Router, AppState), OcError> {
    let dep = cfg.deployment()?;
    let auth = AuthState::new(
        pool.clone(),
        dep.clone(),
        cfg.auth.clone(),
        APP,
        BRANDING,
        APP,
    )
    .await?;
    let (assets, assets_router) = (self::assets(), assets_router());
    let st = AppState {
        pool: pool.clone(),
        auth: auth.clone(),
        ingest: Broadcaster::new(256),
        assets,
    };
    let sessions = oc_kit::session::session_layer(&pool, APP, &dep).await?;
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/", get(|| async { Redirect::to("/notes") }))
        .merge(web::routes())
        .nest(
            "/auth",
            oc_kit::auth::router(auth.clone()).with_state(auth.clone()),
        )
        .nest(
            "/_oc/prefs",
            oc_kit::prefs::prefs_router().with_state(auth.clone()),
        )
        .merge(resource_metadata_router().with_state(auth))
        .with_state(st.clone())
        .fallback(web::errors::not_found)
        .layer(axum::middleware::from_fn(web::errors::html_errors))
        .layer(sessions)
        .merge(asset_router())
        .merge(assets_router);
    let limits = Limits {
        body_bytes: 1024 * 1024,
        ..Limits::default()
    };
    Ok((secure(app, &dep, &cfg.trusted_proxies, limits), st))
}

async fn health(State(st): State<AppState>) -> Result<&'static str, OcError> {
    sqlx::query("SELECT 1").execute(&st.pool).await?;
    Ok("ok")
}

static ASSETS: std::sync::OnceLock<(&'static oc_kit::assets::AssetSet, Router)> =
    std::sync::OnceLock::new();

fn init() -> &'static (&'static oc_kit::assets::AssetSet, Router) {
    ASSETS.get_or_init(|| app_assets::<DemoFiles>("_demo"))
}

pub fn assets() -> &'static oc_kit::assets::AssetSet {
    init().0
}

fn assets_router() -> Router {
    init().1.clone()
}
