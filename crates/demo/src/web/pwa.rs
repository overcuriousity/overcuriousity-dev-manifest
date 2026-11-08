use axum::Router;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use oc_kit::assets::AssetSet;
use oc_kit::prefs::{Prefs, PrefsSource};
use oc_kit::ui::{Html, Page, Profile, TopBar};

use crate::app::assets;
use crate::state::{AppState, BRANDING};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/manifest.webmanifest", get(manifest))
        .route("/sw.js", get(service_worker))
        .route("/offline", get(offline))
}

pub fn head() -> Html {
    Html(format!(
        "<link rel=\"manifest\" href=\"/manifest.webmanifest\">\n<link rel=\"icon\" href=\"{}\" type=\"image/svg+xml\">",
        assets().url("icon.svg")
    ))
}

pub fn script() -> String {
    assets().url("pwa.js")
}

async fn manifest() -> Response {
    let body = serde_json::json!({
        "name": "Incident journal",
        "short_name": "Journal",
        "start_url": "/notes",
        "scope": "/",
        "display": "standalone",
        "icons": [{ "src": assets().url("icon.svg"), "sizes": "any", "type": "image/svg+xml" }],
    });
    ([(header::CONTENT_TYPE, "application/manifest+json")], body.to_string()).into_response()
}

fn shell_urls() -> Vec<String> {
    let kit = AssetSet::kit();
    let mut urls: Vec<String> =
        ["css/oc.css", "css/profile-everyday.css", "js/theme.js", "vendor/htmx.min.js", "vendor/htmx-ext-sse.js"]
            .iter()
            .map(|p| kit.url(p))
            .collect();
    urls.extend(["fonts/atkinson-next-400.woff2", "fonts/atkinson-next-600.woff2"].iter().map(|p| kit.url(p)));
    urls.extend(oc_kit::ui::KIT_MODULES.iter().map(|m| kit.url(&format!("js/{m}.js"))));
    urls.push(assets().url("icon.svg"));
    urls.push("/offline".into());
    urls
}

async fn service_worker() -> Response {
    let template = String::from_utf8_lossy(&assets().get("sw.js").unwrap_or_default()).into_owned();
    let body = template
        .replace("__VERSION__", &format!("journal-{}", AssetSet::kit().hash()))
        .replace("__SHELL__", &serde_json::to_string(&shell_urls()).unwrap_or_else(|_| "[]".into()));
    ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")], body)
        .into_response()
}

async fn offline() -> Response {
    let body = Html(
        "<div class=\"oc-columns oc-empty\"><p class=\"oc-text-section\">You are offline.</p><p>The journal needs the server to show notes. Reconnect and reload.</p></div>"
            .into(),
    );
    let top = TopBar::new(BRANDING, "/notes", Prefs::default(), None);
    Page::new("Offline", Profile::Everyday, top, body, PrefsSource::Local).into_response()
}
