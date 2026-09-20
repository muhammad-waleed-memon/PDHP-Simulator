//! Consent business logic.
//!
//! All consent mutations are audit-logged. Errors from the audit service are
//! logged at ERROR level and surfaced to the caller — audit failures are never
//! silently ignored.

use crate::audit::{events, AuditEvent, AuditService, OUTCOME_ALLOWED, OUTCOME_DENIED};
use crate::auth::jwt::AuthenticatedUser;
use crate::consent::dto::{ConsentGrantRequest, ConsentGrantResponse};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users::UserRole;
use sqlx::FromRow;
use uuid::Uuid;

// ── Internal DB row ────────────────────────────────────────────────────────────

#[derive(Debug, FromRow)]
struct ConsentGrantRow {
    id: Uuid,
    patient_id: Uuid,
    granted_to_user_id: Uuid,
    status: String,
    granted_at: chrono::DateTime<chrono::Utc>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    purpose: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ConsentGrantRow> for ConsentGrantResponse {
    fn from(r: ConsentGrantRow) -> Self {
        Self {
            id: r.id,
            patient_id: r.patient_id,
            granted_to_user_id: r.granted_to_user_id,
            status: r.status,
            granted_at: r.granted_at,
            expires_at: r.expires_at,
            revoked_at: r.revoked_at,
            purpose: r.purpose,
            created_at: r.created_at,
        }
    }
}

// ── Service functions ──────────────────────────────────────────────────────────

/// Grant a provider access to a patient's record.
///
/// # Authorization
///
/// Only the patient (whose `user_id` matches the patient record's `user_id`)
/// or a system admin may grant consent.
pub async fn grant_consent(
    state: &AppState,
    actor: &AuthenticatedUser,
    payload: ConsentGrantRequest,
) -> AppResult<ConsentGrantResponse> {
    let audit = AuditService::new(state.db().clone());

    // ── 1. Resolve the patient record ────────────────────────────────────────
    let patient_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM patients WHERE id = $1")
            .bind(payload.patient_id)
            .fetch_optional(state.db())
            .await?
            .flatten();

    // ── 2. Authorization check ───────────────────────────────────────────────
    let is_system_admin = actor.role == UserRole::SystemAdmin;
    let is_the_patient = patient_user_id.map(|uid| uid == actor.id).unwrap_or(false);

    if !is_system_admin && !is_the_patient {
        audit_deny(
            &audit,
            actor.id,
            payload.patient_id,
            events::CONSENT_GRANTED,
            "Only the patient or system admin may grant consent",
        )
        .await;
        return Err(AppError::Forbidden(
            "Only the patient or a system administrator may grant consent".to_string(),
        ));
    }

    // ── 3. Validate: target provider user exists ─────────────────────────────
    let provider_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(payload.granted_to_user_id)
            .fetch_one(state.db())
            .await?;

    if !provider_exists {
        return Err(AppError::BadRequest(
            "Specified provider user does not exist".to_string(),
        ));
    }

    // ── 4. Check for existing ACTIVE consent (idempotency guard) ─────────────
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM consent_grants
         WHERE patient_id = $1 AND granted_to_user_id = $2 AND status = 'ACTIVE'",
    )
    .bind(payload.patient_id)
    .bind(payload.granted_to_user_id)
    .fetch_optional(state.db())
    .await?;

    if let Some(existing_id) = existing {
        return Err(AppError::Conflict(format!(
            "An active consent grant already exists (id: {existing_id}). Revoke it first."
        )));
    }

    // ── 5. Insert consent grant ───────────────────────────────────────────────
    let row: ConsentGrantRow = sqlx::query_as(
        r#"
        INSERT INTO consent_grants (patient_id, granted_to_user_id, expires_at, purpose, status)
        VALUES ($1, $2, $3, $4, 'ACTIVE')
        RETURNING id, patient_id, granted_to_user_id, status::text AS status, granted_at, expires_at, revoked_at, purpose, created_at
        "#,
    )
    .bind(payload.patient_id)
    .bind(payload.granted_to_user_id)
    .bind(payload.expires_at)
    .bind(&payload.purpose)
    .fetch_one(state.db())
    .await?;

