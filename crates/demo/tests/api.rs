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
    oc_kit::identity::tokens::mint(&a.st.pool, u.id, "test", "demo", demo::domain::now())
        .await
        .unwrap()
        .1
}

async fn api(a: &App, uri: &str, bearer: Option<&str>) -> axum::response::Response {
    let mut r = axum::http::Request::get(uri)
        .header(header::HOST, "localhost:8080")
        .header(header::ACCEPT, "application/json");
    if let Some(t) = bearer {
        r = r.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    a.send(r.body(Body::empty()).unwrap()).await
}

async fn json(res: axum::response::Response) -> serde_json::Value {
    serde_json::from_str(&body(res).await).unwrap()
}

#[tokio::test]
async fn every_route_is_in_the_openapi_spec_and_vice_versa() {
    let a = app().await;
    let t = token(&a).await;
    let spec = json(api(&a, "/api/v1/openapi.json", Some(&t)).await).await;
    let mut paths: Vec<String> = spec["paths"].as_object().unwrap().keys().cloned().collect();
    paths.sort();
    assert_eq!(
        paths,
        [
            "/api/v1/cases",
            "/api/v1/cases/{id}/events",
            "/api/v1/events/{id}",
            "/api/v1/notes",
            "/api/v1/notes/{id}"
        ]
    );
    let case = demo::domain::cases::list(&a.st.pool).await.unwrap()[0].id;
    let note = first_note_id(&a).await;
    for uri in [
        "/api/v1/cases".to_owned(),
        format!("/api/v1/cases/{case}/events"),
        "/api/v1/events/1".into(),
        "/api/v1/notes".into(),
        format!("/api/v1/notes/{note}"),
    ] {
        assert_eq!(
            api(&a, &uri, Some(&t)).await.status(),
            StatusCode::OK,
            "{uri}"
        );
    }
}

#[tokio::test]
async fn events_paginate_with_next_cursor_until_null() {
    let a = app().await;
    let t = token(&a).await;
    let case = demo::domain::cases::list(&a.st.pool)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.name == "Lab ws-07")
        .unwrap();
    let (mut total, mut cursor) = (0, String::new());
    loop {
        let page = json(
            api(
                &a,
                &format!("/api/v1/cases/{}/events?limit=400&cursor={cursor}", case.id),
                Some(&t),
            )
            .await,
        )
        .await;
        total += page["items"].as_array().unwrap().len() as i64;
        match page["next_cursor"].as_str() {
            Some(c) => cursor = c.to_owned(),
            None => break,
        }
    }
    assert_eq!(total, case.events);
}

#[tokio::test]
async fn errors_are_one_sentence_json_and_credentials_are_required() {
    let a = app().await;
    let t = token(&a).await;
    let case = demo::domain::cases::list(&a.st.pool).await.unwrap()[0].id;
    let res = api(
        &a,
        &format!("/api/v1/cases/{case}/events?q=nope%3Ax"),
        Some(&t),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert!(
        json(res).await["error"]
            .as_str()
            .unwrap()
            .starts_with("Unknown field 'nope'.")
    );
    let res = api(&a, "/api/v1/cases", None).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json(res).await["error"], "Sign in first.");
}

#[tokio::test]
async fn a_bearer_token_can_create_a_note_without_an_origin_header() {
    let a = app().await;
    let t = token(&a).await;
    let r = axum::http::Request::post("/api/v1/notes")
        .header(header::HOST, "localhost:8080")
        .header(header::AUTHORIZATION, format!("Bearer {t}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"title":"From the API","body":"x","tags":["api"]}"#,
        ))
        .unwrap();
    let res = a.send(r).await;
    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(json(res).await["title"], "From the API");
}
