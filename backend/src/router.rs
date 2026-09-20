//! Application router — assembles all module routes into a single Axum Router.
//!
//! This is the single place where routes are registered. Each domain module
//! exposes a `router()` function that returns its sub-router.

use axum::{
    http::{HeaderName, HeaderValue, Method},
    Router,
};
use std::time::Duration;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::state::AppState;

/// Standard request ID header name used throughout the system.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Build the complete application router with all middleware applied.
pub fn build_router(state: AppState) -> Router {
    let cors = build_cors_layer(state.config());

    // The x-request-id header used for distributed tracing / log correlation.
    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);

    Router::new()
        // ── Health endpoints (no auth required) ────────────────────
        .merge(crate::health::router())
        // ── API v1 routes (auth required per-route) ────────────────
        .nest("/api/v1", api_v1_router())
        // ── FHIR routes ────────────────────────────────────────────
        .nest("/fhir", fhir_router())
        // ── Middleware (applied outside-in) ───────────────────────
        .layer(axum::middleware::from_fn(crate::middleware::log_request_id))
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// API v1 sub-router.
fn api_v1_router() -> Router<AppState> {
    Router::new()
        .nest("/auth", crate::auth::router())
        .nest("/patients", crate::patients::router())
        .nest("/doctors", crate::doctors::router())
        .nest("/facilities", crate::facilities::router())
        .nest("/consent", crate::consent::router())
        .nest("/medications", crate::medications::router())
        .merge(crate::conditions::router())
        .merge(crate::allergies::router())
        .merge(crate::patient_medications::router())
        .merge(crate::encounters::router())
        .merge(crate::prescriptions::router())
        .merge(crate::laboratories::router())
        .merge(crate::timeline::router())
}

/// FHIR sub-router.
fn fhir_router() -> Router<AppState> {
    crate::fhir::router()
}

/// Build the CORS layer from the application configuration.
fn build_cors_layer(config: &crate::config::Config) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect();

    if origins.is_empty() {
        // Should not happen in practice — config validation should catch this
        tracing::warn!("No valid CORS origins configured; CORS will be very restrictive");
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            HeaderName::from_static("x-request-id"),
        ]))
        .max_age(Duration::from_secs(3600))
}
