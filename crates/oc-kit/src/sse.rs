use std::convert::Infallible;
use std::time::Duration;

use axum::http::{HeaderValue, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

#[derive(Clone)]
pub struct Broadcaster<T: Clone + Send + 'static> {
    tx: broadcast::Sender<T>,
}

impl<T: Clone + Send + 'static> Broadcaster<T> {
    pub fn new(capacity: usize) -> Self {
        Self { tx: broadcast::channel(capacity).0 }
    }

    pub fn send(&self, v: T) {
        let _ = self.tx.send(v);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.tx.subscribe()
    }
}

/// Consumed by htmx's SSE extension via `sse-swap="<name>"` or `hx-trigger="sse:<name>"`.
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub name: &'static str,
    pub data: String,
}

/// A client that falls behind the channel gets a `resync` event, so it can refetch instead of silently missing data.
pub fn sse_response<T, F>(rx: broadcast::Receiver<T>, render: F) -> Response
where
    T: Clone + Send + 'static,
    F: Fn(&T) -> Option<SseEvent> + Send + Sync + 'static,
{
    let stream = BroadcastStream::new(rx).filter_map(move |item| {
        let ev = match item {
            Ok(v) => render(&v)?,
            Err(BroadcastStreamRecvError::Lagged(_)) => SseEvent { name: "resync", data: String::new() },
        };
        Some(Ok::<_, Infallible>(Event::default().event(ev.name).data(ev.data)))
    });
    let mut res = Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))).into_response();
    res.headers_mut().insert("x-accel-buffering", HeaderValue::from_static("no"));
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}
