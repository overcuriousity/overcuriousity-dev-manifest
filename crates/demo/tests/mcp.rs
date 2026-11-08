#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use axum::body::Body;
use axum::http::{StatusCode, header};
use support::*;

async fn token(a: &App) -> String {
    let u = oc_kit::identity::users::by_username(&a.st.pool, "ana")
        .await
        .unwrap()
        .unwrap();
    oc_kit::identity::tokens::mint(&a.st.pool, u.id, "mcp", "demo", demo::domain::now())
        .await
        .unwrap()
        .1
}

async fn rpc(
    a: &App,
    token: Option<&str>,
    session: Option<&str>,
    body: &str,
) -> axum::response::Response {
    let mut r = axum::http::Request::post("/mcp")
        .header(header::HOST, "localhost:8080")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream");
    if let Some(t) = token {
        r = r.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(s) = session {
        r = r
            .header("mcp-session-id", s)
            .header("mcp-protocol-version", "2025-06-18");
    }
    a.send(r.body(Body::from(body.to_owned())).unwrap()).await
}

async fn text(res: axum::response::Response) -> String {
    use http_body_util::BodyExt;
    let mut body = res.into_body();
    let mut out = String::new();
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), async {
        while let Some(Ok(f)) = body.frame().await {
            if let Some(d) = f.data_ref() {
                out.push_str(&String::from_utf8_lossy(d));
                if out.contains("\"result\"") || out.contains("\"error\"") {
                    break;
                }
            }
        }
    })
    .await;
    out
}

async fn session(a: &App, t: &str) -> String {
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#;
    let res = rpc(a, Some(t), None, init).await;
    assert_eq!(res.status(), StatusCode::OK);
    let sid = res.headers()["mcp-session-id"].to_str().unwrap().to_owned();
    let _ = text(res).await;
    rpc(
        a,
        Some(t),
        Some(&sid),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    )
    .await;
    sid
}

#[tokio::test]
async fn mcp_without_a_token_gets_401_with_www_authenticate() {
    let a = app().await;
    let res = rpc(&a, None, None, "{}").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(res.headers().contains_key(header::WWW_AUTHENTICATE));
}

#[tokio::test]
async fn tools_list_names_the_four_tools_and_search_matches_the_api() {
    let a = app().await;
    let t = token(&a).await;
    let sid = session(&a, &t).await;
    let list = text(
        rpc(
            &a,
            Some(&t),
            Some(&sid),
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        )
        .await,
    )
    .await;
    for name in ["list_cases", "search_events", "get_event", "add_note"] {
        assert!(
            list.contains(&format!("\"name\":\"{name}\"")),
            "{name} missing: {list}"
        );
    }
    let case = demo::domain::cases::list(&a.st.pool).await.unwrap()[0].id;
    let call = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"search_events","arguments":{{"case_id":{case},"query":"event_id:4688","limit":5}}}}}}"#
    );
    let out = text(rpc(&a, Some(&t), Some(&sid), &call).await).await;
    let q = demo::domain::query::parse("event_id:4688").unwrap();
    let first = demo::domain::events::search(&a.st.pool, case, &q, None, 5)
        .await
        .unwrap()
        .items[0]
        .id;
    assert!(out.contains(&format!("\\\"id\\\":{first}")), "{out}");
}

#[tokio::test]
async fn add_note_creates_a_note_visible_in_the_web_ui() {
    let a = app().await;
    let t = token(&a).await;
    let sid = session(&a, &t).await;
    let call = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"add_note","arguments":{"title":"Written by an agent","body":"Summary."}}}"#;
    let out = text(rpc(&a, Some(&t), Some(&sid), call).await).await;
    assert!(out.contains("Written by an agent"), "{out}");
    assert!(a.text("/notes").await.contains("Written by an agent"));
}
