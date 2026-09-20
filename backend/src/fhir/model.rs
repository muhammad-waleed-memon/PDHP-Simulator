//! FHIR R4 Resource DTO Definitions — Standards-compliant JSON structures for interoperability.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhirCoding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhirCodeableConcept {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<FhirCoding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl FhirCodeableConcept {
    pub fn from_text(text: &str) -> Self {
        Self {
            coding: None,
            text: Some(text.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhirIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhirHumanName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_type: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhirContactPoint {
    pub system: String, // "phone", "email"
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhirAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhirPeriod {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhirReference {
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl FhirReference {
    pub fn new(resource_type: &str, id: &str, display: Option<String>) -> Self {
        Self {
            reference: format!("{}/{}", resource_type, id),
            display,
        }
    }
}

// ── 1. PATIENT ────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirPatient {
    pub resource_type: String,
    pub id: String,
    pub identifier: Vec<FhirIdentifier>,
    pub active: bool,
    pub name: Vec<FhirHumanName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telecom: Option<Vec<FhirContactPoint>>,
    pub gender: String,
    pub birth_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<FhirAddress>>,
}

// ── 2. PRACTITIONER (Doctor) ──────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirPractitioner {
    pub resource_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<FhirIdentifier>>,
    pub active: bool,
    pub name: Vec<FhirHumanName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualification: Option<Vec<FhirCodeableConcept>>,
}

// ── 3. PRACTITIONER ROLE ──────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirPractitionerRole {
    pub resource_type: String,
    pub id: String,
    pub active: bool,
    pub practitioner: FhirReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<FhirReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specialty: Option<Vec<FhirCodeableConcept>>,
}

// ── 4. ORGANIZATION (Facility) ────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOrganization {
    pub resource_type: String,
    pub id: String,
    pub active: bool,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<FhirAddress>>,
}

// ── 5. LOCATION ───────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirLocation {
    pub resource_type: String,
    pub id: String,
    pub status: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<FhirAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managing_organization: Option<FhirReference>,
}

// ── 6. ENCOUNTER ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirEncounterParticipant {
    pub individual: FhirReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirEncounter {
    pub resource_type: String,
    pub id: String,
    pub status: String,
    pub class: FhirCoding,
    pub subject: FhirReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participant: Option<Vec<FhirEncounterParticipant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_provider: Option<FhirReference>,
    pub period: FhirPeriod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<Vec<FhirCodeableConcept>>,
}

// ── 7. CONDITION ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirCondition {
    pub resource_type: String,
    pub id: String,
    pub clinical_status: FhirCodeableConcept,
    pub code: FhirCodeableConcept,
    pub subject: FhirReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onset_date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_date: Option<String>,
}

// ── 8. ALLERGY INTOLERANCE ────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAllergyReaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Vec<FhirCodeableConcept>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAllergyIntolerance {
    pub resource_type: String,
    pub id: String,
    pub clinical_status: FhirCodeableConcept,
    pub code: FhirCodeableConcept,
    pub patient: FhirReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaction: Option<Vec<FhirAllergyReaction>>,
}

// ── 9. MEDICATION ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirMedication {
    pub resource_type: String,
    pub id: String,
    pub code: FhirCodeableConcept,
    pub status: String,
}

// ── 10. MEDICATION STATEMENT ──────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirDosage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirMedicationStatement {
    pub resource_type: String,
    pub id: String,
    pub status: String,
    pub medication_reference: FhirReference,
    pub subject: FhirReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_period: Option<FhirPeriod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dosage: Option<Vec<FhirDosage>>,
}

// ── 11. MEDICATION REQUEST (Prescription) ─────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirMedicationRequest {
    pub resource_type: String,
    pub id: String,
    pub status: String,
    pub intent: String, // "order"
    pub medication_reference: FhirReference,
    pub subject: FhirReference,
    pub requester: FhirReference,
    pub authored_on: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dosage_instruction: Option<Vec<FhirDosage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<String>>,
}

// ── 12. OBSERVATION (Lab Result) ──────────────────────────────────────────────
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirValueQuantity {
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirObservation {
    pub resource_type: String,
    pub id: String,
    pub status: String,
    pub category: Vec<FhirCodeableConcept>,
    pub code: FhirCodeableConcept,
    pub subject: FhirReference,
    pub effective_date_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation: Option<Vec<FhirCodeableConcept>>,
}

// ── 13. DIAGNOSTIC REPORT (Lab Report) ────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirDiagnosticReport {
    pub resource_type: String,
    pub id: String,
    pub status: String,
    pub code: FhirCodeableConcept,
    pub subject: FhirReference,
    pub effective_date_time: String,
    pub issued: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<FhirReference>>,
}

// ── 14. BUNDLE (Patient Summary Export) ───────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirBundleEntry {
    pub full_url: String,
    pub resource: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirBundle {
    pub resource_type: String,
    pub id: String,
    pub r#type: String, // "collection" or "searchset"
    pub total: usize,
    pub entry: Vec<FhirBundleEntry>,
}

// ── 15. OPERATION OUTCOME (FHIR Error Response) ──────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOperationOutcomeIssue {
    pub severity: String, // "error", "fatal", "warning"
    pub code: String,     // "forbidden", "not-found", "invalid"
    pub diagnostics: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOperationOutcome {
    pub resource_type: String,
    pub issue: Vec<FhirOperationOutcomeIssue>,
}

impl FhirOperationOutcome {
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            resource_type: "OperationOutcome".to_string(),
            issue: vec![FhirOperationOutcomeIssue {
                severity: "error".to_string(),
                code: code.to_string(),
                diagnostics: message.to_string(),
            }],
        }
    }
}
