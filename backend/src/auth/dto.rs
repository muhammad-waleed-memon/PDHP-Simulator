//! Authentication DTOs.

use crate::users::{UserRole, UserStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub phone_number: Option<String>,
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: AuthUserInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthUserInfo {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub role: UserRole,
    pub status: UserStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMeResponse {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub role: UserRole,
    pub status: UserStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::users::UserRole;

    #[test]
    fn test_register_request_deserialization_success() {
        let json_patient = r#"{
            "email": "test@demo.local",
            "password": "Password123!",
            "full_name": "Test Patient",
            "role": "Patient"
        }"#;
        let req: RegisterRequest = serde_json::from_str(json_patient).expect("Should deserialize Patient");
        assert_eq!(req.role, UserRole::Patient);

        let json_doctor = r#"{
            "email": "doc@demo.local",
            "password": "Password123!",
            "full_name": "Test Doctor",
            "role": "Doctor"
        }"#;
        let req_doc: RegisterRequest = serde_json::from_str(json_doctor).expect("Should deserialize Doctor");
        assert_eq!(req_doc.role, UserRole::Doctor);

        let json_lab = r#"{
            "email": "lab@demo.local",
            "password": "Password123!",
            "full_name": "Test Lab",
            "role": "Lab"
        }"#;
        let req_lab: RegisterRequest = serde_json::from_str(json_lab).expect("Should deserialize Lab");
        assert_eq!(req_lab.role, UserRole::Lab);

        let json_fa = r#"{
            "email": "fa@demo.local",
            "password": "Password123!",
            "full_name": "Test Facility Admin",
            "role": "FacilityAdmin"
        }"#;
        let req_fa: RegisterRequest = serde_json::from_str(json_fa).expect("Should deserialize FacilityAdmin");
        assert_eq!(req_fa.role, UserRole::FacilityAdmin);

        let json_sa = r#"{
            "email": "sa@demo.local",
            "password": "Password123!",
            "full_name": "Test System Admin",
            "role": "SystemAdmin"
        }"#;
        let req_sa: RegisterRequest = serde_json::from_str(json_sa).expect("Should deserialize SystemAdmin");
        assert_eq!(req_sa.role, UserRole::SystemAdmin);
    }

    #[test]
    fn test_register_request_deserialization_uppercase_fails() {
        let json_uppercase = r#"{
            "email": "test@demo.local",
            "password": "Password123!",
            "full_name": "Test Patient",
            "role": "PATIENT"
        }"#;
        let res: Result<RegisterRequest, _> = serde_json::from_str(json_uppercase);
        assert!(res.is_err(), "Uppercase PATIENT should fail deserialization into UserRole");
    }
}
