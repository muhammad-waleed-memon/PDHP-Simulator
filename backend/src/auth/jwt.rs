//! JWT creation, validation, and Axum request extractor.

use crate::error::{AppError, ErrorResponse};
use crate::state::AppState;
use crate::users::UserRole;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims embedded in authentication tokens.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (User UUID)
    pub sub: Uuid,
    /// User Login Email
    pub email: String,
    /// User Role
    pub role: UserRole,
    /// Expiration time (unix timestamp)
    pub exp: usize,
    /// Issued at time (unix timestamp)
    pub iat: usize,
}

/// Extractor struct representing an authenticated request identity.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub email: String,
    pub role: UserRole,
}

/// Create a new JWT token for a user.
pub fn create_jwt(
    user_id: Uuid,
    email: &str,
    role: UserRole,
    secret: &str,
    expires_in_seconds: u64,
) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        role,
        exp: now + (expires_in_seconds as usize),
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("Failed to generate authentication token: {e}")))
}

/// Validate a JWT token string against the application secret.
pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::TokenInvalid)?;

    Ok(token_data.claims)
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());

        let auth_header = match auth_header {
            Some(h) if h.starts_with("Bearer ") => &h[7..],
            _ => {
                let err =
                    ErrorResponse::new("UNAUTHORIZED", "Missing or invalid Authorization header");
                return Err((StatusCode::UNAUTHORIZED, Json(err)).into_response());
            }
        };

        // Verify token
        let claims = match verify_jwt(auth_header, &state.config().jwt_secret) {
            Ok(c) => c,
            Err(e) => {
                let status = e.status_code();
                let err = ErrorResponse::new(e.error_code(), e.client_message());
                return Err((status, Json(err)).into_response());
            }
        };

        Ok(AuthenticatedUser {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_validation() {
        let user_id = Uuid::new_v4();
        let email = "doctor@demo.local";
        let role = UserRole::Doctor;
        let secret = "super_secret_test_key_minimum_length_requirement";

        let token = create_jwt(user_id, email, role.clone(), secret, 3600)
            .expect("JWT creation should succeed");
        assert!(!token.is_empty());

        let claims = verify_jwt(&token, secret).expect("JWT verification should succeed");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_expired_jwt_fails() {
        let user_id = Uuid::new_v4();
        let email = "patient@demo.local";
        let secret = "super_secret_test_key_minimum_length_requirement";

        // Created with 0 expiration (or negative in past)
        let now = chrono::Utc::now().timestamp() as usize - 100;
        let claims = Claims {
            sub: user_id,
            email: email.to_string(),
            role: UserRole::Patient,
            exp: now,
            iat: now - 3600,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let result = verify_jwt(&token, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_jwt_fails() {
        let secret = "super_secret_test_key_minimum_length_requirement";
        assert!(verify_jwt("not.a.valid.jwt.token", secret).is_err());
        assert!(verify_jwt("Bearer invalid_token", secret).is_err());
    }

    #[test]
    fn test_wrong_secret_fails() {
        let user_id = Uuid::new_v4();
        let email = "patient@demo.local";
        let token = create_jwt(
            user_id,
            email,
            UserRole::Patient,
            "correct_secret_key_12345",
            3600,
        )
        .unwrap();

        assert!(verify_jwt(&token, "wrong_secret_key_67890").is_err());
    }
}
