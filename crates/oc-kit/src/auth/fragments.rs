//! HTML fragments returned by the auth routes.

use axum::response::{IntoResponse, Response};

use crate::identity::tokens::TokenInfo;
use crate::ui::{Button, Html, HxAttrs, Notice, TokenList, TokenMinted};

pub(super) fn token_minted(info: &TokenInfo, token: &str) -> Response {
    Html::from(TokenMinted { name: info.name.clone(), token: token.to_owned() }).into_response()
}

pub(super) fn token_revoked(info: &TokenInfo) -> Response {
    let undo = Button::default("Undo")
        .attrs(HxAttrs::post(format!("/auth/tokens/{}/restore", info.id)).target("#oc-token-list").swap("outerHTML"));
    let notice = Notice::success(format!("Revoked token '{}'", info.name)).action(undo).autohide(crate::trash::UNDO_MS);
    notice.oob().into_response()
}

pub(super) fn token_list(all: &[TokenInfo]) -> Response {
    Html::from(TokenList { tokens: all.to_vec() }).into_response()
}
