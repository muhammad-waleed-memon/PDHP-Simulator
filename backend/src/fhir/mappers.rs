//! Deterministic mappers from internal domain models to FHIR R4 resources.

use crate::allergies::Allergy;
use crate::conditions::Condition;
use crate::doctors::{Doctor, DoctorStatus};
use crate::encounters::Encounter;
use crate::facilities::{Facility, FacilityStatus};
use crate::fhir::model::*;
use crate::laboratories::LabReport;
use crate::medications::Medication;
use crate::patient_medications::PatientMedication;
use crate::patients::Patient;
use crate::prescriptions::PrescriptionWithDetails;

pub fn map_patient_to_fhir(p: &Patient) -> FhirPatient {
    let mut identifiers = vec![FhirIdentifier {
        use_type: Some("official".to_string()),
        system: Some("https://health.gov.pk/dhid".to_string()),
        value: p.digital_health_id.clone(),
    }];

    if let Some(user_id) = p.user_id {
        identifiers.push(FhirIdentifier {
            use_type: Some("secondary".to_string()),
            system: Some("https://health.gov.pk/users".to_string()),
            value: user_id.to_string(),
        });
    }

    let telecom = p.phone_number.as_ref().map(|phone| {
        vec![FhirContactPoint {
            system: "phone".to_string(),
            value: phone.clone(),
        }]
    });

    let address = if p.address.is_some() || p.city.is_some() || p.province.is_some() {
        Some(vec![FhirAddress {
            line: p.address.clone().map(|addr| vec![addr]),
            city: p.city.clone(),
            state: p.province.clone(),
            country: Some("Pakistan".to_string()),
        }])
    } else {
        None
    };

    FhirPatient {
        resource_type: "Patient".to_string(),
        id: p.id.to_string(),
        identifier: identifiers,
        active: p.status == "ACTIVE",
        name: vec![FhirHumanName {
            use_type: Some("official".to_string()),
            text: p.full_name.clone(),
        }],
        telecom,
        gender: p.gender.to_lowercase(),
        birth_date: p.date_of_birth.to_string(),
        address,
    }
}

pub fn map_doctor_to_practitioner(d: &Doctor, full_name: &str) -> FhirPractitioner {
    let identifier = d.pmdc_license_number.as_ref().map(|lic| {
        vec![FhirIdentifier {
            use_type: Some("official".to_string()),
            system: Some("https://pmdc.org.pk/license".to_string()),
            value: lic.clone(),
        }]
    });

    let qualification = d
        .qualification
        .as_ref()
        .map(|qual| vec![FhirCodeableConcept::from_text(qual)]);

    FhirPractitioner {
        resource_type: "Practitioner".to_string(),
        id: d.id.to_string(),
        identifier,
        active: d.status == DoctorStatus::Verified,
        name: vec![FhirHumanName {
            use_type: Some("official".to_string()),
            text: full_name.to_string(),
        }],
        qualification,
    }
}

pub fn map_doctor_to_practitioner_role(d: &Doctor, doctor_name: &str) -> FhirPractitionerRole {
    let org_ref = d
        .facility_id
        .map(|fid| FhirReference::new("Organization", &fid.to_string(), None));

    FhirPractitionerRole {
        resource_type: "PractitionerRole".to_string(),
        id: d.id.to_string(),
        active: d.status == DoctorStatus::Verified,
        practitioner: FhirReference::new(
            "Practitioner",
            &d.id.to_string(),
            Some(doctor_name.to_string()),
        ),
        organization: org_ref,
        specialty: Some(vec![FhirCodeableConcept::from_text(&d.specialization)]),
    }
}

pub fn map_facility_to_organization(f: &Facility) -> FhirOrganization {
    let address = if f.address.is_some() || f.city.is_some() || f.province.is_some() {
        Some(vec![FhirAddress {
            line: f.address.clone().map(|addr| vec![addr]),
            city: f.city.clone(),
            state: f.province.clone(),
            country: Some("Pakistan".to_string()),
        }])
    } else {
        None
    };

    FhirOrganization {
        resource_type: "Organization".to_string(),
        id: f.id.to_string(),
        active: f.status == FacilityStatus::Active,
        name: f.name.clone(),
        address,
    }
}

pub fn map_facility_to_location(f: &Facility) -> FhirLocation {
    let address = if f.address.is_some() || f.city.is_some() || f.province.is_some() {
        Some(FhirAddress {
            line: f.address.clone().map(|addr| vec![addr]),
            city: f.city.clone(),
            state: f.province.clone(),
            country: Some("Pakistan".to_string()),
        })
    } else {
        None
    };

    FhirLocation {
        resource_type: "Location".to_string(),
        id: f.id.to_string(),
        status: format!("{:?}", f.status).to_lowercase(),
        name: f.name.clone(),
        address,
        managing_organization: Some(FhirReference::new(
            "Organization",
            &f.id.to_string(),
            Some(f.name.clone()),
        )),
    }
}

