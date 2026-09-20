//! Prescriptions module — doctor prescriptions, items, and safety alert workflow.

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
use crate::clinical_alerts::{evaluate_prescription_alerts, ClinicalAlert};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users::UserRole;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Prescription {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub doctor_id: Uuid,
    pub encounter_id: Option<Uuid>,
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub clinical_notes: Option<String>,
    pub override_reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PrescriptionItem {
    pub id: Uuid,
    pub prescription_id: Uuid,
    pub medication_id: Uuid,
    pub dosage: String,
    pub frequency: String,
    pub route: Option<String>,
    pub duration: String,
    pub quantity: i32,
    pub instructions: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct ItemWithMedRow {
    id: Uuid,
    prescription_id: Uuid,
    medication_id: Uuid,
    dosage: String,
    frequency: String,
    route: Option<String>,
    duration: String,
    quantity: i32,
    instructions: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    medication_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrescriptionItemDetail {
    pub item: PrescriptionItem,
    pub medication_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrescriptionWithDetails {
    pub prescription: Prescription,
    pub items: Vec<PrescriptionItemDetail>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePrescriptionItemRequest {
    pub medication_id: Uuid,
    pub dosage: String,
    pub frequency: String,
    pub route: Option<String>,
    pub duration: String,
    pub quantity: Option<i32>,
    pub instructions: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePrescriptionRequest {
    pub encounter_id: Option<Uuid>,
    pub clinical_notes: Option<String>,
    pub override_reason: Option<String>,
    pub items: Vec<CreatePrescriptionItemRequest>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct SafetyAlertResponse {
    pub message: String,
    pub alerts: Vec<ClinicalAlert>,
    pub override_required: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/patients/:patient_id/prescriptions",
            post(create_prescription).get(list_prescriptions),
        )
        .route("/prescriptions/:id", get(get_prescription))
}

pub async fn list_prescriptions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
) -> AppResult<Json<Vec<PrescriptionWithDetails>>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    let prescriptions = sqlx::query_as::<_, Prescription>(
        "SELECT * FROM prescriptions WHERE patient_id = $1 ORDER BY issued_at DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let mut result = Vec::new();
    for rx in prescriptions {
        let items_with_med = sqlx::query_as::<_, ItemWithMedRow>(
            r#"
            SELECT pi.id, pi.prescription_id, pi.medication_id, pi.dosage, pi.frequency,
                   pi.route, pi.duration, pi.quantity, pi.instructions, pi.created_at, pi.updated_at,
                   m.name as medication_name
            FROM prescription_items pi
            JOIN medications m ON m.id = pi.medication_id
            WHERE pi.prescription_id = $1
            "#,
        )
        .bind(rx.id)
        .fetch_all(state.db())
        .await?;

        let items = items_with_med
            .into_iter()
            .map(|row| PrescriptionItemDetail {
                item: PrescriptionItem {
                    id: row.id,
                    prescription_id: row.prescription_id,
                    medication_id: row.medication_id,
                    dosage: row.dosage,
                    frequency: row.frequency,
                    route: row.route,
                    duration: row.duration,
                    quantity: row.quantity,
                    instructions: row.instructions,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                },
                medication_name: row.medication_name,
            })
            .collect();

        result.push(PrescriptionWithDetails {
            prescription: rx,
            items,
        });
    }

    let audit = AuditService::new(state.db().clone());
    let _ = audit
        .record(
            AuditEvent::new(events::PRESCRIPTION_VIEWED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Prescription", patient_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(result))
}

pub async fn get_prescription(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PrescriptionWithDetails>> {
    let rx = sqlx::query_as::<_, Prescription>("SELECT * FROM prescriptions WHERE id = $1")
        .bind(id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Prescription not found".to_string()))?;

    authorize_patient_access(&state, &user, rx.patient_id).await?;

    let items_with_med = sqlx::query_as::<_, ItemWithMedRow>(
        r#"
        SELECT pi.id, pi.prescription_id, pi.medication_id, pi.dosage, pi.frequency,
               pi.route, pi.duration, pi.quantity, pi.instructions, pi.created_at, pi.updated_at,
               m.name as medication_name
        FROM prescription_items pi
        JOIN medications m ON m.id = pi.medication_id
        WHERE pi.prescription_id = $1
        "#,
    )
    .bind(rx.id)
    .fetch_all(state.db())
    .await?;

    let items = items_with_med
        .into_iter()
        .map(|row| PrescriptionItemDetail {
            item: PrescriptionItem {
                id: row.id,
                prescription_id: row.prescription_id,
                medication_id: row.medication_id,
                dosage: row.dosage,
                frequency: row.frequency,
                route: row.route,
                duration: row.duration,
                quantity: row.quantity,
                instructions: row.instructions,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            medication_name: row.medication_name,
        })
        .collect();

    Ok(Json(PrescriptionWithDetails {
        prescription: rx,
        items,
    }))
}

pub async fn create_prescription(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
    Json(req): Json<CreatePrescriptionRequest>,
) -> AppResult<Json<PrescriptionWithDetails>> {
    authorize_patient_access(&state, &user, patient_id).await?;

    // DOCTOR ONLY RESTRICTION
    require_role(&user, &[UserRole::Doctor])?;

    if req.items.is_empty() {
        return Err(AppError::Validation(
            "Prescription must contain at least one medication item".to_string(),
        ));
    }

    // Lookup Doctor ID for user.id
    let doctor_id: Uuid = sqlx::query_scalar("SELECT id FROM doctors WHERE user_id = $1")
        .bind(user.id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::Validation("Doctor profile not found for user".to_string()))?;

    let med_ids: Vec<Uuid> = req.items.iter().map(|item| item.medication_id).collect();

    // Evaluate Clinical Safety Alerts
    let alerts = evaluate_prescription_alerts(state.db(), patient_id, &med_ids).await?;

    let override_provided = req
        .override_reason
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some();

    if !alerts.is_empty() && !override_provided {
        let alert_msgs: Vec<String> = alerts.iter().map(|a| a.message.clone()).collect();
        return Err(AppError::Validation(format!(
            "CLINICAL SAFETY ALERT TRIGGERED: {}. Override reason required to proceed.",
            alert_msgs.join(" | ")
        )));
    }

    let audit = AuditService::new(state.db().clone());

    if !alerts.is_empty() && override_provided {
        let _ = audit
            .record(
                AuditEvent::new(events::CLINICAL_ALERT_OVERRIDDEN)
                    .actor(user.id)
                    .patient(patient_id)
                    .resource("Prescription", patient_id)
                    .outcome(OUTCOME_ALLOWED)
                    .reason(
                        req.override_reason
                            .as_deref()
                            .unwrap_or("Doctor overridden"),
                    ),
            )
            .await;
    }

    let mut tx = state.db().begin().await?;

    let prescription = sqlx::query_as::<_, Prescription>(
        r#"
        INSERT INTO prescriptions (patient_id, doctor_id, encounter_id, clinical_notes, override_reason)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(patient_id)
    .bind(doctor_id)
    .bind(req.encounter_id)
    .bind(req.clinical_notes.as_deref().map(str::trim))
    .bind(req.override_reason.as_deref().map(str::trim))
    .fetch_one(&mut *tx)
    .await?;

    let mut created_items = Vec::new();
    for item_req in req.items {
        let qty = item_req.quantity.unwrap_or(1);

        let item_row = sqlx::query_as::<_, PrescriptionItem>(
            r#"
            INSERT INTO prescription_items (prescription_id, medication_id, dosage, frequency, route, duration, quantity, instructions)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(prescription.id)
        .bind(item_req.medication_id)
        .bind(item_req.dosage.trim())
        .bind(item_req.frequency.trim())
        .bind(item_req.route.as_deref().map(str::trim))
        .bind(item_req.duration.trim())
        .bind(qty)
        .bind(item_req.instructions.as_deref().map(str::trim))
        .fetch_one(&mut *tx)
        .await?;

        let med_name: String = sqlx::query_scalar("SELECT name FROM medications WHERE id = $1")
            .bind(item_req.medication_id)
            .fetch_one(&mut *tx)
            .await?;

        // Add to active patient medications automatically upon prescription
        sqlx::query(
            r#"
            INSERT INTO patient_medications (patient_id, medication_id, prescribing_doctor_id, dosage, frequency, route, instructions, recorded_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(patient_id)
        .bind(item_req.medication_id)
        .bind(doctor_id)
        .bind(item_req.dosage.trim())
        .bind(item_req.frequency.trim())
        .bind(item_req.route.as_deref().map(str::trim))
        .bind(item_req.instructions.as_deref().map(str::trim))
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

        created_items.push(PrescriptionItemDetail {
            item: item_row,
            medication_name: med_name,
        });
    }

    tx.commit().await?;

    let _ = audit
        .record(
            AuditEvent::new(events::PRESCRIPTION_CREATED)
                .actor(user.id)
                .patient(patient_id)
                .resource("Prescription", prescription.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;

    Ok(Json(PrescriptionWithDetails {
        prescription,
        items: created_items,
    }))
}
