pub mod password;
pub mod users;

pub use users::User;
pub mod extract;

use crate::auth::AuthState;
use crate::error::OcError;

/// Bearer verification grows with the configured mechanisms (app tokens, IdP JWTs).
pub async fn verify_bearer(_st: &AuthState, _token: &str) -> Result<Option<User>, OcError> {
    Ok(None)
}

pub fn www_authenticate(_st: &AuthState) -> Option<String> {
    None
}
