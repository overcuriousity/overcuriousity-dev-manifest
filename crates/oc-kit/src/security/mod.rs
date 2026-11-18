//! The middleware every oc app runs behind. Each piece is a small, independent function.

mod client_ip;
mod headers;
mod limits;
mod origin;
mod rate;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::http::header;
use axum::middleware::from_fn_with_state;
use ipnet::IpNet;
use tower_http::compression::CompressionLayer;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;

use crate::config::Deployment;

pub use client_ip::ClientIp;
pub use headers::CSP;
pub use rate::{Buckets, rate_limit};

#[derive(Debug, Clone)]
pub struct Limits {
    pub body_bytes: usize,
    pub timeout: Duration,
    /// Matched by path prefix on whole segments, longest wins. An override's own `overrides` are ignored.
    pub overrides: Vec<(&'static str, Limits)>,
}

impl Default for Limits {
    fn default() -> Self {
        Self { body_bytes: 1024 * 1024, timeout: Duration::from_secs(30), overrides: Vec::new() }
    }
}

pub(crate) struct Ctx {
    pub dep: Deployment,
    pub proxies: Vec<IpNet>,
}

/// Wraps a router in the full security stack. Apply it last, around everything the app serves.
/// The timeout bounds producing response headers, so long-lived SSE bodies are not cut off.
pub fn secure(router: Router, dep: &Deployment, proxies: &[IpNet], limits: Limits) -> Router {
    let ctx = Arc::new(Ctx { dep: dep.clone(), proxies: proxies.to_vec() });
    let limits = Arc::new(limits);
    router
        .layer(CompressionLayer::new())
        .layer(from_fn_with_state(limits.clone(), limits::body_limit))
        .layer(axum::extract::DefaultBodyLimit::disable())
        .layer(from_fn_with_state(ctx.clone(), origin::origin_check))
        .layer(from_fn_with_state(ctx.clone(), origin::host_check))
        .layer(from_fn_with_state(limits, limits::timeout))
        .layer(from_fn_with_state(ctx, headers::security_headers))
        .layer(TraceLayer::new_for_http())
        .layer(SetSensitiveRequestHeadersLayer::new([header::AUTHORIZATION, header::COOKIE]))
}
