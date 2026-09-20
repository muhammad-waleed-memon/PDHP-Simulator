//! Doctor DTOs.

use super::DoctorStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateDoctorRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub phone_number: Option<String>,
    pub pmdc_license_number: Option<String>,
    pub specialization: String,
    pub qualification: Option<String>,
    pub facility_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DoctorResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub facility_id: Option<Uuid>,
    pub full_name: String,
    pub email: String,
    pub pmdc_license_number: Option<String>,
    pub specialization: String,
    pub qualification: Option<String>,
    pub status: DoctorStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
