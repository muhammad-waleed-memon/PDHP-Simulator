//! Allergies module — patient allergy records with version history.

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
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
pub struct Allergy {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub allergen: String,
    pub reaction: Option<String>,
    pub severity: String,
    pub status: String,
    pub recorded_date: chrono::NaiveDate,
    pub recorded_by: Option<Uuid>,
    pub notes: Option<String>,
    pub version: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AllergyHistory {
    pub history_id: Uuid,
    pub allergy_id: Uuid,
    pub version: i32,
    pub status: String,
    pub severity: String,
    pub notes: Option<String>,
    pub modified_by: Option<Uuid>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAllergyRequest {
    pub allergen: String,
    pub reaction: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAllergyRequest {
    pub severity: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/patients/:patient_id/allergies",
            post(create_allergy).get(list_allergies),
        )
        .route("/patients/:patient_id/allergies/:id", put(update_allergy))
        .route(
            "/patients/:patient_id/allergies/:id/history",
            get(get_allergy_history),
        )
}

pub async fn list_allergies(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
) -> AppResult<Json<Vec<Allergy>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let allergies = sqlx::query_as::<_, Allergy>(
        "SELECT * FROM allergies WHERE patient_id = $1 ORDER BY recorded_date DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::ALLERGY_VIEWED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Allergy", patient_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(allergies))
}

pub async fn create_allergy(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
    Json(req): Json<CreateAllergyRequest>,
) -> AppResult<Json<Allergy>> {
    authorize_patient_access(&state, &user, patient_id).await?;
    require_role(&user, &[UserRole::Doctor, UserRole::SystemAdmin])?;

    if req.allergen.trim().is_empty() {
        return Err(AppError::Validation(
            "Allergen name is required".to_string(),
        ));
    }

    let severity = req.severity.unwrap_or_else(|| "MODERATE".to_string());
    let status = req.status.unwrap_or_else(|| "ACTIVE".to_string());

    let allergy = sqlx::query_as::<_, Allergy>(
        r#"
        INSERT INTO allergies (patient_id, allergen, reaction, severity, status, recorded_by, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(patient_id)
    .bind(req.allergen.trim())
    .bind(req.reaction.as_deref().map(str::trim))
    .bind(severity)
    .bind(status)
    .bind(user.id)
    .bind(req.notes.as_deref().map(str::trim))
    .fetch_one(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::ALLERGY_CREATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Allergy", allergy.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(allergy))
}

pub async fn update_allergy(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((patient_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateAllergyRequest>,
) -> AppResult<Json<Allergy>> {
    authorize_patient_access(&state, &user, patient_id).await?;
    require_role(&user, &[UserRole::Doctor, UserRole::SystemAdmin])?;

    let current =
        sqlx::query_as::<_, Allergy>("SELECT * FROM allergies WHERE id = $1 AND patient_id = $2")
            .bind(id)
            .bind(patient_id)
            .fetch_optional(state.db())
            .await?
            .ok_or_else(|| AppError::NotFound("Allergy record not found".to_string()))?;

    let mut tx = state.db().begin().await?;

    // Archive history
    sqlx::query(
        r#"
        INSERT INTO allergy_history (allergy_id, version, status, severity, notes, modified_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(current.id)
    .bind(current.version)
    .bind(&current.status)
    .bind(&current.severity)
    .bind(&current.notes)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    let new_severity = req.severity.unwrap_or(current.severity);
    let new_status = req.status.unwrap_or(current.status);
    let new_notes = req.notes.or(current.notes);
    let new_version = current.version + 1;

    let updated = sqlx::query_as::<_, Allergy>(
        r#"
        UPDATE allergies
        SET severity = $1, status = $2, notes = $3, version = $4, updated_at = NOW()
        WHERE id = $5 AND patient_id = $6
        RETURNING *
        "#,
    )
    .bind(new_severity)
    .bind(new_status)
    .bind(new_notes)
    .bind(new_version)
    .bind(id)
    .bind(patient_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::ALLERGY_UPDATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Allergy", updated.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(updated))
}

pub async fn get_allergy_history(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((patient_id, id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Vec<AllergyHistory>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let history = sqlx::query_as::<_, AllergyHistory>(
        "SELECT * FROM allergy_history WHERE allergy_id = $1 ORDER BY version ASC",
    )
    .bind(id)
    .fetch_all(state.db())
    .await?;

    Ok(Json(history))
}
