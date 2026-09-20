//! Patients module — patient demographic profiles and Digital Health ID management.
//!
//! # Authorization (Phase 3)
//!
//! All patient data endpoints require authentication *and* explicit
//! authorization via [`crate::authorization::authorize_patient_access`].
//!
//! Being authenticated is never sufficient on its own.

pub mod dto;

use crate::audit::{events, AuditEvent, AuditService, OUTCOME_ALLOWED};
use crate::auth::crypto::hash_password;
use crate::auth::dhid::generate_digital_health_id;
use crate::auth::jwt::AuthenticatedUser;
use crate::authorization::authorize_patient_access;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users::{User, UserRole, UserStatus};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use dto::{CreatePatientRequest, PatientResponse};
use sqlx::FromRow;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(FromRow)]
struct IdRow {
    id: Uuid,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct Patient {
    pub id: Uuid,
    pub digital_health_id: String,
    pub user_id: Option<Uuid>,
    pub full_name: String,
    pub date_of_birth: chrono::NaiveDate,
    pub gender: String,
    pub blood_group: Option<String>,
    pub phone_number: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_phone: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(register_patient))
        .route("/me", get(get_my_patient_profile))
        .route("/:id", get(get_patient_by_id))
        .route("/by-digital-health-id/:dhid", get(get_patient_by_dhid))
}

pub async fn get_my_patient_profile(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> AppResult<Json<PatientResponse>> {
    let patient = sqlx::query_as::<_, Patient>("SELECT * FROM patients WHERE user_id = $1")
        .bind(user.id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("No patient profile associated with your user account".to_string()))?;

    Ok(Json(patient_to_response(patient)))
}

/// Register a new patient and generate a unique Digital Health ID.
///
/// # Authorization
///
/// This endpoint is restricted to `SystemAdmin` and `FacilityAdmin` roles.
/// A patient self-registration flow would be a separate endpoint in a later phase.
pub async fn register_patient(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<CreatePatientRequest>,
) -> AppResult<Json<PatientResponse>> {
    // Only system admins and facility admins may register patients
    crate::authorization::require_role(&user, &[UserRole::SystemAdmin, UserRole::FacilityAdmin])?;

    if payload.full_name.trim().is_empty() {
        return Err(AppError::BadRequest("Full name is required".to_string()));
    }

    if payload.gender.trim().is_empty() {
        return Err(AppError::BadRequest("Gender is required".to_string()));
    }

    // Generate unique Digital Health ID
    let dhid = generate_digital_health_id();

    let mut tx = state.db().begin().await?;

    // Optional user account creation
    let mut created_user_id: Option<Uuid> = None;

    if let (Some(email), Some(password)) = (&payload.email, &payload.password) {
        if !email.trim().is_empty() && !password.trim().is_empty() {
            // Check for existing email
            let existing = sqlx::query_as::<_, IdRow>("SELECT id FROM users WHERE email = $1")
                .bind(email.trim())
                .fetch_optional(&mut *tx)
                .await?;

            if existing.is_some() {
                return Err(AppError::Conflict(
                    "User with this email already exists".to_string(),
                ));
            }

            let pwd_hash = hash_password(password)?;

            let created_user = sqlx::query_as::<_, User>(
                r#"
                INSERT INTO users (email, password_hash, full_name, phone_number, role, status)
                VALUES ($1, $2, $3, $4, $5::user_role, $6::user_status)
                RETURNING *
                "#,
            )
            .bind(email.trim())
            .bind(&pwd_hash)
            .bind(&payload.full_name)
            .bind(&payload.phone_number)
            .bind(UserRole::Patient)
            .bind(UserStatus::Active)
            .fetch_one(&mut *tx)
            .await?;

            created_user_id = Some(created_user.id);
        }
    }

    // Create Patient record with generated DHID
    let patient = sqlx::query_as::<_, Patient>(
        r#"
        INSERT INTO patients (
            digital_health_id, user_id, full_name, date_of_birth, gender, blood_group,
            phone_number, address, city, province, emergency_contact_name, emergency_contact_phone, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'ACTIVE')
        RETURNING *
        "#
    )
    .bind(&dhid)
    .bind(created_user_id)
    .bind(&payload.full_name)
    .bind(payload.date_of_birth)
    .bind(&payload.gender)
    .bind(&payload.blood_group)
    .bind(&payload.phone_number)
    .bind(&payload.address)
    .bind(&payload.city)
    .bind(&payload.province)
    .bind(&payload.emergency_contact_name)
    .bind(&payload.emergency_contact_phone)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    // Audit log the creation
    let audit = AuditService::new(state.db().clone());
    if let Err(e) = audit
        .record(
            AuditEvent::new(events::PATIENT_RECORD_CREATED)
                .actor(user.id)
                .patient(patient.id)
                .resource("Patient", patient.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await
    {
        tracing::error!(error = %e, "Failed to write audit log for PATIENT_RECORD_CREATED");
    }

    Ok(Json(patient_to_response(patient)))
}

/// Retrieve patient record by UUID.
///
/// # Authorization
///
/// Requires: authentication + explicit authorization for this patient's record
/// (own record, active consent, or system admin).
pub async fn get_patient_by_id(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PatientResponse>> {
    // Explicit authorization check — authentication alone is not sufficient
    authorize_patient_access(&state, &user, id).await?;

    let patient = sqlx::query_as::<_, Patient>("SELECT * FROM patients WHERE id = $1")
        .bind(id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Patient record not found".to_string()))?;

    Ok(Json(patient_to_response(patient)))
}

/// Retrieve patient record by Digital Health ID.
///
/// # Authorization
///
/// Requires: authentication + explicit authorization for this patient's record.
/// The patient's UUID is resolved first, then authorization is enforced.
pub async fn get_patient_by_dhid(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(dhid): Path<String>,
) -> AppResult<Json<PatientResponse>> {
    // First resolve the patient — reveal nothing about existence before auth
    let patient =
        sqlx::query_as::<_, Patient>("SELECT * FROM patients WHERE digital_health_id = $1")
            .bind(dhid.trim())
            .fetch_optional(state.db())
            .await?
            .ok_or_else(|| {
                AppError::NotFound("Patient not found for specified Digital Health ID".to_string())
            })?;

    // Explicit authorization check against the resolved patient UUID
    authorize_patient_access(&state, &user, patient.id).await?;

    Ok(Json(patient_to_response(patient)))
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn patient_to_response(p: Patient) -> PatientResponse {
    PatientResponse {
        id: p.id,
        digital_health_id: p.digital_health_id,
        user_id: p.user_id,
        full_name: p.full_name,
        date_of_birth: p.date_of_birth,
        gender: p.gender,
        blood_group: p.blood_group,
        phone_number: p.phone_number,
        address: p.address,
        city: p.city,
        province: p.province,
        emergency_contact_name: p.emergency_contact_name,
        emergency_contact_phone: p.emergency_contact_phone,
        status: p.status,
        created_at: p.created_at,
    }
}
