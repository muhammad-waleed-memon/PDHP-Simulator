//! Centralized authorization policy enforcer.
//!
//! # Principle
//!
//! Authentication proves *who you are*.
//! Authorization proves *what you are allowed to do*.
//!
//! These are distinct checks.  Being authenticated is **not** sufficient
//! to access patient data.  Every handler that touches patient-specific
//! data must call into this module and receive an explicit `Ok(())` before
//! proceeding.
//!
//! # Role Hierarchy
//!
//! ```text
//! SystemAdmin   — unrestricted access (all patients, all records)
//! FacilityAdmin — no patient data access unless explicit consent exists
//! Doctor        — access only to patients who have granted them consent
//! Lab           — access only to patients who have granted them consent
//! Patient       — access only to their own record
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! # async fn example() {
//! use crate::authorization::authorize_patient_access;
//! // In a handler:
//! authorize_patient_access(&state, &user, patient_id).await?;
//! // Now safe to proceed with reading patient data
//! # }
//! ```

use crate::audit::{events, AuditEvent, AuditService, OUTCOME_ALLOWED, OUTCOME_DENIED};
use crate::consent::service::check_consent;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users::UserRole;
use uuid::Uuid;

use crate::auth::jwt::AuthenticatedUser;

/// Authorize access to a specific patient's record.
///
/// # Policy
///
/// | Role          | Access allowed when…                                      |
/// |---------------|-----------------------------------------------------------|
/// | SystemAdmin   | Always                                                    |
/// | Patient       | `actor.id == patient.user_id`                             |
/// | Doctor / Lab  | Active consent grant exists for this patient              |
/// | FacilityAdmin | Active consent grant exists for this patient              |
///
/// # Errors
///
/// Returns `AppError::Forbidden` or `AppError::ConsentRequired` if access is
/// denied.  The denial is audit-logged before returning.
pub async fn authorize_patient_access(
    state: &AppState,
    actor: &AuthenticatedUser,
    patient_id: Uuid,
) -> AppResult<()> {
    let audit = AuditService::new(state.db().clone());

    // ── System Admin: unrestricted ─────────────────────────────────────────
    if actor.role == UserRole::SystemAdmin {
        record_allow(
            &audit,
            actor.id,
            patient_id,
            events::PATIENT_RECORD_ACCESSED,
        )
        .await;
        return Ok(());
    }

    // ── Patient: own record only ───────────────────────────────────────────
    if actor.role == UserRole::Patient {
        let owns: bool = check_patient_ownership(state, actor.id, patient_id).await?;
        if owns {
            record_allow(
                &audit,
                actor.id,
                patient_id,
                events::PATIENT_RECORD_ACCESSED,
            )
            .await;
            return Ok(());
        }
        record_deny(
            &audit,
            actor.id,
            patient_id,
            events::ACCESS_DENIED,
            "Patient may only access their own record",
        )
        .await;
        return Err(AppError::Forbidden(
            "You may only access your own patient record".to_string(),
        ));
    }

    // ── Doctor / Lab / FacilityAdmin: consent required ────────────────────
    if matches!(
        actor.role,
        UserRole::Doctor | UserRole::Lab | UserRole::FacilityAdmin
    ) {
        match check_consent(state, actor.id, patient_id).await {
            Ok(()) => {
                record_allow(
                    &audit,
                    actor.id,
                    patient_id,
                    events::PATIENT_RECORD_ACCESSED,
                )
                .await;
                return Ok(());
            }
            Err(AppError::ConsentRequired) => {
                record_deny(
                    &audit,
                    actor.id,
                    patient_id,
                    events::ACCESS_DENIED,
                    "No active consent grant found",
                )
                .await;
                return Err(AppError::ConsentRequired);
            }
            Err(e) => return Err(e),
        }
    }

    // ── Catch-all: deny unknown roles ─────────────────────────────────────
    record_deny(
        &audit,
        actor.id,
        patient_id,
        events::ACCESS_DENIED,
        "Role not permitted to access patient records",
    )
    .await;
    Err(AppError::Forbidden(
        "Your role does not permit access to patient records".to_string(),
    ))
}

/// Check whether a user with role=Patient owns the specified patient record.
async fn check_patient_ownership(
    state: &AppState,
    user_id: Uuid,
    patient_id: Uuid,
) -> AppResult<bool> {
    let patient_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM patients WHERE id = $1")
            .bind(patient_id)
            .fetch_optional(state.db())
            .await?
            .flatten();

    Ok(patient_user_id == Some(user_id))
}

// ── Audit helpers ─────────────────────────────────────────────────────────────

async fn record_allow(
    audit: &AuditService,
    actor_id: Uuid,
    patient_id: Uuid,
    event_type: &'static str,
) {
    if let Err(e) = audit
        .record(
            AuditEvent::new(event_type)
                .actor(actor_id)
                .patient(patient_id)
                .resource("Patient", patient_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await
    {
        tracing::error!(error = %e, event_type, "Failed to write allowed-access audit log");
    }
}

async fn record_deny(
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
                .resource("Patient", patient_id)
                .outcome(OUTCOME_DENIED)
                .reason(reason),
        )
        .await
    {
        tracing::error!(error = %e, event_type, "Failed to write denied-access audit log");
    }
}

// ── Role guard helpers ────────────────────────────────────────────────────────

/// Assert the acting user has one of the required roles.
///
/// This is for non-patient-data endpoints (e.g. admin-only actions).
/// Use [`authorize_patient_access`] for patient data.
pub fn require_role(actor: &AuthenticatedUser, allowed_roles: &[UserRole]) -> AppResult<()> {
    if allowed_roles.contains(&actor.role) {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "This action requires one of the following roles: {}",
            allowed_roles
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::users::UserRole;

    #[test]
    fn require_role_allows_correct_role() {
        let user = AuthenticatedUser {
            id: Uuid::new_v4(),
            email: "admin@demo.local".to_string(),
            role: UserRole::SystemAdmin,
        };
        assert!(require_role(&user, &[UserRole::SystemAdmin]).is_ok());
        assert!(require_role(&user, &[UserRole::SystemAdmin, UserRole::FacilityAdmin]).is_ok());
    }

    #[test]
    fn require_role_denies_wrong_role() {
        let user = AuthenticatedUser {
            id: Uuid::new_v4(),
            email: "patient@demo.local".to_string(),
            role: UserRole::Patient,
        };
        assert!(require_role(&user, &[UserRole::SystemAdmin]).is_err());
        assert!(require_role(&user, &[UserRole::Doctor, UserRole::Lab]).is_err());
    }
}
