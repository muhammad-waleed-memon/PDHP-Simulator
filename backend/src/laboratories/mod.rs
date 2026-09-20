//! Laboratories module — laboratory test reports and results.

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
pub struct LabReport {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub encounter_id: Option<Uuid>,
    pub laboratory_id: Option<Uuid>,
    pub test_name: String,
    pub result: String,
    pub unit: Option<String>,
    pub reference_range: Option<String>,
    pub abnormal_flag: bool,
    pub report_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub entered_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLabReportRequest {
    pub encounter_id: Option<Uuid>,
    pub laboratory_id: Option<Uuid>,
    pub test_name: String,
    pub result: String,
    pub unit: Option<String>,
    pub reference_range: Option<String>,
    pub abnormal_flag: Option<bool>,
    pub report_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/patients/:patient_id/lab-reports",
            post(create_lab_report).get(list_lab_reports),
        )
        .route("/patients/:patient_id/lab-reports/:id", get(get_lab_report))
}

pub async fn list_lab_reports(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
) -> AppResult<Json<Vec<LabReport>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let reports = sqlx::query_as::<_, LabReport>(
        "SELECT * FROM lab_reports WHERE patient_id = $1 ORDER BY report_date DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::LAB_REPORT_VIEWED)
                .actor(user.id)
                .patient(patient_id)
                .resource("LabReport", patient_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(reports))
}

pub async fn get_lab_report(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((patient_id, id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<LabReport>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let report = sqlx::query_as::<_, LabReport>(
        "SELECT * FROM lab_reports WHERE id = $1 AND patient_id = $2",
    )
    .bind(id)
    .bind(patient_id)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::NotFound("Lab report not found".to_string()))?;

    Ok(Json(report))
}

pub async fn create_lab_report(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
    Json(req): Json<CreateLabReportRequest>,
) -> AppResult<Json<LabReport>> {
    authorize_patient_access(&state, &user, patient_id).await?;
    require_role(
        &user,
        &[UserRole::Doctor, UserRole::Lab, UserRole::SystemAdmin],
    )?;

    if req.test_name.trim().is_empty() || req.result.trim().is_empty() {
        return Err(AppError::Validation(
            "Test name and result are required".to_string(),
        ));
    }

    let report_date = req.report_date.unwrap_or_else(chrono::Utc::now);
    let status = req.status.unwrap_or_else(|| "FINAL".to_string());
    let abnormal_flag = req.abnormal_flag.unwrap_or(false);

    let report = sqlx::query_as::<_, LabReport>(
        r#"
        INSERT INTO lab_reports (
            patient_id, encounter_id, laboratory_id, test_name, result,
            unit, reference_range, abnormal_flag, report_date, status, entered_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#,
    )
    .bind(patient_id)
    .bind(req.encounter_id)
    .bind(req.laboratory_id)
    .bind(req.test_name.trim())
    .bind(req.result.trim())
    .bind(req.unit.as_deref().map(str::trim))
    .bind(req.reference_range.as_deref().map(str::trim))
    .bind(abnormal_flag)
    .bind(report_date)
    .bind(status)
    .bind(user.id)
    .fetch_one(state.db())
    .await?;

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::LAB_REPORT_CREATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("LabReport", report.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(report))
}
