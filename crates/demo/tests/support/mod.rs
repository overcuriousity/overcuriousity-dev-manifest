#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;

use axum::Router;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, Response, StatusCode, header};
use demo::state::AppState;
use oc_kit::config::OcConfig;
use tower::ServiceExt;

pub struct App {
    pub router: Router,
    pub st: AppState,
    pub cookie: String,
    _dir: tempfile::TempDir,
}

pub async fn app() -> App {
    let dir = tempfile::tempdir().unwrap();
    let pool = demo::db::open(dir.path()).await.unwrap();
    oc_kit::identity::users::create(&pool, "ana", "Ana Analyst", true, Some("pw-ana-12345"), demo::domain::now())
        .await
        .unwrap();
    demo::domain::seed::seed(&pool, demo::domain::now()).await.unwrap();
    let (router, st) = demo::app::build(&OcConfig::default(), pool).await.unwrap();
    let router = router.layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 5000))));
    let mut a = App { router, st, cookie: String::new(), _dir: dir };
    let res = a.post("/auth/sign-in", "username=ana&password=pw-ana-12345&next=/").await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    a.cookie = res.headers()[header::SET_COOKIE].to_str().unwrap().split(';').next().unwrap().to_owned();
    a
}

impl App {
    pub async fn send(&self, r: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(r).await.unwrap()
    }

    pub fn req(&self, method: &str, uri: &str) -> axum::http::request::Builder {
        let mut b = Request::builder().method(method).uri(uri).header(header::HOST, "localhost:8080");
        if !self.cookie.is_empty() {
            b = b.header(header::COOKIE, &self.cookie);
        }
        if method != "GET" {
            b = b.header(header::ORIGIN, "http://localhost:8080");
        }
        b
    }

    pub async fn get(&self, uri: &str) -> Response<Body> {
        self.send(self.req("GET", uri).header(header::ACCEPT, "text/html").body(Body::empty()).unwrap()).await
    }

    pub async fn htmx_get(&self, uri: &str) -> Response<Body> {
        self.send(self.req("GET", uri).header("hx-request", "true").body(Body::empty()).unwrap()).await
    }

    pub async fn post(&self, uri: &str, form: &str) -> Response<Body> {
        let r = self
            .req("POST", uri)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header("hx-request", "true")
            .body(Body::from(form.to_owned()))
            .unwrap();
        self.send(r).await
    }

    pub async fn text(&self, uri: &str) -> String {
        let res = self.get(uri).await;
        assert!(res.status().is_success(), "{uri}: {}", res.status());
        body(res).await
    }
}

pub async fn body(res: Response<Body>) -> String {
    String::from_utf8(axum::body::to_bytes(res.into_body(), 8 << 20).await.unwrap().to_vec()).unwrap()
}

pub async fn first_note_id(a: &App) -> i64 {
    demo::domain::notes::list(&a.st.pool, None, "", false).await.unwrap()[0].id
}
