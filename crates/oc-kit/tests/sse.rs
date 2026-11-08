#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, header};
use axum::routing::get;
use http_body_util::BodyExt;
use oc_kit::config::OcConfig;
use oc_kit::security::{Limits, secure};
use oc_kit::sse::{Broadcaster, SseEvent, sse_response};
use tower::ServiceExt;

fn app(b: Broadcaster<u32>, timeout: Duration) -> Router {
    let r = Router::new().route(
        "/live",
        get(move || {
            let rx = b.subscribe();
            async move {
                sse_response(rx, |v: &u32| v.is_multiple_of(2).then(|| SseEvent { name: "tick", data: v.to_string() }))
            }
        }),
    );
    let cfg = OcConfig::default();
    secure(r, &cfg.deployment().unwrap(), &cfg.trusted_proxies, Limits { timeout, ..Limits::default() })
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))))
}

async fn read_for(body: &mut Body, d: Duration) -> String {
    let mut out = String::new();
    let _ = tokio::time::timeout(d, async {
        while let Some(Ok(f)) = body.frame().await {
            if let Some(b) = f.data_ref() {
                out.push_str(&String::from_utf8_lossy(b));
            }
        }
    })
    .await;
    out
}

fn req() -> Request<Body> {
    Request::get("/live").header(header::HOST, "localhost:8080").body(Body::empty()).unwrap()
}

#[tokio::test]
async fn events_stream_with_their_names_and_disable_proxy_buffering() {
    let b = Broadcaster::new(16);
    let res = app(b.clone(), Duration::from_secs(5)).oneshot(req()).await.unwrap();
    assert_eq!(res.headers()["x-accel-buffering"], "no");
    assert_eq!(res.headers()[header::CONTENT_TYPE], "text/event-stream");
    for v in 1..=4 {
        b.send(v);
    }
    let text = read_for(&mut res.into_body(), Duration::from_millis(200)).await;
    assert!(text.contains("event: tick\ndata: 2") && text.contains("data: 4") && !text.contains("data: 1"), "{text}");
}

#[tokio::test]
async fn a_lagging_subscriber_receives_resync_and_keeps_streaming() {
    let b = Broadcaster::new(2);
    let res = app(b.clone(), Duration::from_secs(5)).oneshot(req()).await.unwrap();
    for v in 0..10 {
        b.send(v * 2);
    }
    let mut body = res.into_body();
    let first = read_for(&mut body, Duration::from_millis(150)).await;
    assert!(first.contains("event: resync"), "{first}");
    b.send(100);
    assert!(read_for(&mut body, Duration::from_millis(150)).await.contains("data: 100"));
}

#[tokio::test]
async fn the_stream_is_not_cut_by_the_request_timeout() {
    let b = Broadcaster::new(4);
    let res = app(b.clone(), Duration::from_millis(100)).oneshot(req()).await.unwrap();
    let mut body = res.into_body();
    tokio::time::sleep(Duration::from_millis(300)).await;
    b.send(8);
    assert!(read_for(&mut body, Duration::from_millis(150)).await.contains("data: 8"));
}

#[tokio::test]
async fn events_without_a_payload_still_carry_a_data_line_so_browsers_dispatch_them() {
    let b = Broadcaster::new(4);
    let r = Router::new().route(
        "/live",
        get({
            let b = b.clone();
            move || {
                let rx = b.subscribe();
                async move { sse_response(rx, |_: &u32| Some(SseEvent { name: "ping", data: String::new() })) }
            }
        }),
    );
    let cfg = OcConfig::default();
    let app = secure(r, &cfg.deployment().unwrap(), &cfg.trusted_proxies, Limits::default())
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
    let res = app.oneshot(req()).await.unwrap();
    b.send(1);
    let text = read_for(&mut res.into_body(), Duration::from_millis(150)).await;
    assert!(text.contains("event: ping\ndata:"), "{text:?}");
}
