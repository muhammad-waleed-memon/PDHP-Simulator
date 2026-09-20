//! Clinical Alerts module — safety rules for allergy conflicts and duplicate medications.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertType {
    AllergyConflict,
    DuplicateMedication,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalAlert {
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub medication_id: Uuid,
    pub medication_name: String,
    pub message: String,
}

#[derive(FromRow)]
struct MedInfo {
    id: Uuid,
    name: String,
    generic_name: Option<String>,
}

#[derive(FromRow)]
struct AllergyInfo {
    allergen: String,
    severity: String,
    #[allow(dead_code)]
    reaction: Option<String>,
}

#[derive(FromRow)]
struct ActiveMedInfo {
    medication_id: Uuid,
    name: String,
}

/// Evaluate safety rules for a list of prescribed medications against a patient's active records.
pub async fn evaluate_prescription_alerts(
    pool: &PgPool,
    patient_id: Uuid,
    prescribed_med_ids: &[Uuid],
) -> Result<Vec<ClinicalAlert>, sqlx::Error> {
    let mut alerts = Vec::new();

    if prescribed_med_ids.is_empty() {
        return Ok(alerts);
    }

    // Fetch details of prescribed medications
    let prescribed_meds = sqlx::query_as::<_, MedInfo>(
        "SELECT id, name, generic_name FROM medications WHERE id = ANY($1)",
    )
    .bind(prescribed_med_ids)
    .fetch_all(pool)
    .await?;

    // 1. ALLERGY CONFLICT CHECK
    let active_allergies = sqlx::query_as::<_, AllergyInfo>(
        "SELECT allergen, severity, reaction FROM allergies WHERE patient_id = $1 AND status = 'ACTIVE'",
    )
    .bind(patient_id)
    .fetch_all(pool)
    .await?;

    for med in &prescribed_meds {
        let med_name_lower = med.name.to_lowercase();
        let generic_lower = med.generic_name.as_deref().unwrap_or("").to_lowercase();

        for allergy in &active_allergies {
            let allergen_lower = allergy.allergen.to_lowercase();

            // Check if allergen name matches med name or generic name
            if !allergen_lower.is_empty()
                && (med_name_lower.contains(&allergen_lower)
                    || allergen_lower.contains(&med_name_lower)
                    || (!generic_lower.is_empty()
                        && (generic_lower.contains(&allergen_lower)
                            || allergen_lower.contains(&generic_lower))))
            {
                let severity = match allergy.severity.to_uppercase().as_str() {
                    "SEVERE" | "CRITICAL" => AlertSeverity::Critical,
                    _ => AlertSeverity::Warning,
                };

                alerts.push(ClinicalAlert {
                    alert_type: AlertType::AllergyConflict,
                    severity,
                    medication_id: med.id,
                    medication_name: med.name.clone(),
                    message: format!(
                        "ALLERGY CONFLICT ALERT: Patient has known active allergy to '{}' (Severity: {}). Prescribed: '{}'.",
                        allergy.allergen, allergy.severity, med.name
                    ),
                });
            }
        }
    }

    // 2. DUPLICATE MEDICATION CHECK
    let active_patient_meds = sqlx::query_as::<_, ActiveMedInfo>(
        r#"
        SELECT pm.medication_id, m.name
        FROM patient_medications pm
        JOIN medications m ON m.id = pm.medication_id
        WHERE pm.patient_id = $1 AND pm.status = 'ACTIVE'
        "#,
    )
    .bind(patient_id)
    .fetch_all(pool)
    .await?;

    for med in &prescribed_meds {
        for active in &active_patient_meds {
            if med.id == active.medication_id
                || med.name.to_lowercase() == active.name.to_lowercase()
            {
                alerts.push(ClinicalAlert {
                    alert_type: AlertType::DuplicateMedication,
                    severity: AlertSeverity::Warning,
                    medication_id: med.id,
                    medication_name: med.name.clone(),
                    message: format!(
                        "DUPLICATE MEDICATION WARNING: Patient is currently taking active medication '{}'.",
                        active.name
                    ),
                });
            }
        }
    }

    Ok(alerts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_severity_serialization() {
        let alert = ClinicalAlert {
            alert_type: AlertType::AllergyConflict,
            severity: AlertSeverity::Critical,
            medication_id: Uuid::new_v4(),
            medication_name: "Amoxicillin".to_string(),
            message: "Test alert".to_string(),
        };

        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("AllergyConflict"));
        assert!(json.contains("Critical"));
    }
}
