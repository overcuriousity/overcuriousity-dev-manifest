use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::Ctx;
use super::client_ip::{ClientIp, resolve, trusted_peer};

pub(super) async fn host_check(State(ctx): State<Arc<Ctx>>, req: Request, next: Next) -> Response {
    let (mut parts, body) = req.into_parts();
    let forwarded = trusted_peer(&parts, &ctx.proxies)
        .then(|| {
            parts
                .headers
                .get("x-forwarded-host")
                .and_then(|v| v.to_str().ok())
        })
        .flatten();
    let host = forwarded
        .or_else(|| {
            parts
                .headers
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
        })
        .or_else(|| parts.uri.authority().map(|a| a.as_str()));
    if !host.is_some_and(|h| ctx.dep.host_allowed(h)) {
        return (
            StatusCode::MISDIRECTED_REQUEST,
            "This server does not answer for that host name.",
        )
            .into_response();
    }
    if let Some(ip) = resolve(&parts, &ctx.proxies) {
        parts.extensions.insert(ClientIp(ip));
    }
    next.run(Request::from_parts(parts, body)).await
}

/// Cookie-authenticated writes must come from our own origin. Bearer requests from non-browser
/// clients carry neither `Origin` nor `Sec-Fetch-Site` and cannot be forged cross-site.
pub(super) async fn origin_check(
    State(ctx): State<Arc<Ctx>>,
    req: Request,
    next: Next,
) -> Response {
    if matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        return next.run(req).await;
    }
    let h = req.headers();
    let allowed = if let Some(origin) = h.get(header::ORIGIN) {
        origin.to_str().is_ok_and(|o| ctx.dep.origin_allowed(o))
    } else if let Some(site) = h.get("sec-fetch-site") {
        matches!(site.to_str(), Ok("same-origin" | "none"))
    } else {
        let bearer = h
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("Bearer "));
        bearer && !h.contains_key(header::COOKIE)
    };
    if allowed {
        next.run(req).await
    } else {
        (
            StatusCode::FORBIDDEN,
            "This request came from another site and was blocked.",
        )
            .into_response()
    }
}
