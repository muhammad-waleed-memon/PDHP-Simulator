//! Longitudinal Patient Timeline module — aggregates clinical timeline events.

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::audit::{events, AuditEvent, AuditService, OUTCOME_ALLOWED};
use crate::auth::jwt::AuthenticatedUser;
use crate::authorization::authorize_patient_access;
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItem {
    pub id: Uuid,
    pub event_type: String, // "ENCOUNTER", "CONDITION", "ALLERGY", "MEDICATION", "PRESCRIPTION", "LAB_REPORT"
    pub title: String,
    pub description: String,
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(FromRow)]
struct EncounterRow {
    id: Uuid,
    encounter_type: String,
    reason: String,
    status: String,
    start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow)]
struct ConditionRow {
    id: Uuid,
    name: String,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow)]
struct AllergyRow {
    id: Uuid,
    allergen: String,
    severity: String,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow)]
struct PrescriptionRow {
    id: Uuid,
    status: String,
    issued_at: chrono::DateTime<chrono::Utc>,
    clinical_notes: Option<String>,
}

#[derive(FromRow)]
struct LabReportRow {
    id: Uuid,
    test_name: String,
    result: String,
    unit: Option<String>,
    abnormal_flag: bool,
    report_date: chrono::DateTime<chrono::Utc>,
    status: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/patients/:patient_id/timeline", get(get_patient_timeline))
}

pub async fn get_patient_timeline(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
) -> AppResult<Json<Vec<TimelineItem>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let mut timeline: Vec<TimelineItem> = Vec::new();

    // 1. Encounters
    let encounters = sqlx::query_as::<_, EncounterRow>(
        "SELECT id, encounter_type, reason, status, start_time FROM encounters WHERE patient_id = $1",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    for enc in encounters {
        timeline.push(TimelineItem {
            id: enc.id,
            event_type: "ENCOUNTER".to_string(),
            title: format!("Encounter ({})", enc.encounter_type),
            description: enc.reason,
            status: enc.status,
            timestamp: enc.start_time,
            metadata: None,
        });
    }

    // 2. Conditions
    let conditions = sqlx::query_as::<_, ConditionRow>(
        "SELECT id, name, status, created_at FROM conditions WHERE patient_id = $1",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    for cond in conditions {
        timeline.push(TimelineItem {
            id: cond.id,
            event_type: "CONDITION".to_string(),
            title: format!("Condition: {}", cond.name),
            description: format!("Status: {}", cond.status),
            status: cond.status,
            timestamp: cond.created_at,
            metadata: None,
        });
    }

    // 3. Allergies
    let allergies = sqlx::query_as::<_, AllergyRow>(
        "SELECT id, allergen, severity, status, created_at FROM allergies WHERE patient_id = $1",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    for allergy in allergies {
        timeline.push(TimelineItem {
            id: allergy.id,
            event_type: "ALLERGY".to_string(),
            title: format!("Allergy Recorded: {}", allergy.allergen),
            description: format!("Severity: {}", allergy.severity),
            status: allergy.status,
            timestamp: allergy.created_at,
            metadata: None,
        });
    }

    // 4. Prescriptions
    let prescriptions = sqlx::query_as::<_, PrescriptionRow>(
        "SELECT id, status, issued_at, clinical_notes FROM prescriptions WHERE patient_id = $1",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    for rx in prescriptions {
        timeline.push(TimelineItem {
            id: rx.id,
            event_type: "PRESCRIPTION".to_string(),
            title: "Prescription Issued".to_string(),
            description: rx
                .clinical_notes
                .unwrap_or_else(|| "Medication prescription".to_string()),
            status: rx.status,
            timestamp: rx.issued_at,
            metadata: None,
        });
    }

    // 5. Lab Reports
    let lab_reports = sqlx::query_as::<_, LabReportRow>(
        "SELECT id, test_name, result, unit, abnormal_flag, report_date, status FROM lab_reports WHERE patient_id = $1",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    for lab in lab_reports {
        let flag_text = if lab.abnormal_flag { " [ABNORMAL]" } else { "" };
        timeline.push(TimelineItem {
            id: lab.id,
            event_type: "LAB_REPORT".to_string(),
            title: format!("Lab Result: {}{}", lab.test_name, flag_text),
            description: format!(
                "Result: {} {}",
                lab.result,
                lab.unit.as_deref().unwrap_or("")
            ),
            status: lab.status,
            timestamp: lab.report_date,
            metadata: None,
        });
    }

    // Sort descending chronologically
    timeline.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::TIMELINE_VIEWED)
                .actor(user.id)
                .patient(patient_id)
                .resource("PatientTimeline", patient_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(timeline))
}
