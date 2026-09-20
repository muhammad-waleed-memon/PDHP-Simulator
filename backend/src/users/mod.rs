//! Users module — user representation and role models.

pub mod dto;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Application user roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Patient,
    Doctor,
    Lab,
    FacilityAdmin,
    SystemAdmin,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Patient => write!(f, "PATIENT"),
            UserRole::Doctor => write!(f, "DOCTOR"),
            UserRole::Lab => write!(f, "LAB"),
            UserRole::FacilityAdmin => write!(f, "FACILITY_ADMIN"),
            UserRole::SystemAdmin => write!(f, "SYSTEM_ADMIN"),
        }
    }
}

/// User account status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    PendingVerification,
}

/// Core User database entity.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub phone_number: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
