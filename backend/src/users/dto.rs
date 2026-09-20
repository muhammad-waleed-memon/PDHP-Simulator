//! User DTOs — safe user representations excluding password hashes.

use super::{User, UserRole, UserStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Safe public user response (never exposes password hashes).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub phone_number: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            phone_number: user.phone_number,
            role: user.role,
            status: user.status,
            created_at: user.created_at,
        }
    }
}
