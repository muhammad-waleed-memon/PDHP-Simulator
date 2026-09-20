//! Authentication module — Argon2id password security, JWT, and Axum handlers.

pub mod crypto;
pub mod dhid;
pub mod dto;
pub mod jwt;
pub mod service;

use crate::auth::dto::{AuthResponse, LoginRequest, RegisterRequest, UserMeResponse};
use crate::auth::jwt::AuthenticatedUser;
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(handle_register))
        .route("/login", post(handle_login))
        .route("/me", get(handle_me))
}

pub async fn handle_register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    let res = service::register_user(&state, payload).await?;
    Ok(Json(res))
}

pub async fn handle_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let res = service::login_user(&state, payload).await?;
    Ok(Json(res))
}

pub async fn handle_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> AppResult<Json<UserMeResponse>> {
    let res = service::get_me(&state, user).await?;
    Ok(Json(res))
}
