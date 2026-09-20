//! Authentication service logic.
//!
//! Login and registration events are audit-logged via [`AuditService`].
//! Passwords and password hashes are NEVER logged.

use crate::audit::{events, AuditEvent, AuditService, OUTCOME_ALLOWED, OUTCOME_DENIED};
use crate::auth::crypto::{hash_password, verify_password};
use crate::auth::dto::{AuthResponse, AuthUserInfo, LoginRequest, RegisterRequest, UserMeResponse};
use crate::auth::jwt::{create_jwt, AuthenticatedUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users::{User, UserStatus};
use sqlx::FromRow;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(FromRow)]
struct IdRow {
    id: Uuid,
}

/// Register a new user account with Argon2id password hash and return JWT auth response.
pub async fn register_user(state: &AppState, payload: RegisterRequest) -> AppResult<AuthResponse> {
    if payload.email.trim().is_empty() || payload.password.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Email and password are required".to_string(),
        ));
    }

    if payload.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    // Check duplicate user
    let existing = sqlx::query_as::<_, IdRow>("SELECT id FROM users WHERE email = $1")
        .bind(payload.email.trim())
        .fetch_optional(state.db())
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "An account with this email already exists".to_string(),
        ));
    }

    // Hash password securely (never logged)
    let pwd_hash = hash_password(&payload.password)?;

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, full_name, phone_number, role, status)
        VALUES ($1, $2, $3, $4, $5::user_role, $6::user_status)
        RETURNING *
        "#,
    )
    .bind(payload.email.trim())
    .bind(&pwd_hash)
    .bind(&payload.full_name)
    .bind(&payload.phone_number)
    .bind(&payload.role)
    .bind(UserStatus::Active)
    .fetch_one(state.db())
    .await?;

    // Audit: registration (no patient_id, no sensitive data)
    let audit = AuditService::new(state.db().clone());
    if let Err(e) = audit
        .record(
            AuditEvent::new(events::REGISTER)
                .actor(user.id)
                .resource("User", user.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await
    {
        tracing::error!(error = %e, "Failed to write audit log for REGISTER");
    }

    let token = create_jwt(
        user.id,
        &user.email,
        user.role.clone(),
        &state.config().jwt_secret,
        state.config().jwt_expires_in_seconds,
    )?;

    Ok(AuthResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: state.config().jwt_expires_in_seconds,
        user: AuthUserInfo {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            role: user.role,
            status: user.status,
        },
    })
}

/// Login with user credentials and generate JWT token.
pub async fn login_user(state: &AppState, payload: LoginRequest) -> AppResult<AuthResponse> {
    let email = payload.email.trim();
    let audit = AuditService::new(state.db().clone());

    let user_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(state.db())
        .await?;

    let user = match user_result {
        Some(u) => u,
        None => {
            // Audit: failed login (no user id available)
            if let Err(e) = audit
                .record(
                    AuditEvent::new(events::LOGIN_FAILURE)
                        .resource("User", email)
                        .outcome(OUTCOME_DENIED)
                        .reason("User not found"),
                )
                .await
            {
                tracing::error!(error = %e, "Failed to write audit log for LOGIN_FAILURE");
            }
            return Err(AppError::InvalidCredentials);
        }
    };

    // Verify password hash — NEVER log the password or hash
    let valid = verify_password(&payload.password, &user.password_hash)?;
    if !valid {
        if let Err(e) = audit
            .record(
                AuditEvent::new(events::LOGIN_FAILURE)
                    .actor(user.id)
                    .resource("User", user.id)
                    .outcome(OUTCOME_DENIED)
                    .reason("Invalid password"),
            )
            .await
        {
            tracing::error!(error = %e, "Failed to write audit log for LOGIN_FAILURE");
        }
        return Err(AppError::InvalidCredentials);
    }

    if user.status != UserStatus::Active {
        if let Err(e) = audit
            .record(
                AuditEvent::new(events::LOGIN_FAILURE)
                    .actor(user.id)
                    .resource("User", user.id)
                    .outcome(OUTCOME_DENIED)
                    .reason("Account inactive or suspended"),
            )
            .await
        {
            tracing::error!(error = %e, "Failed to write audit log for LOGIN_FAILURE");
        }
        return Err(AppError::Forbidden(
            "Account is inactive or suspended".to_string(),
        ));
    }

    // Audit: successful login
    if let Err(e) = audit
        .record(
            AuditEvent::new(events::LOGIN_SUCCESS)
                .actor(user.id)
                .resource("User", user.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await
    {
        tracing::error!(error = %e, "Failed to write audit log for LOGIN_SUCCESS");
    }

    let token = create_jwt(
        user.id,
        &user.email,
        user.role.clone(),
        &state.config().jwt_secret,
        state.config().jwt_expires_in_seconds,
    )?;

    Ok(AuthResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: state.config().jwt_expires_in_seconds,
        user: AuthUserInfo {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            role: user.role,
            status: user.status,
        },
    })
}

/// Get profile information of currently authenticated user.
pub async fn get_me(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> AppResult<UserMeResponse> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(authenticated_user.id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("User record not found".to_string()))?;

    Ok(UserMeResponse {
        id: user.id,
        email: user.email,
        full_name: user.full_name,
        role: user.role,
        status: user.status,
        created_at: user.created_at,
    })
}
