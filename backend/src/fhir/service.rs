//! FHIR Service — enforces security, consent, authorization, audit logging, and resource mapping.

use sqlx::PgPool;
use uuid::Uuid;

use crate::allergies::Allergy;
use crate::audit::{events, AuditEvent, AuditService, OUTCOME_ALLOWED};
use crate::auth::jwt::AuthenticatedUser;
use crate::authorization::authorize_patient_access;
use crate::conditions::Condition;
use crate::doctors::Doctor;
use crate::encounters::Encounter;
use crate::error::{AppError, AppResult};
use crate::facilities::Facility;
use crate::fhir::mappers::*;
use crate::fhir::model::*;
use crate::laboratories::LabReport;
use crate::medications::Medication;
use crate::patient_medications::PatientMedication;
use crate::patients::Patient;
use crate::state::AppState;

pub async fn get_fhir_patient(
    state: &AppState,
    user: &AuthenticatedUser,
    patient_id: Uuid,
) -> AppResult<FhirPatient> {
    authorize_patient_access(state, user, patient_id).await?;

    let patient = sqlx::query_as::<_, Patient>("SELECT * FROM patients WHERE id = $1")
        .bind(patient_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Patient not found".to_string()))?;

    let fhir_patient = map_patient_to_fhir(&patient);

    record_fhir_audit(
        state.db(),
        user.id,
        patient_id,
        events::FHIR_RESOURCE_VIEWED,
        "Patient",
        &patient_id.to_string(),
    )
    .await;

    Ok(fhir_patient)
}

pub async fn get_fhir_patient_by_dhid(
    state: &AppState,
    user: &AuthenticatedUser,
    dhid: &str,
) -> AppResult<FhirPatient> {
    let patient =
        sqlx::query_as::<_, Patient>("SELECT * FROM patients WHERE digital_health_id = $1")
            .bind(dhid)
            .fetch_optional(state.db())
            .await?
            .ok_or_else(|| {
                AppError::NotFound("Patient not found for specified Digital Health ID".to_string())
            })?;

    authorize_patient_access(state, user, patient.id).await?;

    let fhir_patient = map_patient_to_fhir(&patient);

    record_fhir_audit(
        state.db(),
        user.id,
        patient.id,
        events::FHIR_SEARCH_EXECUTED,
        "Patient",
        dhid,
    )
    .await;

    Ok(fhir_patient)
}

pub async fn get_fhir_practitioner(
    state: &AppState,
    user: &AuthenticatedUser,
    doctor_id: Uuid,
) -> AppResult<FhirPractitioner> {
    let doctor = sqlx::query_as::<_, Doctor>("SELECT * FROM doctors WHERE id = $1")
        .bind(doctor_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Doctor practitioner not found".to_string()))?;

    let name: String = sqlx::query_scalar("SELECT full_name FROM users WHERE id = $1")
        .bind(doctor.user_id)
        .fetch_one(state.db())
        .await?;

    let fhir_doc = map_doctor_to_practitioner(&doctor, &name);

    record_fhir_audit(
        state.db(),
        user.id,
        doctor_id,
        events::FHIR_RESOURCE_VIEWED,
        "Practitioner",
        &doctor_id.to_string(),
    )
    .await;

    Ok(fhir_doc)
}

pub async fn get_fhir_practitioner_role(
    state: &AppState,
    user: &AuthenticatedUser,
    doctor_id: Uuid,
) -> AppResult<FhirPractitionerRole> {
    let doctor = sqlx::query_as::<_, Doctor>("SELECT * FROM doctors WHERE id = $1")
        .bind(doctor_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("PractitionerRole not found".to_string()))?;

    let name: String = sqlx::query_scalar("SELECT full_name FROM users WHERE id = $1")
        .bind(doctor.user_id)
        .fetch_one(state.db())
        .await?;

    let fhir_role = map_doctor_to_practitioner_role(&doctor, &name);

    record_fhir_audit(
        state.db(),
        user.id,
        doctor_id,
        events::FHIR_RESOURCE_VIEWED,
        "PractitionerRole",
        &doctor_id.to_string(),
    )
    .await;

    Ok(fhir_role)
}

pub async fn get_fhir_organization(
    state: &AppState,
    user: &AuthenticatedUser,
    facility_id: Uuid,
) -> AppResult<FhirOrganization> {
    let fac = sqlx::query_as::<_, Facility>("SELECT * FROM facilities WHERE id = $1")
        .bind(facility_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Facility organization not found".to_string()))?;

    let fhir_org = map_facility_to_organization(&fac);

    record_fhir_audit(
        state.db(),
        user.id,
        facility_id,
        events::FHIR_RESOURCE_VIEWED,
        "Organization",
        &facility_id.to_string(),
    )
    .await;

    Ok(fhir_org)
}

pub async fn get_fhir_location(
    state: &AppState,
    user: &AuthenticatedUser,
    facility_id: Uuid,
) -> AppResult<FhirLocation> {
    let fac = sqlx::query_as::<_, Facility>("SELECT * FROM facilities WHERE id = $1")
        .bind(facility_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Facility location not found".to_string()))?;

    let fhir_loc = map_facility_to_location(&fac);

    record_fhir_audit(
        state.db(),
        user.id,
        facility_id,
        events::FHIR_RESOURCE_VIEWED,
        "Location",
        &facility_id.to_string(),
    )
    .await;

    Ok(fhir_loc)
}

pub async fn get_fhir_medication(
    state: &AppState,
    user: &AuthenticatedUser,
    med_id: Uuid,
) -> AppResult<FhirMedication> {
    let med = sqlx::query_as::<_, Medication>("SELECT * FROM medications WHERE id = $1")
        .bind(med_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Medication resource not found".to_string()))?;

    let fhir_med = map_medication_to_fhir(&med);

    record_fhir_audit(
        state.db(),
        user.id,
        med_id,
        events::FHIR_RESOURCE_VIEWED,
        "Medication",
        &med_id.to_string(),
    )
    .await;

    Ok(fhir_med)
}

pub async fn search_fhir_medication_statements(
    state: &AppState,
    user: &AuthenticatedUser,
    patient_id: Uuid,
) -> AppResult<Vec<FhirMedicationStatement>> {
    authorize_patient_access(state, user, patient_id).await?;

    let pms = sqlx::query_as::<_, PatientMedication>(
        "SELECT * FROM patient_medications WHERE patient_id = $1 ORDER BY start_date DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let patient_name: String = sqlx::query_scalar("SELECT full_name FROM patients WHERE id = $1")
        .bind(patient_id)
        .fetch_one(state.db())
        .await?;

    let mut result = Vec::new();
    for pm in pms {
        let med_name: String = sqlx::query_scalar("SELECT name FROM medications WHERE id = $1")
            .bind(pm.medication_id)
            .fetch_optional(state.db())
            .await?
            .unwrap_or_else(|| "Unknown Medication".to_string());

        result.push(map_patient_medication_to_fhir(
            &pm,
            &patient_name,
            &med_name,
        ));
    }

    record_fhir_audit(
        state.db(),
        user.id,
        patient_id,
        events::FHIR_SEARCH_EXECUTED,
        "MedicationStatement",
        &patient_id.to_string(),
    )
    .await;

    Ok(result)
}

pub async fn get_fhir_condition(
    state: &AppState,
    user: &AuthenticatedUser,
    condition_id: Uuid,
) -> AppResult<FhirCondition> {
    let cond = sqlx::query_as::<_, Condition>("SELECT * FROM conditions WHERE id = $1")
        .bind(condition_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Condition resource not found".to_string()))?;

    authorize_patient_access(state, user, cond.patient_id).await?;

    let patient_name: String = sqlx::query_scalar("SELECT full_name FROM patients WHERE id = $1")
        .bind(cond.patient_id)
        .fetch_one(state.db())
        .await?;

    let fhir_cond = map_condition_to_fhir(&cond, &patient_name);

    record_fhir_audit(
        state.db(),
        user.id,
        cond.patient_id,
        events::FHIR_RESOURCE_VIEWED,
        "Condition",
        &condition_id.to_string(),
    )
    .await;

    Ok(fhir_cond)
}

pub async fn search_fhir_conditions(
    state: &AppState,
    user: &AuthenticatedUser,
    patient_id: Uuid,
) -> AppResult<Vec<FhirCondition>> {
    authorize_patient_access(state, user, patient_id).await?;

    let conditions = sqlx::query_as::<_, Condition>(
        "SELECT * FROM conditions WHERE patient_id = $1 ORDER BY created_at DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let patient_name: String = sqlx::query_scalar("SELECT full_name FROM patients WHERE id = $1")
        .bind(patient_id)
        .fetch_one(state.db())
        .await?;

    let result = conditions
        .iter()
        .map(|c| map_condition_to_fhir(c, &patient_name))
        .collect();

    record_fhir_audit(
        state.db(),
        user.id,
        patient_id,
        events::FHIR_SEARCH_EXECUTED,
        "Condition",
        &patient_id.to_string(),
    )
    .await;

    Ok(result)
}

pub async fn get_fhir_allergy(
    state: &AppState,
    user: &AuthenticatedUser,
    allergy_id: Uuid,
) -> AppResult<FhirAllergyIntolerance> {
    let allergy = sqlx::query_as::<_, Allergy>("SELECT * FROM allergies WHERE id = $1")
        .bind(allergy_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("AllergyIntolerance resource not found".to_string()))?;

    authorize_patient_access(state, user, allergy.patient_id).await?;

    let patient_name: String = sqlx::query_scalar("SELECT full_name FROM patients WHERE id = $1")
        .bind(allergy.patient_id)
        .fetch_one(state.db())
        .await?;

    let fhir_allergy = map_allergy_to_fhir(&allergy, &patient_name);

    record_fhir_audit(
        state.db(),
        user.id,
        allergy.patient_id,
        events::FHIR_RESOURCE_VIEWED,
        "AllergyIntolerance",
        &allergy_id.to_string(),
    )
    .await;

    Ok(fhir_allergy)
}

pub async fn search_fhir_allergies(
    state: &AppState,
    user: &AuthenticatedUser,
    patient_id: Uuid,
) -> AppResult<Vec<FhirAllergyIntolerance>> {
    authorize_patient_access(state, user, patient_id).await?;

    let allergies = sqlx::query_as::<_, Allergy>(
        "SELECT * FROM allergies WHERE patient_id = $1 ORDER BY recorded_date DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let patient_name: String = sqlx::query_scalar("SELECT full_name FROM patients WHERE id = $1")
        .bind(patient_id)
        .fetch_one(state.db())
        .await?;

    let result = allergies
        .iter()
        .map(|a| map_allergy_to_fhir(a, &patient_name))
        .collect();

    record_fhir_audit(
        state.db(),
        user.id,
        patient_id,
        events::FHIR_SEARCH_EXECUTED,
        "AllergyIntolerance",
        &patient_id.to_string(),
    )
    .await;

    Ok(result)
}

pub async fn get_fhir_encounter(
    state: &AppState,
    user: &AuthenticatedUser,
    encounter_id: Uuid,
) -> AppResult<FhirEncounter> {
    let enc = sqlx::query_as::<_, Encounter>("SELECT * FROM encounters WHERE id = $1")
        .bind(encounter_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Encounter resource not found".to_string()))?;

    authorize_patient_access(state, user, enc.patient_id).await?;

    let patient_name: String = sqlx::query_scalar("SELECT full_name FROM patients WHERE id = $1")
        .bind(enc.patient_id)
        .fetch_one(state.db())
        .await?;

    let doctor_name: String = sqlx::query_scalar(
        "SELECT u.full_name FROM doctors d JOIN users u ON u.id = d.user_id WHERE d.id = $1",
    )
    .bind(enc.doctor_id)
    .fetch_one(state.db())
    .await?;

    let fhir_enc = map_encounter_to_fhir(&enc, &patient_name, &doctor_name);

    record_fhir_audit(
        state.db(),
        user.id,
        enc.patient_id,
        events::FHIR_RESOURCE_VIEWED,
        "Encounter",
        &encounter_id.to_string(),
    )
    .await;

    Ok(fhir_enc)
}

pub async fn search_fhir_encounters(
    state: &AppState,
    user: &AuthenticatedUser,
    patient_id: Uuid,
) -> AppResult<Vec<FhirEncounter>> {
    authorize_patient_access(state, user, patient_id).await?;

    let encounters = sqlx::query_as::<_, Encounter>(
        "SELECT * FROM encounters WHERE patient_id = $1 ORDER BY start_time DESC",
    )
    .bind(patient_id)
    .fetch_all(state.db())
    .await?;

    let patient_name: String = sqlx::query_scalar("SELECT full_name FROM patients WHERE id = $1")
        .bind(patient_id)
        .fetch_one(state.db())
        .await?;

    let mut result = Vec::new();
    for enc in encounters {
        let doctor_name: String = sqlx::query_scalar(
            "SELECT u.full_name FROM doctors d JOIN users u ON u.id = d.user_id WHERE d.id = $1",
        )
        .bind(enc.doctor_id)
        .fetch_optional(state.db())
        .await?
        .unwrap_or_else(|| "Dr. Unknown".to_string());

        result.push(map_encounter_to_fhir(&enc, &patient_name, &doctor_name));
    }

    record_fhir_audit(
        state.db(),
        user.id,
        patient_id,
        events::FHIR_SEARCH_EXECUTED,
        "Encounter",
        &patient_id.to_string(),
    )
    .await;

    Ok(result)
}

pub async fn export_patient_fhir_bundle(
    state: &AppState,
    user: &AuthenticatedUser,
    patient_id: Uuid,
) -> AppResult<FhirBundle> {
    authorize_patient_access(state, user, patient_id).await?;

    let patient = sqlx::query_as::<_, Patient>("SELECT * FROM patients WHERE id = $1")
        .bind(patient_id)
        .fetch_optional(state.db())
        .await?
        .ok_or_else(|| AppError::NotFound("Patient not found".to_string()))?;

    let patient_name = patient.full_name.clone();
    let fhir_patient = map_patient_to_fhir(&patient);

    let mut entries = vec![FhirBundleEntry {
        full_url: format!("https://health.gov.pk/fhir/Patient/{}", patient_id),
        resource: serde_json::to_value(&fhir_patient).unwrap(),
    }];

    // Add Conditions
    let conditions =
        sqlx::query_as::<_, Condition>("SELECT * FROM conditions WHERE patient_id = $1")
            .bind(patient_id)
            .fetch_all(state.db())
            .await?;

    for c in conditions {
        let fhir_c = map_condition_to_fhir(&c, &patient_name);
        entries.push(FhirBundleEntry {
            full_url: format!("https://health.gov.pk/fhir/Condition/{}", c.id),
            resource: serde_json::to_value(&fhir_c).unwrap(),
        });
    }

    // Add Allergies
    let allergies = sqlx::query_as::<_, Allergy>("SELECT * FROM allergies WHERE patient_id = $1")
        .bind(patient_id)
        .fetch_all(state.db())
        .await?;

    for a in allergies {
        let fhir_a = map_allergy_to_fhir(&a, &patient_name);
        entries.push(FhirBundleEntry {
            full_url: format!("https://health.gov.pk/fhir/AllergyIntolerance/{}", a.id),
            resource: serde_json::to_value(&fhir_a).unwrap(),
        });
    }

    // Add Encounters
    let encounters =
        sqlx::query_as::<_, Encounter>("SELECT * FROM encounters WHERE patient_id = $1")
            .bind(patient_id)
            .fetch_all(state.db())
            .await?;

    for enc in encounters {
        let doctor_name: String = sqlx::query_scalar(
            "SELECT u.full_name FROM doctors d JOIN users u ON u.id = d.user_id WHERE d.id = $1",
        )
        .bind(enc.doctor_id)
        .fetch_optional(state.db())
        .await?
        .unwrap_or_else(|| "Dr. Unknown".to_string());

        let fhir_enc = map_encounter_to_fhir(&enc, &patient_name, &doctor_name);
        entries.push(FhirBundleEntry {
            full_url: format!("https://health.gov.pk/fhir/Encounter/{}", enc.id),
            resource: serde_json::to_value(&fhir_enc).unwrap(),
        });
    }

    // Add Prescriptions (MedicationRequests)
    let rx_list = crate::prescriptions::list_prescriptions(
        axum::extract::State(state.clone()),
        user.clone(),
        axum::extract::Path(patient_id),
    )
    .await?
    .0;

    for rx in rx_list {
        let doctor_name: String = sqlx::query_scalar(
            "SELECT u.full_name FROM doctors d JOIN users u ON u.id = d.user_id WHERE d.id = $1",
        )
        .bind(rx.prescription.doctor_id)
        .fetch_optional(state.db())
        .await?
        .unwrap_or_else(|| "Dr. Unknown".to_string());

        let requests = map_prescription_to_fhir(&rx, &patient_name, &doctor_name);
        for req in requests {
            entries.push(FhirBundleEntry {
                full_url: format!("https://health.gov.pk/fhir/MedicationRequest/{}", req.id),
                resource: serde_json::to_value(&req).unwrap(),
            });
        }
    }

    // Add Lab Reports (Observations & DiagnosticReports)
    let lab_reports =
        sqlx::query_as::<_, LabReport>("SELECT * FROM lab_reports WHERE patient_id = $1")
            .bind(patient_id)
            .fetch_all(state.db())
            .await?;

    for lab in lab_reports {
        let (obs, diag) = map_lab_report_to_fhir(&lab, &patient_name);
        entries.push(FhirBundleEntry {
            full_url: format!("https://health.gov.pk/fhir/Observation/{}", obs.id),
            resource: serde_json::to_value(&obs).unwrap(),
        });
        entries.push(FhirBundleEntry {
            full_url: format!("https://health.gov.pk/fhir/DiagnosticReport/{}", diag.id),
            resource: serde_json::to_value(&diag).unwrap(),
        });
    }

    let bundle = FhirBundle {
        resource_type: "Bundle".to_string(),
        id: patient_id.to_string(),
        r#type: "collection".to_string(),
        total: entries.len(),
        entry: entries,
    };

    record_fhir_audit(
        state.db(),
        user.id,
        patient_id,
        events::FHIR_BUNDLE_ACCESSED,
        "Bundle",
        &patient_id.to_string(),
    )
    .await;

    Ok(bundle)
}

async fn record_fhir_audit(
    pool: &PgPool,
    actor_id: Uuid,
    patient_id: Uuid,
    event_type: &'static str,
    resource_type: &'static str,
    resource_id: &str,
) {
    let audit = AuditService::new(pool.clone());
    let _ = audit
        .record(
            AuditEvent::new(event_type)
                .actor(actor_id)
                .patient(patient_id)
                .resource(resource_type, resource_id)
                .outcome(OUTCOME_ALLOWED),
        )
        .await;
}
