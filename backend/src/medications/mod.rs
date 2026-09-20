//! Medications module — medication catalog.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::jwt::AuthenticatedUser;
use crate::authorization::require_role;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users::UserRole;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Medication {
    pub id: Uuid,
    pub name: String,
    pub generic_name: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub route: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMedicationRequest {
    pub name: String,
    pub generic_name: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub route: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_medication).get(list_medications))
        .route("/:id", get(get_medication))
}

pub async fn list_medications(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> AppResult<Json<Vec<Medication>>> {
    let meds = sqlx::query_as::<_, Medication>(
        "SELECT * FROM medications WHERE status = 'ACTIVE' ORDER BY name ASC",
    )
    .fetch_all(state.db())
    .await?;

    Ok(Json(meds))
}

pub async fn get_medication(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Medication>> {
    let med = sqlx::query_as::<_, Medication>("SELECT * FROM medications WHERE id = $1")
        .bind(id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Medication not found".to_string()))?;

    Ok(Json(med))
}

pub async fn create_medication(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateMedicationRequest>,
) -> AppResult<Json<Medication>> {
    require_role(&user, &[UserRole::Doctor, UserRole::SystemAdmin])?;

    if req.name.trim().is_empty() {
        return Err(AppError::Validation(
            "Medication name is required".to_string(),
        ));
    }

    let med = sqlx::query_as::<_, Medication>(
        r#"
        INSERT INTO medications (name, generic_name, strength, dosage_form, route)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(req.name.trim())
    .bind(req.generic_name.as_deref().map(str::trim))
    .bind(req.strength.as_deref().map(str::trim))
    .bind(req.dosage_form.as_deref().map(str::trim))
    .bind(req.route.as_deref().map(str::trim))
    .fetch_one(state.db())
    .await?;

    Ok(Json(med))
}
