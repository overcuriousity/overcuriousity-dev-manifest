use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use oc_kit::prefs::{Prefs, PrefsSource};
use oc_kit::ui::{Html, Page, Profile, TopBar};

use crate::state::BRANDING;

fn page(status: StatusCode, message: &str, reference: Option<&str>) -> Response {
    let reference = reference
        .map(|r| format!("<p class=\"oc-secondary\">Reference <code class=\"oc-mono\">{r}</code></p>"))
        .unwrap_or_default();
    let body = Html::raw(format!(
        "<div class=\"oc-columns oc-empty\"><p class=\"oc-text-section\">{}</p>{reference}<a href=\"/notes\">Back to notes</a></div>",
        askama::filters::escape(message, askama::filters::Html).map(|s| s.to_string()).unwrap_or_default()
    ));
    let top = TopBar::new(BRANDING, "/notes", Prefs::default(), None);
    (status, Page::new(message, Profile::Everyday, top, body, PrefsSource::Local)).into_response()
}

pub async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "Page not found.").into_response()
}

/// Turns plain-text errors into the shared error page for browser navigations; htmx and API clients keep the one-line text.
pub async fn html_errors(req: Request, next: Next) -> Response {
    let wants_html =
        req.headers().get(header::ACCEPT).and_then(|v| v.to_str().ok()).is_some_and(|a| a.contains("text/html"))
            && !req.headers().contains_key("hx-request");
    let path = req.uri().path().to_owned();
    let res = next.run(req).await;
    let status = res.status();
    let plain = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|c| c.starts_with("text/plain"));
    if !wants_html || !plain || !(status == StatusCode::NOT_FOUND || status.is_server_error()) {
        return res;
    }
    if status.is_server_error() {
        let reference = oc_kit::identity::password::generate(10);
        tracing::error!(%reference, %path, %status, "request failed");
        return page(status, "Something went wrong.", Some(&reference));
    }
    page(status, "Page not found.", None)
}
