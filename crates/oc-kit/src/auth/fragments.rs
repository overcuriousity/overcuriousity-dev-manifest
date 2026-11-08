//! HTML fragments returned by the auth routes. Replaced by Askama components in `ui`.

use axum::response::{Html, IntoResponse, Response};

use crate::identity::tokens::TokenInfo;

pub(super) fn token_minted(_info: &TokenInfo, token: &str) -> Response {
    Html(format!("<p>{token}</p>")).into_response()
}

pub(super) fn token_revoked(info: &TokenInfo) -> Response {
    Html(format!("<p>Revoked {}</p>", info.name)).into_response()
}

pub(super) fn token_list(all: &[TokenInfo]) -> Response {
    Html(
        all.iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>()
            .join(","),
    )
    .into_response()
}
