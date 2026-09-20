//! Centralized application error types and HTTP error response formatting.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// The top-level application error type.
#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum AppError {
    // ── 400 Bad Request ───────────────────────────────────────────
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    // ── 401 Unauthorized ─────────────────────────────────────────
    #[error("Authentication required")]
    Unauthorized,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired or invalid")]
    TokenInvalid,

    // ── 403 Forbidden ─────────────────────────────────────────────
    #[error("Insufficient permissions: {0}")]
    Forbidden(String),

    #[error("Consent required for this operation")]
    ConsentRequired,

    // ── 404 Not Found ─────────────────────────────────────────────
    #[error("Resource not found: {0}")]
    NotFound(String),

    // ── 409 Conflict ──────────────────────────────────────────────
    #[error("Resource already exists: {0}")]
    Conflict(String),

    // ── 429 Too Many Requests ─────────────────────────────────────
    #[error("Too many requests")]
    RateLimited,

    // ── 500 Internal Server Error ────────────────────────────────
    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),

    /// Database errors — internal details are logged, not returned.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl AppError {
    pub fn internal<M: std::fmt::Display>(msg: M) -> Self {
        AppError::Internal(anyhow::anyhow!(msg.to_string()))
    }
}

/// The JSON body structure for error responses.
#[allow(dead_code)]
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[allow(dead_code)]
impl ErrorResponse {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            error: ErrorDetail {
                code,
                message: message.into(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct ErrorDetail {
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    /// HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) | AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized | AppError::InvalidCredentials | AppError::TokenInvalid => {
                StatusCode::UNAUTHORIZED
            }
            AppError::Forbidden(_) | AppError::ConsentRequired => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::Internal(_) | AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Short error code string for the JSON response.
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::InvalidCredentials => "INVALID_CREDENTIALS",
            AppError::TokenInvalid => "TOKEN_INVALID",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::ConsentRequired => "CONSENT_REQUIRED",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::RateLimited => "RATE_LIMITED",
            AppError::Internal(_) => "INTERNAL_SERVER_ERROR",
            AppError::Database(_) => "INTERNAL_SERVER_ERROR",
        }
    }

    /// Client-safe error message (no internal details).
    pub fn client_message(&self) -> String {
        match self {
            // Safe to return verbatim
            AppError::Validation(msg) => msg.clone(),
            AppError::BadRequest(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Forbidden(msg) => msg.clone(),
            // Generic messages for sensitive errors
            AppError::Unauthorized => "Authentication required".to_string(),
            AppError::InvalidCredentials => "Invalid username or password".to_string(),
            AppError::TokenInvalid => "Token is expired or invalid".to_string(),
            AppError::ConsentRequired => {
                "Patient consent is required to access this resource".to_string()
            }
            AppError::RateLimited => "Too many requests. Please wait before retrying.".to_string(),
            // Internal errors: log the real cause, return generic message
            AppError::Internal(_) => "An internal server error occurred".to_string(),
            AppError::Database(_) => "An internal server error occurred".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            AppError::Internal(e) => {
                tracing::error!(error = %e, "Internal server error");
            }
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error");
            }
            _ => {
                tracing::debug!(error = %self, "Application error");
            }
        }

        let status = self.status_code();
        let body = ErrorResponse {
            error: ErrorDetail {
                code: self.error_code(),
                message: self.client_message(),
            },
        };

        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_is_400() {
        let err = AppError::Validation("field is required".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code(), "VALIDATION_ERROR");
    }

    #[test]
    fn unauthorized_is_401() {
        let err = AppError::Unauthorized;
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn forbidden_is_403() {
        let err = AppError::Forbidden("Access denied".to_string());
        assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn not_found_is_404() {
        let err = AppError::NotFound("Resource not found".to_string());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn internal_error_does_not_expose_details() {
        let err = AppError::Internal(anyhow::anyhow!("SECRET INTERNAL DETAIL"));
        assert!(!err.client_message().contains("SECRET INTERNAL DETAIL"));
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
