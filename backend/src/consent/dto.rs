//! Data Transfer Objects for the consent API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request body to grant consent.
#[derive(Debug, Deserialize)]
pub struct ConsentGrantRequest {
    /// The patient's UUID to grant access for.
    pub patient_id: Uuid,
    /// The user UUID of the provider being granted access.
    pub granted_to_user_id: Uuid,
    /// Optional expiry time.  If omitted, the grant does not auto-expire.
    pub expires_at: Option<DateTime<Utc>>,
    /// Optional human-readable purpose / scope description.
    pub purpose: Option<String>,
}

/// API response representing a consent grant.
#[derive(Debug, Serialize)]
pub struct ConsentGrantResponse {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub granted_to_user_id: Uuid,
    pub status: String,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub purpose: Option<String>,
    pub created_at: DateTime<Utc>,
}
