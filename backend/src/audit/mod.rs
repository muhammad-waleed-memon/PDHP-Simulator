//! Audit logging service — tamper-evident, append-only event recording.
//!
//! # Design
//!
//! Every security-relevant event is written to the `audit_log` table.
//! Each row contains a `row_hash` that is computed as:
//!
//! ```text
//! SHA-256(previous_hash || event_type || actor_user_id || patient_id
//!         || resource_type || resource_id || outcome || occurred_at_rfc3339)
//! ```
//!
//! The `previous_hash` is the `row_hash` of the immediately preceding
//! row (ordered by `id`).  The genesis row uses a fixed 64-zero hex string
//! as its `previous_hash`.
//!
//! This chain allows an offline verifier to detect any inserted, deleted,
//! or modified rows.
//!
//! # Failure Policy
//!
//! Audit failures are **never silently ignored**.  If the audit write fails,
//! the error is logged at `ERROR` level.  The caller must decide whether to
//! propagate the error (for write operations) or allow the action to proceed
//! (for read operations that already completed).  See [`AuditService::record`].

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

// ── Event type constants ──────────────────────────────────────────────────────

/// All recognised audit event type strings.
///
/// Using typed constants avoids typo-driven blind spots in queries/reports.
#[allow(dead_code)]
pub mod events {
    pub const LOGIN_SUCCESS: &str = "LOGIN_SUCCESS";
    pub const LOGIN_FAILURE: &str = "LOGIN_FAILURE";
    pub const REGISTER: &str = "REGISTER";
    pub const TOKEN_REFRESH: &str = "TOKEN_REFRESH";

    pub const PATIENT_RECORD_ACCESSED: &str = "PATIENT_RECORD_ACCESSED";
    pub const PATIENT_RECORD_CREATED: &str = "PATIENT_RECORD_CREATED";
    pub const PATIENT_RECORD_UPDATED: &str = "PATIENT_RECORD_UPDATED";

    pub const CONSENT_GRANTED: &str = "CONSENT_GRANTED";
    pub const CONSENT_REVOKED: &str = "CONSENT_REVOKED";
    pub const CONSENT_CHECKED: &str = "CONSENT_CHECKED";

    pub const ACCESS_DENIED: &str = "ACCESS_DENIED";
    pub const BREAK_GLASS_ACCESS: &str = "BREAK_GLASS_ACCESS";

    // Clinical events (Phase 4)
    pub const CONDITION_CREATED: &str = "CONDITION_CREATED";
    pub const CONDITION_UPDATED: &str = "CONDITION_UPDATED";
    pub const CONDITION_VIEWED: &str = "CONDITION_VIEWED";

    pub const ALLERGY_CREATED: &str = "ALLERGY_CREATED";
    pub const ALLERGY_UPDATED: &str = "ALLERGY_UPDATED";
    pub const ALLERGY_VIEWED: &str = "ALLERGY_VIEWED";

    pub const MEDICATION_CREATED: &str = "MEDICATION_CREATED";
    pub const MEDICATION_VIEWED: &str = "MEDICATION_VIEWED";

    pub const PATIENT_MEDICATION_CREATED: &str = "PATIENT_MEDICATION_CREATED";
    pub const PATIENT_MEDICATION_UPDATED: &str = "PATIENT_MEDICATION_UPDATED";
    pub const PATIENT_MEDICATION_VIEWED: &str = "PATIENT_MEDICATION_VIEWED";

    pub const ENCOUNTER_CREATED: &str = "ENCOUNTER_CREATED";
    pub const ENCOUNTER_UPDATED: &str = "ENCOUNTER_UPDATED";
    pub const ENCOUNTER_VIEWED: &str = "ENCOUNTER_VIEWED";

    pub const PRESCRIPTION_CREATED: &str = "PRESCRIPTION_CREATED";
    pub const PRESCRIPTION_VIEWED: &str = "PRESCRIPTION_VIEWED";

