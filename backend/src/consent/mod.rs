//! Consent management module.
//!
//! Provides a [`ConsentService`] for checking, granting, and revoking
//! patient consent, plus Axum HTTP handlers for the consent API.
//!
//! # Consent Model
//!
//! A patient grants a specific provider (doctor/lab/facility-admin) access
//! to their medical record for an optional time window.
//!
//! - Only the patient themselves (or a system admin) may grant or revoke consent.
//! - Only one ACTIVE consent per (patient, provider) pair is allowed at a time.
//! - Revoking consent does not delete history — it transitions the record to REVOKED.
//! - Every grant/revoke action is audit-logged.

pub mod dto;
pub mod service;

use crate::auth::jwt::AuthenticatedUser;
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use dto::{ConsentGrantRequest, ConsentGrantResponse};
use uuid::Uuid;

/// Mount consent routes under `/api/v1/consent`.
pub fn router() -> Router<AppState> {
    Router::new()
        // Patient grants consent to a provider
        .route("/", post(handle_grant_consent))
        // Patient revokes consent for a specific grant
        .route("/:grant_id/revoke", delete(handle_revoke_consent))
        // List all consent grants for a patient (patient sees their own, admin sees all)
        .route(
            "/patient/:patient_id",
            get(handle_list_consents_for_patient),
        )
}

/// POST /api/v1/consent
///
/// Grant a provider access to a patient's record.
/// Only the patient user or a system admin may call this.
async fn handle_grant_consent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<ConsentGrantRequest>,
) -> AppResult<Json<ConsentGrantResponse>> {
    let grant = service::grant_consent(&state, &user, payload).await?;
    Ok(Json(grant))
}

/// DELETE /api/v1/consent/:grant_id/revoke
///
/// Revoke an existing consent grant.
/// Only the patient who owns the grant or a system admin may revoke it.
async fn handle_revoke_consent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(grant_id): Path<Uuid>,
) -> AppResult<Json<ConsentGrantResponse>> {
    let grant = service::revoke_consent(&state, &user, grant_id).await?;
    Ok(Json(grant))
}

/// GET /api/v1/consent/patient/:patient_id
///
/// List all consent grants (ACTIVE/REVOKED/EXPIRED) for a specific patient.
/// Only the patient themselves or a system admin may call this.
async fn handle_list_consents_for_patient(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(patient_id): Path<Uuid>,
) -> AppResult<Json<Vec<ConsentGrantResponse>>> {
    let grants = service::list_consents_for_patient(&state, &user, patient_id).await?;
    Ok(Json(grants))
}
