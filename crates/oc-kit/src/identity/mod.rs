pub mod password;
pub mod tokens;
pub mod users;

pub use users::User;
pub mod extract;

use crate::auth::AuthState;
use crate::error::OcError;

/// Bearer verification grows with the configured mechanisms (app tokens, IdP JWTs).
pub async fn verify_bearer(st: &AuthState, token: &str) -> Result<Option<User>, OcError> {
    if token.starts_with(&format!("{}_", st.token_prefix)) {
        return tokens::verify(
            &st.pool,
            token,
            st.token_prefix,
            time::OffsetDateTime::now_utc(),
        )
        .await;
    }
    crate::auth::oidc::user_from_jwt(st, token).await
}

pub fn www_authenticate(st: &AuthState) -> Option<String> {
    st.oidc.as_ref().map(|_| {
        format!(
            "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource\"",
            st.dep.origin
        )
    })
}
