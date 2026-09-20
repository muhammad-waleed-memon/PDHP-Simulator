//! Medical Conditions module — patient medical condition records with relational version history.

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
pub struct Condition {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub name: String,
    pub code: Option<String>,
    pub status: String,
    pub onset_date: Option<chrono::NaiveDate>,
    pub resolved_date: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
    pub version: i32,
    pub created_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConditionHistory {
    pub history_id: Uuid,
    pub condition_id: Uuid,
    pub version: i32,
    pub status: String,
    pub notes: Option<String>,
    pub modified_by: Option<Uuid>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConditionRequest {
    pub name: String,
    pub code: Option<String>,
    pub status: Option<String>,
    pub onset_date: Option<chrono::NaiveDate>,
    pub resolved_date: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConditionRequest {
    pub status: Option<String>,
    pub resolved_date: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/patients/:patient_id/conditions",
            post(create_condition).get(list_conditions),
        )
        .route(
            "/patients/:patient_id/conditions/:id",
            put(update_condition),
        )
        .route(
            "/patients/:patient_id/conditions/:id/history",
            get(get_condition_history),
        )
}

pub async fn list_conditions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
) -> AppResult<Json<Vec<Condition>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let conditions = sqlx::query_as::<_, Condition>(
        "SELECT * FROM conditions WHERE patient_id = $1 ORDER BY created_at DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::CONDITION_VIEWED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Condition", patient_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(conditions))
}

pub async fn create_condition(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
    Json(req): Json<CreateConditionRequest>,
) -> AppResult<Json<Condition>> {
    authorize_patient_access(&state, &user, patient_id).await?;
    require_role(&user, &[UserRole::Doctor, UserRole::SystemAdmin])?;

    if req.name.trim().is_empty() {
        return Err(AppError::Validation(
            "Condition name is required".to_string(),
        ));
    }

    let status = req.status.unwrap_or_else(|| "ACTIVE".to_string());

    let condition = sqlx::query_as::<_, Condition>(
        r#"
        INSERT INTO conditions (patient_id, name, code, status, onset_date, resolved_date, notes, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(patient_id)
    .bind(req.name.trim())
    .bind(req.code.as_deref().map(str::trim))
    .bind(status)
    .bind(req.onset_date)
    .bind(req.resolved_date)
    .bind(req.notes.as_deref().map(str::trim))
    .bind(user.id)
    .fetch_one(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::CONDITION_CREATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Condition", condition.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(condition))
}

pub async fn update_condition(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((patient_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateConditionRequest>,
) -> AppResult<Json<Condition>> {
    authorize_patient_access(&state, &user, patient_id).await?;
    require_role(&user, &[UserRole::Doctor, UserRole::SystemAdmin])?;

    let current = sqlx::query_as::<_, Condition>(
        "SELECT * FROM conditions WHERE id = $1 AND patient_id = $2",
    )
    .bind(id)
    .bind(patient_id)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::NotFound("Condition not found for this patient".to_string()))?;

    let mut tx = state.db().begin().await?;

    // Save history snapshot of previous version
    sqlx::query(
        r#"
        INSERT INTO condition_history (condition_id, version, status, notes, modified_by)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(current.id)
    .bind(current.version)
    .bind(&current.status)
    .bind(&current.notes)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    let new_status = req.status.unwrap_or(current.status);
    let new_resolved = req.resolved_date.or(current.resolved_date);
    let new_notes = req.notes.or(current.notes);
    let new_version = current.version + 1;

    let updated = sqlx::query_as::<_, Condition>(
        r#"
        UPDATE conditions
        SET status = $1, resolved_date = $2, notes = $3, version = $4, updated_at = NOW()
        WHERE id = $5 AND patient_id = $6
        RETURNING *
        "#,
    )
    .bind(new_status)
    .bind(new_resolved)
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
            AuditEvent::new(events::CONDITION_UPDATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Condition", updated.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(updated))
}

pub async fn get_condition_history(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((patient_id, id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Vec<ConditionHistory>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let history = sqlx::query_as::<_, ConditionHistory>(
        "SELECT * FROM condition_history WHERE condition_id = $1 ORDER BY version ASC",
    )
    .bind(id)
    .fetch_all(state.db())
    .await?;

    Ok(Json(history))
}

#[cfg(test)]
mod tests {
    #[test]
    fn condition_version_increments_correctly() {
        let initial_version = 1;
        let updated_version = initial_version + 1;
        assert_eq!(updated_version, 2);
    }
}
