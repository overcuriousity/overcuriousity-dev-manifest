//! OIDC sign-in as a backend-for-frontend: the browser only ever holds the session cookie.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl,
    Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::sync::{OnceCell, RwLock};
use tower_sessions::Session;

use super::{AuthState, render_sign_in, safe_next};
use crate::config::{Deployment, OidcConfig};
use crate::error::OcError;
use crate::identity::{User, users};
use crate::session::USER_KEY;

const PENDING_KEY: &str = "oc.oidc.pending";

type Client =
    CoreClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointMaybeSet, EndpointMaybeSet>;

pub struct Oidc {
    cfg: OidcConfig,
    redirect: RedirectUrl,
    resource: String,
    http: reqwest::Client,
    client: OnceCell<(Client, String)>,
    jwks: RwLock<Option<(JwkSet, Instant)>>,
}

#[derive(Serialize, Deserialize)]
struct Pending {
    state: String,
    nonce: String,
    verifier: String,
    next: String,
}

impl Oidc {
    pub fn new(cfg: &OidcConfig, dep: &Deployment) -> Result<Arc<Self>, OcError> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(OcError::internal)?;
        Ok(Arc::new(Self {
            cfg: cfg.clone(),
            redirect: RedirectUrl::new(format!("{}/auth/oidc/callback", dep.origin)).map_err(OcError::internal)?,
            resource: format!("{}/mcp", dep.origin),
            http,
            client: OnceCell::new(),
            jwks: RwLock::new(None),
        }))
    }

    /// Discovery is lazy, so the app starts even while the IdP is unreachable.
    async fn client(&self) -> Result<&(Client, String), OcError> {
        self.client
            .get_or_try_init(|| async {
                let issuer = IssuerUrl::new(self.cfg.issuer.as_str().trim_end_matches('/').to_owned())
                    .map_err(OcError::internal)?;
                let meta = CoreProviderMetadata::discover_async(issuer, &self.http).await.map_err(|e| {
                    tracing::warn!(error = %e, "OIDC discovery failed");
                    OcError::BadRequest("The sign-in provider is not reachable. Try again later.".into())
                })?;
                let jwks_uri = meta.jwks_uri().url().to_string();
                let client = CoreClient::from_provider_metadata(
                    meta,
                    ClientId::new(self.cfg.client_id.clone()),
                    Some(ClientSecret::new(self.cfg.client_secret.expose().to_owned())),
                )
                .set_redirect_uri(self.redirect.clone());
                Ok((client, jwks_uri))
            })
            .await
    }

    async fn jwks(&self, refresh: bool) -> Result<JwkSet, OcError> {
        if let Some((set, at)) = self.jwks.read().await.as_ref()
            && (!refresh || at.elapsed() < Duration::from_secs(60))
        {
            return Ok(set.clone());
        }
        let (_, uri) = self.client().await?;
        let set: JwkSet =
            self.http.get(uri).send().await.map_err(OcError::internal)?.json().await.map_err(OcError::internal)?;
        *self.jwks.write().await = Some((set.clone(), Instant::now()));
        Ok(set)
    }

    /// Validates an IdP-issued JWT bearer. Accepted audiences: the client ID or this app's MCP resource.
    pub async fn verify_jwt(&self, token: &str) -> Result<Option<(String, String)>, OcError> {
        let Ok(header) = decode_header(token) else {
            return Ok(None);
        };
        let Some(kid) = header.kid else {
            return Ok(None);
        };
        let mut set = self.jwks(false).await?;
        if set.find(&kid).is_none() {
            set = self.jwks(true).await?;
        }
        let Some(jwk) = set.find(&kid) else {
            return Ok(None);
        };
        let key = match DecodingKey::from_jwk(jwk) {
            Ok(k) => k,
            Err(e) => {
                tracing::debug!(error = %e, "unusable JWK");
                return Ok(None);
            }
        };
        let mut v = Validation::new(header.alg);
        v.set_issuer(&[self.cfg.issuer.as_str().trim_end_matches('/')]);
        v.set_audience(&[self.cfg.client_id.as_str(), self.resource.as_str()]);
        #[derive(Deserialize)]
        struct Claims {
            iss: String,
            sub: String,
        }
        match decode::<Claims>(token, &key, &v) {
            Ok(d) => Ok(Some((d.claims.iss, d.claims.sub))),
            Err(e) => {
                tracing::debug!(error = %e, "JWT bearer rejected");
                Ok(None)
            }
        }
    }

    pub fn issuer(&self) -> &str {
        self.cfg.issuer.as_str().trim_end_matches('/')
    }
}

