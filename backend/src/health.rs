//! Health check endpoints.
//!
//! GET /health        — Liveness: is the application running?
//! GET /health/ready  — Readiness: is the application ready to serve traffic?
//!                      (Includes database connectivity check)

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use chrono::Utc;
use serde::Serialize;

use crate::state::AppState;

/// Health check response body.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub timestamp: String,
}

/// Readiness check response body (includes subsystem statuses).
#[derive(Serialize)]
pub struct ReadinessResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub timestamp: String,
    pub checks: ReadinessChecks,
}

#[derive(Serialize)]
pub struct ReadinessChecks {
    pub database: SubsystemStatus,
}

#[derive(Serialize)]
pub struct SubsystemStatus {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Build the health check router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(liveness))
        .route("/health/ready", get(readiness))
}

/// GET /health
///
/// Simple liveness check. Returns 200 if the application process is alive.
/// Does not check database connectivity.
async fn liveness() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            timestamp: Utc::now().to_rfc3339(),
        }),
    )
}

/// GET /health/ready
///
/// Readiness check. Returns 200 if the application is ready to serve traffic
/// (database is reachable). Returns 503 if not ready.
async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let db_status = check_database(state.db()).await;
    let all_healthy = db_status.status == "ok";

    let http_status = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        http_status,
        Json(ReadinessResponse {
            status: if all_healthy { "ok" } else { "degraded" },
            version: env!("CARGO_PKG_VERSION"),
            timestamp: Utc::now().to_rfc3339(),
            checks: ReadinessChecks {
                database: db_status,
            },
        }),
    )
}

async fn check_database(pool: &sqlx::PgPool) -> SubsystemStatus {
    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => SubsystemStatus {
            status: "ok",
            message: None,
        },
        Err(e) => {
            tracing::warn!(error = %e, "Database health check failed");
            SubsystemStatus {
                status: "error",
                // Generic message — do not expose connection string or internals
                message: Some("Database connectivity check failed".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_serializes() {
        let resp = HealthResponse {
            status: "ok",
            version: "0.1.0",
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"version\":\"0.1.0\""));
    }

    #[test]
    fn readiness_response_serializes() {
        let resp = ReadinessResponse {
            status: "ok",
            version: "0.1.0",
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            checks: ReadinessChecks {
                database: SubsystemStatus {
                    status: "ok",
                    message: None,
                },
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"database\""));
        // Ensure "message" is omitted when None (skip_serializing_if)
        assert!(!json.contains("\"message\""));
    }

    #[test]
    fn degraded_response_includes_message() {
        let resp = ReadinessResponse {
            status: "degraded",
            version: "0.1.0",
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            checks: ReadinessChecks {
                database: SubsystemStatus {
                    status: "error",
                    message: Some("Database connectivity check failed".to_string()),
                },
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"degraded\""));
        assert!(json.contains("\"message\""));
    }
}
