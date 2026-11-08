#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, jwk::Jwk};
use serde_json::{Value, json};

pub const KEY: &[u8] = include_bytes!("../fixtures/test-idp-es256.pem");

#[derive(Clone)]
pub struct Idp {
    pub issuer: String,
}

impl Idp {
    pub fn sign(&self, claims: Value) -> String {
        let mut h = Header::new(Algorithm::ES256);
        h.kid = Some("k1".into());
        encode(&h, &claims, &EncodingKey::from_ec_pem(KEY).unwrap()).unwrap()
    }

    pub fn id_token(&self, sub: &str, nonce: &str, aud: &str) -> String {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        self.sign(json!({"iss": self.issuer, "sub": sub, "aud": aud, "nonce": nonce, "iat": now, "exp": now + 300, "name": "Test Person", "preferred_username": "tp"}))
    }
}

/// The token endpoint treats the authorization code as `sub:nonce`, so tests choose both.
pub async fn start() -> Idp {
    start_at("").await
}

/// `path` like `/realm/` gives an issuer with a trailing slash, as Authentik and Auth0 use.
pub async fn start_at(path: &str) -> Idp {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}{path}", listener.local_addr().unwrap());
    let base = path.trim_end_matches('/');
    let idp = Idp { issuer };
    let app = Router::new()
        .route(&format!("{base}/.well-known/openid-configuration"), get(discovery))
        .route(&format!("{base}/jwks"), get(jwks))
        .route(&format!("{base}/token"), post(token))
        .with_state(idp.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    idp
}

async fn discovery(State(idp): State<Idp>) -> Json<Value> {
    let i = idp.issuer.trim_end_matches('/');
    let iss = &idp.issuer;
    Json(json!({
        "issuer": iss,
        "authorization_endpoint": format!("{i}/authorize"),
        "token_endpoint": format!("{i}/token"),
        "jwks_uri": format!("{i}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["ES256"],
    }))
}

async fn jwks() -> Json<Value> {
    let mut jwk = Jwk::from_encoding_key(&EncodingKey::from_ec_pem(KEY).unwrap(), Algorithm::ES256).unwrap();
    jwk.common.key_id = Some("k1".into());
    Json(json!({ "keys": [jwk] }))
}

async fn token(State(idp): State<Idp>, Form(f): Form<std::collections::HashMap<String, String>>) -> Json<Value> {
    let (sub, nonce) = f["code"].split_once(':').unwrap();
    assert!(f.contains_key("code_verifier"), "PKCE verifier missing");
    let aud = f.get("client_id").cloned().unwrap_or_else(|| "app".into());
    Json(
        json!({"access_token": "at", "token_type": "Bearer", "expires_in": 300, "id_token": idp.id_token(sub, nonce, &aud)}),
    )
}
