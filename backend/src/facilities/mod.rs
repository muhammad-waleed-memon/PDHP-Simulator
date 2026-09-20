//! Healthcare Facilities module.

use crate::auth::jwt::AuthenticatedUser;
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "facility_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FacilityStatus {
    Active,
    Inactive,
    Pending,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Facility {
    pub id: Uuid,
    pub name: String,
    pub facility_type: String,
    pub license_number: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub contact_number: Option<String>,
    pub status: FacilityStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFacilityRequest {
    pub name: String,
    pub facility_type: String,
    pub license_number: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub contact_number: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_facilities).post(create_facility))
        .route("/:id", get(get_facility_by_id))
}

pub async fn list_facilities(State(state): State<AppState>) -> AppResult<Json<Vec<Facility>>> {
    let facilities = sqlx::query_as::<_, Facility>("SELECT * FROM facilities ORDER BY name ASC")
        .fetch_all(state.db())
        .await?;

    Ok(Json(facilities))
}

pub async fn get_facility_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Facility>> {
    let facility = sqlx::query_as::<_, Facility>("SELECT * FROM facilities WHERE id = $1")
        .bind(id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Facility not found".to_string()))?;

    Ok(Json(facility))
}

pub async fn create_facility(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<CreateFacilityRequest>,
) -> AppResult<Json<Facility>> {
    crate::authorization::require_role(&user, &[crate::users::UserRole::FacilityAdmin, crate::users::UserRole::SystemAdmin])?;
    let facility = sqlx::query_as::<_, Facility>(
        r#"
        INSERT INTO facilities (name, facility_type, license_number, address, city, province, contact_number, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'ACTIVE')
        RETURNING *
        "#
    )
    .bind(&payload.name)
    .bind(&payload.facility_type)
    .bind(&payload.license_number)
    .bind(&payload.address)
    .bind(&payload.city)
    .bind(&payload.province)
    .bind(&payload.contact_number)
    .fetch_one(state.db())
    .await?;

    Ok(Json(facility))
}