    pub const LAB_REPORT_CREATED: &str = "LAB_REPORT_CREATED";
    pub const LAB_REPORT_VIEWED: &str = "LAB_REPORT_VIEWED";

    pub const TIMELINE_VIEWED: &str = "TIMELINE_VIEWED";
    pub const CLINICAL_ALERT_TRIGGERED: &str = "CLINICAL_ALERT_TRIGGERED";
    pub const CLINICAL_ALERT_OVERRIDDEN: &str = "CLINICAL_ALERT_OVERRIDDEN";

    // FHIR Interoperability events (Phase 5)
    pub const FHIR_RESOURCE_VIEWED: &str = "FHIR_RESOURCE_VIEWED";
    pub const FHIR_SEARCH_EXECUTED: &str = "FHIR_SEARCH_EXECUTED";
    pub const FHIR_BUNDLE_ACCESSED: &str = "FHIR_BUNDLE_ACCESSED";
    pub const FHIR_ACCESS_DENIED: &str = "FHIR_ACCESS_DENIED";
    pub const FHIR_VALIDATION_FAILED: &str = "FHIR_VALIDATION_FAILED";
}

// ── Outcome constants ─────────────────────────────────────────────────────────

/// The `outcome` column value when an action was permitted.
pub const OUTCOME_ALLOWED: &str = "ALLOWED";
/// The `outcome` column value when an action was denied.
pub const OUTCOME_DENIED: &str = "DENIED";

// ── Domain types ──────────────────────────────────────────────────────────────

/// Optional structured metadata attached to an audit event.
///
/// **Never include** passwords, hashes, tokens, or PII beyond what is
/// strictly necessary for auditability.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditMetadata {
    /// Caller IP address, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// HTTP User-Agent header value, truncated if necessary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// Free-form extra fields (must not contain sensitive data).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// Builder for an audit event.
///
/// ```rust,no_run
/// # use backend::audit::{AuditEvent, events, OUTCOME_ALLOWED};
/// # use uuid::Uuid;
/// let event = AuditEvent::new(events::LOGIN_SUCCESS)
///     .actor(Uuid::new_v4())
///     .outcome(OUTCOME_ALLOWED);
/// ```
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub actor_user_id: Option<Uuid>,
    pub patient_id: Option<Uuid>,
    pub event_type: &'static str,
    pub resource_type: Option<&'static str>,
    pub resource_id: Option<String>,
    pub metadata: Option<AuditMetadata>,
    pub outcome: &'static str,
    pub reason: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event with `outcome = ALLOWED` by default.
    pub fn new(event_type: &'static str) -> Self {
        Self {
            actor_user_id: None,
            patient_id: None,
            event_type,
            resource_type: None,
            resource_id: None,
            metadata: None,
            outcome: OUTCOME_ALLOWED,
            reason: None,
        }
    }

    pub fn actor(mut self, user_id: Uuid) -> Self {
        self.actor_user_id = Some(user_id);
        self
    }

    pub fn patient(mut self, patient_id: Uuid) -> Self {
        self.patient_id = Some(patient_id);
        self
    }

    pub fn resource(mut self, resource_type: &'static str, resource_id: impl ToString) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id.to_string());
        self
    }

    pub fn outcome(mut self, outcome: &'static str) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    #[allow(dead_code)]
    pub fn metadata(mut self, meta: AuditMetadata) -> Self {
        self.metadata = Some(meta);
        self
    }
}

// ── Row struct (for reading back from DB) ─────────────────────────────────────

/// A row from the `audit_log` table.
#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: i64,
    pub actor_user_id: Option<Uuid>,
    pub patient_id: Option<Uuid>,
    pub event_type: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub outcome: String,
    pub reason: Option<String>,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub previous_hash: String,
    pub row_hash: String,
}

// ── Service ───────────────────────────────────────────────────────────────────

/// Audit service — records tamper-evident events to the database.
///
/// Clone is cheap (inner `PgPool` is `Arc`-wrapped).
#[derive(Clone)]
pub struct AuditService {
    db: PgPool,
}

