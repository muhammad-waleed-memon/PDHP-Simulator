//! Doctors module — provider profile management and registration.

pub mod dto;

use crate::auth::crypto::hash_password;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users::{User, UserRole, UserStatus};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use dto::{CreateDoctorRequest, DoctorResponse};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "doctor_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DoctorStatus {
    Pending,
    Verified,
    Suspended,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct Doctor {
    pub id: Uuid,
    pub user_id: Uuid,
    pub facility_id: Option<Uuid>,
    pub pmdc_license_number: Option<String>,
    pub specialization: String,
    pub qualification: Option<String>,
    pub status: DoctorStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, FromRow)]
struct DoctorWithUser {
    pub doctor_id: Uuid,
    pub user_id: Uuid,
    pub facility_id: Option<Uuid>,
    pub pmdc_license_number: Option<String>,
    pub specialization: String,
    pub qualification: Option<String>,
    pub status: DoctorStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub full_name: String,
    pub email: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(register_doctor).get(list_doctors))
        .route("/:id", get(get_doctor_by_id))
}

/// Register a new Doctor user and provider record in a single transaction.
pub async fn register_doctor(
    State(state): State<AppState>,
    Json(payload): Json<CreateDoctorRequest>,
) -> AppResult<Json<DoctorResponse>> {
    if payload.email.trim().is_empty() || payload.password.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Email and password are required".to_string(),
        ));
    }

    if payload.specialization.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Specialization is required".to_string(),
        ));
    }

    let pwd_hash = hash_password(&payload.password)?;

    let mut tx = state.db().begin().await?;

    // Check duplicate email
    let existing_user = sqlx::query("SELECT id FROM users WHERE email = $1")
        .bind(payload.email.trim())
        .fetch_optional(&mut *tx)
        .await?;

    if existing_user.is_some() {
        return Err(AppError::Conflict(
            "User with this email already exists".to_string(),
        ));
    }

    // Verify facility_id exists if provided
    if let Some(fac_id) = payload.facility_id {
        let fac_exists = sqlx::query("SELECT id FROM facilities WHERE id = $1")
            .bind(fac_id)
            .fetch_optional(&mut *tx)
            .await?;

        if fac_exists.is_none() {
            return Err(AppError::BadRequest(
                "Specified facility_id does not exist".to_string(),
            ));
        }
    }

    // 1. Create User
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
    .bind(UserRole::Doctor)
    .bind(UserStatus::Active)
    .fetch_one(&mut *tx)
    .await?;

    // 2. Create Doctor Record
    let doctor = sqlx::query_as::<_, Doctor>(
        r#"
        INSERT INTO doctors (user_id, facility_id, pmdc_license_number, specialization, qualification, status)
        VALUES ($1, $2, $3, $4, $5, 'PENDING'::doctor_status)
        RETURNING *
        "#
    )
    .bind(user.id)
    .bind(payload.facility_id)
    .bind(&payload.pmdc_license_number)
    .bind(&payload.specialization)
    .bind(&payload.qualification)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(DoctorResponse {
        id: doctor.id,
        user_id: user.id,
        facility_id: doctor.facility_id,
        full_name: user.full_name,
        email: user.email,
        pmdc_license_number: doctor.pmdc_license_number,
        specialization: doctor.specialization,
        qualification: doctor.qualification,
        status: doctor.status,
        created_at: doctor.created_at,
    }))
}

pub async fn list_doctors(State(state): State<AppState>) -> AppResult<Json<Vec<DoctorResponse>>> {
    let records = sqlx::query_as::<_, DoctorWithUser>(
        r#"
        SELECT d.id as doctor_id, d.user_id, d.facility_id, d.pmdc_license_number, d.specialization,
               d.qualification, d.status, d.created_at,
               u.full_name, u.email
        FROM doctors d
        JOIN users u ON d.user_id = u.id
        ORDER BY u.full_name ASC
        "#,
    )
    .fetch_all(state.db())
    .await?;

    let doctors = records
        .into_iter()
        .map(|r| DoctorResponse {
            id: r.doctor_id,
            user_id: r.user_id,
            facility_id: r.facility_id,
            full_name: r.full_name,
            email: r.email,
            pmdc_license_number: r.pmdc_license_number,
            specialization: r.specialization,
            qualification: r.qualification,
            status: r.status,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(doctors))
}

pub async fn get_doctor_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DoctorResponse>> {
    let r = sqlx::query_as::<_, DoctorWithUser>(
        r#"
        SELECT d.id as doctor_id, d.user_id, d.facility_id, d.pmdc_license_number, d.specialization,
               d.qualification, d.status, d.created_at,
               u.full_name, u.email
        FROM doctors d
        JOIN users u ON d.user_id = u.id
        WHERE d.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::NotFound("Doctor profile not found".to_string()))?;

    Ok(Json(DoctorResponse {
        id: r.doctor_id,
        user_id: r.user_id,
        facility_id: r.facility_id,
        full_name: r.full_name,
        email: r.email,
        pmdc_license_number: r.pmdc_license_number,
        specialization: r.specialization,
        qualification: r.qualification,
        status: r.status,
        created_at: r.created_at,
    }))
}