    // ── 6. Audit ──────────────────────────────────────────────────────────────
    if let Err(e) = audit
        .record(
            AuditEvent::new(events::CONSENT_GRANTED)
                .actor(actor.id)
                .patient(payload.patient_id)
                .resource("ConsentGrant", row.id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await
    {
        tracing::error!(error = %e, "Failed to write audit log for CONSENT_GRANTED");
        return Err(AppError::internal(format!("Audit write failed: {e}")));
    }

    tracing::info!(
        grant_id = %row.id,
        patient_id = %payload.patient_id,
        granted_to = %payload.granted_to_user_id,
        "Consent granted"
    );

    Ok(row.into())
}

/// Revoke an existing consent grant.
///
/// # Authorization
///
/// Only the patient who owns the grant, or a system admin, may revoke it.
pub async fn revoke_consent(
    state: &AppState,
    actor: &AuthenticatedUser,
    grant_id: Uuid,
) -> AppResult<ConsentGrantResponse> {
    let audit = AuditService::new(state.db().clone());

    // ── 1. Fetch the grant ────────────────────────────────────────────────────
    let row: Option<ConsentGrantRow> = sqlx::query_as("SELECT id, patient_id, granted_to_user_id, status::text AS status, granted_at, expires_at, revoked_at, purpose, created_at FROM consent_grants WHERE id = $1")
        .bind(grant_id)
        .fetch_optional(state.db())
        .await?;

    let grant = row.ok_or_else(|| AppError::NotFound("Consent grant not found".to_string()))?;

    // ── 2. Authorization check ────────────────────────────────────────────────
    let patient_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM patients WHERE id = $1")
            .bind(grant.patient_id)
            .fetch_optional(state.db())
            .await?
            .flatten();

    let is_system_admin = actor.role == UserRole::SystemAdmin;
    let is_the_patient = patient_user_id.map(|uid| uid == actor.id).unwrap_or(false);

    if !is_system_admin && !is_the_patient {
        audit_deny(
            &audit,
            actor.id,
            grant.patient_id,
            events::CONSENT_REVOKED,
            "Only the patient or system admin may revoke consent",
        )
        .await;
        return Err(AppError::Forbidden(
            "Only the patient or a system administrator may revoke consent".to_string(),
        ));
    }

    // ── 3. Check it is still ACTIVE ───────────────────────────────────────────
    if grant.status != "ACTIVE" {
        return Err(AppError::BadRequest(format!(
            "Consent grant is already {} and cannot be revoked again",
            grant.status
        )));
    }

    // ── 4. Transition to REVOKED ──────────────────────────────────────────────
    let updated: ConsentGrantRow = sqlx::query_as(
        r#"
        UPDATE consent_grants
        SET status = 'REVOKED'::consent_status, revoked_at = NOW(), updated_at = NOW()
        WHERE id = $1
        RETURNING id, patient_id, granted_to_user_id, status::text AS status, granted_at, expires_at, revoked_at, purpose, created_at
        "#,
    )
    .bind(grant_id)
    .fetch_one(state.db())
    .await?;

    // ── 5. Audit ──────────────────────────────────────────────────────────────
    if let Err(e) = audit
        .record(
            AuditEvent::new(events::CONSENT_REVOKED)
                .actor(actor.id)
                .patient(grant.patient_id)
                .resource("ConsentGrant", grant_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await
    {
        tracing::error!(error = %e, "Failed to write audit log for CONSENT_REVOKED");
        return Err(AppError::internal(format!("Audit write failed: {e}")));
    }

    tracing::info!(
        grant_id = %grant_id,
        patient_id = %grant.patient_id,
        "Consent revoked"
    );

    Ok(updated.into())
}

/// List all consent grants for a patient.
///
/// # Authorization
///
/// Only the patient themselves or a system admin may call this.
pub async fn list_consents_for_patient(
    state: &AppState,
    actor: &AuthenticatedUser,
    patient_id: Uuid,
) -> AppResult<Vec<ConsentGrantResponse>> {
    // Resolve patient owner
    let patient_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM patients WHERE id = $1")
            .bind(patient_id)
            .fetch_optional(state.db())
            .await?
            .flatten();

    let is_system_admin = actor.role == UserRole::SystemAdmin;
    let is_the_patient = patient_user_id.map(|uid| uid == actor.id).unwrap_or(false);

    if !is_system_admin && !is_the_patient {
        return Err(AppError::Forbidden(
            "Only the patient or a system administrator may view consent grants".to_string(),
        ));
    }

    let rows: Vec<ConsentGrantRow> = sqlx::query_as(
        "SELECT id, patient_id, granted_to_user_id, status::text AS status, granted_at, expires_at, revoked_at, purpose, created_at FROM consent_grants WHERE patient_id = $1 ORDER BY granted_at DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Check whether a user has active consent to access a patient's record.
///
/// This is the core authorization gate called by other services.
/// Returns `Ok(())` if access is permitted, `Err(AppError::ConsentRequired)` otherwise.
///
/// Does NOT log — callers should log the outcome themselves via `AuditService`.
pub async fn check_consent(
    state: &AppState,
    provider_user_id: Uuid,
    patient_id: Uuid,
) -> AppResult<()> {
    let now = chrono::Utc::now();

    let active: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id FROM consent_grants
        WHERE patient_id = $1
          AND granted_to_user_id = $2
          AND status = 'ACTIVE'
          AND (expires_at IS NULL OR expires_at > $3)
        LIMIT 1
        "#,
    )
    .bind(patient_id)
    .bind(provider_user_id)
    .bind(now)
    .fetch_optional(state.db())
    .await?;

    if active.is_some() {
        Ok(())
    } else {
        Err(AppError::ConsentRequired)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Record an access-denied audit event without propagating the audit error.
/// Audit write failures are logged at ERROR but do not mask the original Forbidden.
async fn audit_deny(
    audit: &AuditService,
    actor_id: Uuid,
    patient_id: Uuid,
    event_type: &'static str,
    reason: &'static str,
) {
    if let Err(e) = audit
        .record(
            AuditEvent::new(event_type)
                .actor(actor_id)
                .patient(patient_id)
                .outcome(OUTCOME_DENIED)
                .reason(reason),
        )
        .await
    {
        tracing::error!(error = %e, event_type, "Failed to write denied-access audit log");
    }
}