impl AuditService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Record an audit event.
    ///
    /// # Errors
    ///
    /// Returns a `sqlx::Error` if the database write fails.
    /// The caller must decide how to handle this — audit failures should
    /// never be silently discarded.
    pub async fn record(&self, event: AuditEvent) -> Result<i64, sqlx::Error> {
        let occurred_at = chrono::Utc::now();

        // Fetch the hash of the most recent row to chain from.
        let previous_hash: String = sqlx::query_scalar(
            "SELECT COALESCE(row_hash, '0000000000000000000000000000000000000000000000000000000000000000') \
             FROM audit_log ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or_else(|| {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        });

        // Compute the hash for this row.
        let row_hash = compute_row_hash(
            &previous_hash,
            event.event_type,
            event.actor_user_id.as_ref(),
            event.patient_id.as_ref(),
            event.resource_type.unwrap_or(""),
            event.resource_id.as_deref().unwrap_or(""),
            event.outcome,
            &occurred_at.to_rfc3339(),
        );

        let metadata_json: Option<serde_json::Value> = event
            .metadata
            .as_ref()
            .and_then(|m| serde_json::to_value(m).ok());

        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO audit_log (
                actor_user_id, patient_id, event_type,
                resource_type, resource_id, metadata,
                outcome, reason, occurred_at,
                previous_hash, row_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id
            "#,
        )
        .bind(event.actor_user_id)
        .bind(event.patient_id)
        .bind(event.event_type)
        .bind(event.resource_type)
        .bind(event.resource_id)
        .bind(metadata_json)
        .bind(event.outcome)
        .bind(event.reason)
        .bind(occurred_at)
        .bind(&previous_hash)
        .bind(&row_hash)
        .fetch_one(&self.db)
        .await?;

        tracing::debug!(
            audit_id = id,
            event_type = event.event_type,
            outcome = event.outcome,
            actor = ?event.actor_user_id,
            "Audit event recorded"
        );

        Ok(id)
    }

    /// Verify the integrity of the entire audit chain.
    ///
    /// Returns `(total_rows, first_broken_id)` — if `first_broken_id` is
    /// `None`, the chain is intact.
    ///
    /// This is an O(n) scan — do not call on hot paths.
    #[allow(dead_code)]
    pub async fn verify_chain(&self) -> Result<(i64, Option<i64>), sqlx::Error> {
        let rows: Vec<AuditLogRow> = sqlx::query_as("SELECT * FROM audit_log ORDER BY id ASC")
            .fetch_all(&self.db)
            .await?;

        let total = rows.len() as i64;
        let mut prev_hash =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        for row in &rows {
            let expected = compute_row_hash(
                &prev_hash,
                &row.event_type,
                row.actor_user_id.as_ref(),
                row.patient_id.as_ref(),
                row.resource_type.as_deref().unwrap_or(""),
                row.resource_id.as_deref().unwrap_or(""),
                &row.outcome,
                &row.occurred_at.to_rfc3339(),
            );

            if expected != row.row_hash {
                tracing::error!(
                    audit_id = row.id,
                    expected_hash = %expected,
                    stored_hash = %row.row_hash,
                    "Audit chain integrity violation detected"
                );
                return Ok((total, Some(row.id)));
            }

            prev_hash = row.row_hash.clone();
        }

        Ok((total, None))
    }
}

// ── Hash computation ──────────────────────────────────────────────────────────