pub(super) fn routes() -> Router<AuthState> {
    Router::new().route("/oidc/start", get(start)).route("/oidc/callback", get(callback))
}

fn oidc(st: &AuthState) -> Result<&Oidc, OcError> {
    st.oidc.as_deref().ok_or(OcError::NotFound)
}

#[derive(Deserialize)]
struct StartQuery {
    #[serde(default)]
    next: String,
}

async fn start(
    State(st): State<AuthState>,
    session: Session,
    Query(q): Query<StartQuery>,
) -> Result<Response, OcError> {
    let o = oidc(&st)?;
    let (client, _) = o.client().await?;
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let mut req =
        client.authorize_url(CoreAuthenticationFlow::AuthorizationCode, CsrfToken::new_random, Nonce::new_random);
    for s in o.cfg.scopes.iter().filter(|s| *s != "openid") {
        req = req.add_scope(Scope::new(s.clone()));
    }
    let (url, state, nonce) = req.set_pkce_challenge(challenge).url();
    let pending = Pending {
        state: state.secret().clone(),
        nonce: nonce.secret().clone(),
        verifier: verifier.secret().clone(),
        next: safe_next(&q.next).to_owned(),
    };
    session.insert(PENDING_KEY, pending).await.map_err(OcError::internal)?;
    Ok(Redirect::to(url.as_str()).into_response())
}

async fn callback(
    State(st): State<AuthState>,
    session: Session,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Response, OcError> {
    let o = oidc(&st)?;
    let failed = || render_sign_in(&st, "/", Some("Sign-in with the provider failed. Try again."));
    let Some(p) = session.remove::<Pending>(PENDING_KEY).await.map_err(OcError::internal)? else {
        return Ok(failed());
    };
    let (Some(code), Some(state)) = (q.get("code"), q.get("state")) else {
        return Ok(failed());
    };
    if *state != p.state {
        return Ok(failed());
    }
    let (client, _) = o.client().await?;
    let Ok(exchange) = client.exchange_code(AuthorizationCode::new(code.clone())) else {
        return Ok(failed());
    };
    let token = match exchange.set_pkce_verifier(PkceCodeVerifier::new(p.verifier)).request_async(&o.http).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "OIDC code exchange failed");
            return Ok(failed());
        }
    };
    let Some(id_token) = token.id_token() else {
        return Ok(failed());
    };
    let claims = match id_token.claims(&client.id_token_verifier(), &Nonce::new(p.nonce)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "ID token rejected");
            return Ok(failed());
        }
    };
    let name = claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.as_str().to_owned())
        .unwrap_or_else(|| claims.subject().as_str().to_owned());
    let preferred = claims.preferred_username().map(|u| u.as_str().to_owned());
    let user = users::find_or_provision_oidc(
        &st.pool,
        claims.issuer().as_str(),
        claims.subject().as_str(),
        &name,
        preferred.as_deref(),
        o.cfg.auto_provision,
        OffsetDateTime::now_utc(),
    )
    .await?;
    let Some(user) = user else {
        return Ok(render_sign_in(&st, "/", Some("No account for this sign-in. Ask an admin to add you.")));
    };
    session.cycle_id().await.map_err(OcError::internal)?;
    session.insert(USER_KEY, user.id).await.map_err(OcError::internal)?;
    Ok(Redirect::to(&p.next).into_response())
}

pub(crate) async fn user_from_jwt(st: &AuthState, token: &str) -> Result<Option<User>, OcError> {
    let Some(o) = st.oidc.as_deref() else {
        return Ok(None);
    };
    let Some((iss, sub)) = o.verify_jwt(token).await? else {
        return Ok(None);
    };
    let id = sqlx::query_scalar!("SELECT user_id FROM oc_oidc_identities WHERE issuer = ? AND subject = ?", iss, sub)
        .fetch_optional(&st.pool)
        .await?;
    match id {
        Some(id) => users::by_id(&st.pool, id).await,
        None => Ok(None),
    }
}

/// RFC 9728 metadata pointing MCP clients at the IdP. Mount at the root; it answers 404 without OIDC.
pub fn resource_metadata_router() -> Router<AuthState> {
    Router::new().route("/.well-known/oauth-protected-resource", get(resource_metadata))
}

async fn resource_metadata(State(st): State<AuthState>) -> Result<Response, OcError> {
    let o = oidc(&st)?;
    Ok(Json(serde_json::json!({ "resource": o.resource, "authorization_servers": [o.issuer()] })).into_response())
}
