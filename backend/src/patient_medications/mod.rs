//! Patient Medication History module.

use axum::{
    extract::{Path, State},
    routing::{post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::audit::{events, AuditEvent, AuditService, OUTCOME_ALLOWED};
use crate::auth::jwt::AuthenticatedUser;
use crate::authorization::{authorize_patient_access, require_role};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users::UserRole;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PatientMedication {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub medication_id: Uuid,
    pub prescribing_doctor_id: Option<Uuid>,
    pub dosage: String,
    pub frequency: String,
    pub route: Option<String>,
    pub start_date: chrono::NaiveDate,
    pub end_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub instructions: Option<String>,
    pub reason: Option<String>,
    pub version: i32,
    pub recorded_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PatientMedicationHistory {
    pub history_id: Uuid,
    pub patient_medication_id: Uuid,
    pub version: i32,
    pub status: String,
    pub dosage: String,
    pub frequency: String,
    pub modified_by: Option<Uuid>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePatientMedicationRequest {
    pub medication_id: Uuid,
    pub prescribing_doctor_id: Option<Uuid>,
    pub dosage: String,
    pub frequency: String,
    pub route: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub status: Option<String>,
    pub instructions: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePatientMedicationRequest {
    pub dosage: Option<String>,
    pub frequency: Option<String>,
    pub status: Option<String>,
    pub end_date: Option<chrono::NaiveDate>,
    pub instructions: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/patients/:patient_id/medications",
            post(create_patient_medication).get(list_patient_medications),
        )
        .route(
            "/patients/:patient_id/medications/:id",
            put(update_patient_medication),
        )
}

pub async fn list_patient_medications(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
) -> AppResult<Json<Vec<PatientMedication>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let meds = sqlx::query_as::<_, PatientMedication>(
        "SELECT * FROM patient_medications WHERE patient_id = $1 ORDER BY start_date DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::PATIENT_MEDICATION_VIEWED)
                .actor(user.id)
                .patient(patient_id)
                .resource("PatientMedication", patient_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(meds))
}

pub async fn create_patient_medication(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
    Json(req): Json<CreatePatientMedicationRequest>,
) -> AppResult<Json<PatientMedication>> {
    authorize_patient_access(&state, &user, patient_id).await?;
    require_role(&user, &[UserRole::Doctor, UserRole::SystemAdmin])?;

    if req.dosage.trim().is_empty() || req.frequency.trim().is_empty() {
        return Err(AppError::Validation(
            "Dosage and frequency are required".to_string(),
        ));
    }

    let start_date = req
        .start_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let status = req.status.unwrap_or_else(|| "ACTIVE".to_string());

    let med = sqlx::query_as::<_, PatientMedication>(
        r#"
        INSERT INTO patient_medications (
            patient_id, medication_id, prescribing_doctor_id, dosage, frequency,
            route, start_date, end_date, status, instructions, reason, recorded_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING *
        "#,
    )
    .bind(patient_id)
    .bind(req.medication_id)
    .bind(req.prescribing_doctor_id)
    .bind(req.dosage.trim())
    .bind(req.frequency.trim())
    .bind(req.route.as_deref().map(str::trim))
    .bind(start_date)
    .bind(req.end_date)
    .bind(status)
    .bind(req.instructions.as_deref().map(str::trim))
    .bind(req.reason.as_deref().map(str::trim))
    .bind(user.id)
    .fetch_one(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::PATIENT_MEDICATION_CREATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("PatientMedication", med.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(med))
}

pub async fn update_patient_medication(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((patient_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdatePatientMedicationRequest>,
) -> AppResult<Json<PatientMedication>> {
    authorize_patient_access(&state, &user, patient_id).await?;
    require_role(&user, &[UserRole::Doctor, UserRole::SystemAdmin])?;

    let current = sqlx::query_as::<_, PatientMedication>(
        "SELECT * FROM patient_medications WHERE id = $1 AND patient_id = $2",
    )
    .bind(id)
    .bind(patient_id)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::NotFound("Patient medication record not found".to_string()))?;

    let mut tx = state.db().begin().await?;

    // Archive snapshot
    sqlx::query(
        r#"
        INSERT INTO patient_medication_history (patient_medication_id, version, status, dosage, frequency, modified_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(current.id)
    .bind(current.version)
    .bind(&current.status)
    .bind(&current.dosage)
    .bind(&current.frequency)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    let new_dosage = req.dosage.unwrap_or(current.dosage);
    let new_frequency = req.frequency.unwrap_or(current.frequency);
    let new_status = req.status.unwrap_or(current.status);
    let new_end_date = req.end_date.or(current.end_date);
    let new_instructions = req.instructions.or(current.instructions);
    let new_version = current.version + 1;

    let updated = sqlx::query_as::<_, PatientMedication>(
        r#"
        UPDATE patient_medications
        SET dosage = $1, frequency = $2, status = $3, end_date = $4, instructions = $5, version = $6, updated_at = NOW()
        WHERE id = $7 AND patient_id = $8
        RETURNING *
        "#,
    )
    .bind(new_dosage)
    .bind(new_frequency)
    .bind(new_status)
    .bind(new_end_date)
    .bind(new_instructions)
    .bind(new_version)
    .bind(id)
    .bind(patient_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::PATIENT_MEDICATION_UPDATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("PatientMedication", updated.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(updated))
}