/// Compute the SHA-256 chain hash for one audit row.
///
/// Inputs are concatenated with a null-byte separator to avoid
/// length-extension ambiguity.
#[allow(clippy::too_many_arguments)]
fn compute_row_hash(
    previous_hash: &str,
    event_type: &str,
    actor_user_id: Option<&Uuid>,
    patient_id: Option<&Uuid>,
    resource_type: &str,
    resource_id: &str,
    outcome: &str,
    occurred_at_rfc3339: &str,
) -> String {
    let mut hasher = Sha256::new();

    // Each field separated by NUL to prevent concatenation collisions
    hasher.update(previous_hash.as_bytes());
    hasher.update(b"\x00");
    hasher.update(event_type.as_bytes());
    hasher.update(b"\x00");
    hasher.update(
        actor_user_id
            .map(|u| u.to_string())
            .as_deref()
            .unwrap_or("")
            .as_bytes(),
    );
    hasher.update(b"\x00");
    hasher.update(
        patient_id
            .map(|u| u.to_string())
            .as_deref()
            .unwrap_or("")
            .as_bytes(),
    );
    hasher.update(b"\x00");
    hasher.update(resource_type.as_bytes());
    hasher.update(b"\x00");
    hasher.update(resource_id.as_bytes());
    hasher.update(b"\x00");
    hasher.update(outcome.as_bytes());
    hasher.update(b"\x00");
    hasher.update(occurred_at_rfc3339.as_bytes());

    format!("{:x}", hasher.finalize())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let h1 = compute_row_hash(
            "0000000000000000000000000000000000000000000000000000000000000000",
            events::LOGIN_SUCCESS,
            None,
            None,
            "",
            "",
            OUTCOME_ALLOWED,
            "2024-01-01T00:00:00+00:00",
        );
        let h2 = compute_row_hash(
            "0000000000000000000000000000000000000000000000000000000000000000",
            events::LOGIN_SUCCESS,
            None,
            None,
            "",
            "",
            OUTCOME_ALLOWED,
            "2024-01-01T00:00:00+00:00",
        );
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_events_produce_different_hashes() {
        let base = "0000000000000000000000000000000000000000000000000000000000000000";
        let ts = "2024-01-01T00:00:00+00:00";
        let h1 = compute_row_hash(
            base,
            events::LOGIN_SUCCESS,
            None,
            None,
            "",
            "",
            OUTCOME_ALLOWED,
            ts,
        );
        let h2 = compute_row_hash(
            base,
            events::LOGIN_FAILURE,
            None,
            None,
            "",
            "",
            OUTCOME_DENIED,
            ts,
        );
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_length_is_64_hex_chars() {
        let h = compute_row_hash(
            "0000000000000000000000000000000000000000000000000000000000000000",
            events::ACCESS_DENIED,
            Some(&Uuid::new_v4()),
            Some(&Uuid::new_v4()),
            "Patient",
            "some-id",
            OUTCOME_DENIED,
            "2024-06-15T12:30:00+00:00",
        );
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn chaining_changes_hash() {
        let base = "0000000000000000000000000000000000000000000000000000000000000000";
        let ts = "2024-01-01T00:00:00+00:00";
        let h1 = compute_row_hash(
            base,
            events::LOGIN_SUCCESS,
            None,
            None,
            "",
            "",
            OUTCOME_ALLOWED,
            ts,
        );
        // Chain from h1
        let h2 = compute_row_hash(
            &h1,
            events::PATIENT_RECORD_ACCESSED,
            None,
            None,
            "",
            "",
            OUTCOME_ALLOWED,
            ts,
        );
        assert_ne!(h1, h2);
        // Chaining from h1 again should give the same result
        let h2b = compute_row_hash(
            &h1,
            events::PATIENT_RECORD_ACCESSED,
            None,
            None,
            "",
            "",
            OUTCOME_ALLOWED,
            ts,
        );
        assert_eq!(h2, h2b);
    }

    #[test]
    fn audit_event_builder() {
        let uid = Uuid::new_v4();
        let pid = Uuid::new_v4();
        let event = AuditEvent::new(events::CONSENT_GRANTED)
            .actor(uid)
            .patient(pid)
            .resource("ConsentGrant", "grant-123")
            .outcome(OUTCOME_ALLOWED)
            .reason("Doctor requested access");

        assert_eq!(event.event_type, events::CONSENT_GRANTED);
        assert_eq!(event.actor_user_id, Some(uid));
        assert_eq!(event.patient_id, Some(pid));
        assert_eq!(event.outcome, OUTCOME_ALLOWED);
        assert!(event.reason.is_some());
    }
}
