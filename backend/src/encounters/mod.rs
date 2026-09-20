//! Encounters module — clinical encounters/visits.

use axum::{
    extract::{Path, State},
    routing::{get, post},
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
pub struct Encounter {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub doctor_id: Uuid,
    pub facility_id: Uuid,
    pub encounter_type: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub reason: String,
    pub clinical_notes: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEncounterRequest {
    pub facility_id: Uuid,
    pub encounter_type: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub reason: String,
    pub clinical_notes: Option<String>,
    pub status: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/patients/:patient_id/encounters",
            post(create_encounter).get(list_encounters),
        )
        .route("/patients/:patient_id/encounters/:id", get(get_encounter))
}

pub async fn list_encounters(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
) -> AppResult<Json<Vec<Encounter>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let encounters = sqlx::query_as::<_, Encounter>(
        "SELECT * FROM encounters WHERE patient_id = $1 ORDER BY start_time DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::ENCOUNTER_VIEWED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Encounter", patient_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(encounters))
}

pub async fn get_encounter(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((patient_id, id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Encounter>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let encounter = sqlx::query_as::<_, Encounter>(
        "SELECT * FROM encounters WHERE id = $1 AND patient_id = $2",
    )
    .bind(id)
    .bind(patient_id)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::NotFound("Encounter not found".to_string()))?;

    Ok(Json(encounter))
}

pub async fn create_encounter(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
    Json(req): Json<CreateEncounterRequest>,
) -> AppResult<Json<Encounter>> {
    authorize_patient_access(&state, &user, patient_id).await?;
    require_role(&user, &[UserRole::Doctor, UserRole::SystemAdmin])?;

    if req.reason.trim().is_empty() {
        return Err(AppError::Validation(
            "Chief complaint/reason is required".to_string(),
        ));
    }

    // Lookup doctor profile for user.id
    let doctor_id: Uuid = if user.role == UserRole::Doctor {
        sqlx::query_scalar("SELECT id FROM doctors WHERE user_id = $1")
            .bind(user.id)
            .fetch_optional(state.db())
            .await?
            .ok_or_else(|| AppError::Validation("Doctor profile not found for user".to_string()))?
    } else {
        // Fallback for SystemAdmin testing: look up first doctor or create virtual UUID
        sqlx::query_scalar("SELECT id FROM doctors LIMIT 1")
            .fetch_optional(state.db())
            .await?
            .ok_or_else(|| AppError::Validation("No doctor available in system".to_string()))?
    };

    let encounter_type = req
        .encounter_type
        .unwrap_or_else(|| "OUTPATIENT".to_string());
    let start_time = req.start_time.unwrap_or_else(chrono::Utc::now);
    let status = req.status.unwrap_or_else(|| "COMPLETED".to_string());

    let encounter = sqlx::query_as::<_, Encounter>(
        r#"
        INSERT INTO encounters (patient_id, doctor_id, facility_id, encounter_type, start_time, end_time, reason, clinical_notes, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(patient_id)
    .bind(doctor_id)
    .bind(req.facility_id)
    .bind(encounter_type)
    .bind(start_time)
    .bind(req.end_time)
    .bind(req.reason.trim())
    .bind(req.clinical_notes.as_deref().map(str::trim))
    .bind(status)
    .fetch_one(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::ENCOUNTER_CREATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Encounter", encounter.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(encounter))
}
