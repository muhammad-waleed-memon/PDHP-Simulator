//! Axum middleware functions used across the application.

use axum::{extract::Request, middleware::Next, response::Response};

/// Log the request ID on every request so it appears in tracing spans.
pub async fn log_request_id(req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none")
        .to_string();

    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %req.method(),
        uri = %req.uri(),
    );

    let _guard = span.enter();
    let response = next.run(req).await;
    response
}