pub fn map_encounter_to_fhir(
    e: &Encounter,
    patient_name: &str,
    doctor_name: &str,
) -> FhirEncounter {
    FhirEncounter {
        resource_type: "Encounter".to_string(),
        id: e.id.to_string(),
        status: e.status.to_lowercase(),
        class: FhirCoding {
            system: Some("http://terminology.hl7.org/CodeSystem/v3-ActCode".to_string()),
            code: Some(e.encounter_type.clone()),
            display: Some(e.encounter_type.clone()),
        },
        subject: FhirReference::new(
            "Patient",
            &e.patient_id.to_string(),
            Some(patient_name.to_string()),
        ),
        participant: Some(vec![FhirEncounterParticipant {
            individual: FhirReference::new(
                "Practitioner",
                &e.doctor_id.to_string(),
                Some(doctor_name.to_string()),
            ),
        }]),
        service_provider: Some(FhirReference::new(
            "Organization",
            &e.facility_id.to_string(),
            None,
        )),
        period: FhirPeriod {
            start: Some(e.start_time.to_rfc3339()),
            end: e.end_time.map(|t| t.to_rfc3339()),
        },
        reason_code: Some(vec![FhirCodeableConcept::from_text(&e.reason)]),
    }
}

pub fn map_condition_to_fhir(c: &Condition, patient_name: &str) -> FhirCondition {
    FhirCondition {
        resource_type: "Condition".to_string(),
        id: c.id.to_string(),
        clinical_status: FhirCodeableConcept::from_text(&c.status),
        code: FhirCodeableConcept {
            coding: c.code.as_ref().map(|cd| {
                vec![FhirCoding {
                    system: Some("http://hl7.org/fhir/sid/icd-10".to_string()),
                    code: Some(cd.clone()),
                    display: Some(c.name.clone()),
                }]
            }),
            text: Some(c.name.clone()),
        },
        subject: FhirReference::new(
            "Patient",
            &c.patient_id.to_string(),
            Some(patient_name.to_string()),
        ),
        onset_date_time: c.onset_date.map(|d| d.to_string()),
        recorded_date: Some(c.created_at.to_rfc3339()),
    }
}

pub fn map_allergy_to_fhir(a: &Allergy, patient_name: &str) -> FhirAllergyIntolerance {
    let reaction = a.reaction.as_ref().map(|react| {
        vec![FhirAllergyReaction {
            manifest: Some(vec![FhirCodeableConcept::from_text(react)]),
            severity: Some(a.severity.to_lowercase()),
        }]
    });

    FhirAllergyIntolerance {
        resource_type: "AllergyIntolerance".to_string(),
        id: a.id.to_string(),
        clinical_status: FhirCodeableConcept::from_text(&a.status),
        code: FhirCodeableConcept::from_text(&a.allergen),
        patient: FhirReference::new(
            "Patient",
            &a.patient_id.to_string(),
            Some(patient_name.to_string()),
        ),
        recorded_date: Some(a.created_at.to_rfc3339()),
        reaction,
    }
}

pub fn map_medication_to_fhir(m: &Medication) -> FhirMedication {
    FhirMedication {
        resource_type: "Medication".to_string(),
        id: m.id.to_string(),
        code: FhirCodeableConcept {
            coding: m.generic_name.as_ref().map(|gen| {
                vec![FhirCoding {
                    system: None,
                    code: None,
                    display: Some(gen.clone()),
                }]
            }),
            text: Some(m.name.clone()),
        },
        status: m.status.to_lowercase(),
    }
}

pub fn map_patient_medication_to_fhir(
    pm: &PatientMedication,
    patient_name: &str,
    med_name: &str,
) -> FhirMedicationStatement {
    FhirMedicationStatement {
        resource_type: "MedicationStatement".to_string(),
        id: pm.id.to_string(),
        status: pm.status.to_lowercase(),
        medication_reference: FhirReference::new(
            "Medication",
            &pm.medication_id.to_string(),
            Some(med_name.to_string()),
        ),
        subject: FhirReference::new(
            "Patient",
            &pm.patient_id.to_string(),
            Some(patient_name.to_string()),
        ),
        effective_period: Some(FhirPeriod {
            start: Some(pm.start_date.to_string()),
            end: pm.end_date.map(|d| d.to_string()),
        }),
        dosage: Some(vec![FhirDosage {
            text: format!("{} {}", pm.dosage, pm.frequency),
        }]),
    }
}

