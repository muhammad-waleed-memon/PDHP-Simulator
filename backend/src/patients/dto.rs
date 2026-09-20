//! Patient DTOs — request payload definitions and safe public responses.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreatePatientRequest {
    /// Optional user login email if creating a patient user account
    pub email: Option<String>,
    /// Optional password if creating a patient user account
    pub password: Option<String>,
    pub full_name: String,
    pub date_of_birth: chrono::NaiveDate,
    pub gender: String,
    pub blood_group: Option<String>,
    pub phone_number: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatientResponse {
    pub id: Uuid,
    pub digital_health_id: String,
    pub user_id: Option<Uuid>,
    pub full_name: String,
    pub date_of_birth: chrono::NaiveDate,
    pub gender: String,
    pub blood_group: Option<String>,
    pub phone_number: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_phone: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
