use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "Page not found.").into_response()
}