pub fn map_prescription_to_fhir(
    rx: &PrescriptionWithDetails,
    patient_name: &str,
    doctor_name: &str,
) -> Vec<FhirMedicationRequest> {
    let mut requests = Vec::new();

    for item_detail in &rx.items {
        let it = &item_detail.item;
        requests.push(FhirMedicationRequest {
            resource_type: "MedicationRequest".to_string(),
            id: format!("{}-{}", rx.prescription.id, it.id),
            status: rx.prescription.status.to_lowercase(),
            intent: "order".to_string(),
            medication_reference: FhirReference::new(
                "Medication",
                &it.medication_id.to_string(),
                Some(item_detail.medication_name.clone()),
            ),
            subject: FhirReference::new(
                "Patient",
                &rx.prescription.patient_id.to_string(),
                Some(patient_name.to_string()),
            ),
            requester: FhirReference::new(
                "Practitioner",
                &rx.prescription.doctor_id.to_string(),
                Some(doctor_name.to_string()),
            ),
            authored_on: rx.prescription.issued_at.to_rfc3339(),
            dosage_instruction: Some(vec![FhirDosage {
                text: format!("{} {} for {}", it.dosage, it.frequency, it.duration),
            }]),
            note: rx
                .prescription
                .clinical_notes
                .as_ref()
                .map(|n| vec![n.clone()]),
        });
    }

    requests
}

pub fn map_lab_report_to_fhir(
    report: &LabReport,
    patient_name: &str,
) -> (FhirObservation, FhirDiagnosticReport) {
    let val_num = report.result.parse::<f64>().ok();

    let obs = FhirObservation {
        resource_type: "Observation".to_string(),
        id: format!("obs-{}", report.id),
        status: report.status.to_lowercase(),
        category: vec![FhirCodeableConcept::from_text("laboratory")],
        code: FhirCodeableConcept::from_text(&report.test_name),
        subject: FhirReference::new(
            "Patient",
            &report.patient_id.to_string(),
            Some(patient_name.to_string()),
        ),
        effective_date_time: report.report_date.to_rfc3339(),
        value_string: if val_num.is_none() {
            Some(report.result.clone())
        } else {
            Some(format!(
                "{} {}",
                report.result,
                report.unit.as_deref().unwrap_or("")
            ))
        },
        interpretation: if report.abnormal_flag {
            Some(vec![FhirCodeableConcept::from_text("Abnormal")])
        } else {
            None
        },
    };

    let diag = FhirDiagnosticReport {
        resource_type: "DiagnosticReport".to_string(),
        id: report.id.to_string(),
        status: report.status.to_lowercase(),
        code: FhirCodeableConcept::from_text(&report.test_name),
        subject: FhirReference::new(
            "Patient",
            &report.patient_id.to_string(),
            Some(patient_name.to_string()),
        ),
        effective_date_time: report.report_date.to_rfc3339(),
        issued: report.created_at.to_rfc3339(),
        result: Some(vec![FhirReference::new(
            "Observation",
            &format!("obs-{}", report.id),
            Some(report.test_name.clone()),
        )]),
    };

    (obs, diag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_patient_mapping_structure() {
        let patient = Patient {
            id: Uuid::new_v4(),
            digital_health_id: "PK-HID-8F3A-4B2C-9E10".to_string(),
            user_id: Some(Uuid::new_v4()),
            full_name: "Tariq Mahmood".to_string(),
            date_of_birth: chrono::NaiveDate::from_ymd_opt(1985, 10, 20).unwrap(),
            gender: "Male".to_string(),
            blood_group: Some("O+".to_string()),
            phone_number: Some("+92-300-1234567".to_string()),
            address: Some("Street 5".to_string()),
            city: Some("Islamabad".to_string()),
            province: Some("ICT".to_string()),
            emergency_contact_name: None,
            emergency_contact_phone: None,
            status: "ACTIVE".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let fhir = map_patient_to_fhir(&patient);
        assert_eq!(fhir.resource_type, "Patient");
        assert_eq!(fhir.id, patient.id.to_string());
        assert_eq!(fhir.identifier[0].value, "PK-HID-8F3A-4B2C-9E10");
        assert_eq!(fhir.gender, "male");
    }

    #[test]
    fn test_operation_outcome_serialization() {
        let outcome = FhirOperationOutcome::error("forbidden", "Access denied");
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains("OperationOutcome"));
        assert!(json.contains("forbidden"));
        assert!(json.contains("Access denied"));
    }
}
