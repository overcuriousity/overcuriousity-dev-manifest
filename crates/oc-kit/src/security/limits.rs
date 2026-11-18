//! Body-size and timeout limits, picked per request by path prefix.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http_body_util::Limited;

use super::Limits;

impl Limits {
    pub(crate) fn for_path(&self, path: &str) -> (usize, Duration) {
        self.overrides
            .iter()
            .filter(|(p, _)| {
                path.strip_prefix(*p).is_some_and(|rest| p.ends_with('/') || rest.is_empty() || rest.starts_with('/'))
            })
            .max_by_key(|(p, _)| p.len())
            .map_or((self.body_bytes, self.timeout), |(_, l)| (l.body_bytes, l.timeout))
    }
}

/// Streamed bodies are wrapped in `Limited`; axum's extractors find its `LengthLimitError` and answer 413.
pub(crate) async fn body_limit(State(limits): State<Arc<Limits>>, req: Request, next: Next) -> Response {
    let max = limits.for_path(req.uri().path()).0;
    let declared = req.headers().get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()?.parse::<u64>().ok());
    if declared.is_some_and(|n| n > max as u64) {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    next.run(req.map(|b| Body::new(Limited::new(b, max)))).await
}

pub(crate) async fn timeout(State(limits): State<Arc<Limits>>, req: Request, next: Next) -> Response {
    let t = limits.for_path(req.uri().path()).1;
    tokio::time::timeout(t, next.run(req)).await.unwrap_or_else(|_| StatusCode::REQUEST_TIMEOUT.into_response())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::Limits;

    fn limits(prefixes: &[&'static str]) -> Limits {
        let o =
            |i: usize| Limits { body_bytes: 100 + i, timeout: Duration::from_secs(1 + i as u64), ..Limits::default() };
        Limits { overrides: prefixes.iter().enumerate().map(|(i, p)| (*p, o(i))).collect(), ..Limits::default() }
    }

    #[test]
    fn prefix_matches_on_whole_segments() {
        let l = limits(&["/upload"]);
        let base = (l.body_bytes, l.timeout);
        assert_eq!(l.for_path("/upload"), (100, Duration::from_secs(1)));
        assert_eq!(l.for_path("/upload/x"), (100, Duration::from_secs(1)));
        assert_eq!(l.for_path("/uploads"), base);
    }

    #[test]
    fn longest_prefix_wins() {
        assert_eq!(limits(&["/a", "/a/b"]).for_path("/a/b/c"), (101, Duration::from_secs(2)));
        assert_eq!(limits(&["/a/b", "/a"]).for_path("/a/b/c"), (100, Duration::from_secs(1)));
    }
}
