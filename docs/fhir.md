# Pakistan Digital Health Platform — FHIR R4 Interoperability Guidance

> **Disclaimer**: This documentation represents the **Project Prototype FHIR Implementation Guidance** for the Digital Health Identity and Longitudinal Medical Record Platform for Pakistan. It is a standards-oriented adaptation layer for interoperability and exchange, **NOT** an official Pakistani national FHIR profile standard.

---

## 1. Overview & Architecture

The FHIR Interoperability Layer operates as an adaptation overlay above the platform's internal PostgreSQL database domain model:

```
Internal Domain Model (PostgreSQL - Source of Truth)
        ↓
FHIR Mapping / Adapter Layer (backend/src/fhir/mappers.rs)
        ↓
FHIR R4 Resource Representation (backend/src/fhir/model.rs)
        ↓
FHIR R4 API Routes (/fhir/*)
        ↓
External Interoperable Systems / Authorized Clients
```

### Key Architectural Constraints:
- **Exchange Representation**: The internal PostgreSQL schema remains the single source of truth. Internal tables are not renamed or restructured to mirror FHIR.
- **FHIR Version**: **HL7 FHIR R4 (v4.0.1)** (`FHIR_VERSION=R4`).
- **Read/Exchange Orientation**: Write interoperability is intentionally deferred in this phase to enforce domain validation rules via native REST endpoints.
- **Zero Security Bypass**: Every FHIR request undergoes JWT authentication, RBAC authorization, patient-level consent validation (`authorize_patient_access`), and append-only tamper-evident audit logging.

---

## 2. Supported FHIR R4 Resources

| Domain Entity | FHIR R4 Resource | Endpoints | Search Parameters |
|---|---|---|---|
| Patient | `Patient` | `GET /fhir/Patient/{id}`, `GET /fhir/Patient/{id}/$everything` | `identifier`, `name` |
| Doctor | `Practitioner` | `GET /fhir/Practitioner/{id}` | - |
| Doctor + Facility | `PractitionerRole` | `GET /fhir/PractitionerRole/{id}` | - |
| Healthcare Facility | `Organization` | `GET /fhir/Organization/{id}` | - |
| Facility Location | `Location` | `GET /fhir/Location/{id}` | - |
| Encounter | `Encounter` | `GET /fhir/Encounter/{id}` | `patient` |
| Medical Condition | `Condition` | `GET /fhir/Condition/{id}` | `patient` |
| Allergy | `AllergyIntolerance` | `GET /fhir/AllergyIntolerance/{id}` | `patient` |
| Medication Catalog | `Medication` | `GET /fhir/Medication/{id}` | - |
| Patient Medication History | `MedicationStatement` | `GET /fhir/MedicationStatement/{id}` | `patient` |
| Issued Prescription | `MedicationRequest` | `GET /fhir/MedicationRequest/{id}` | `patient` |
| Lab Result | `Observation` | `GET /fhir/Observation/{id}` | `patient` |
| Lab Report | `DiagnosticReport` | `GET /fhir/DiagnosticReport/{id}` | `patient` |

---

## 3. Identifiers & Terminology Strategy

### Identifier Strategy
- **Resource IDs**: Stable UUIDs from the internal PostgreSQL database are used directly as FHIR resource IDs (`Patient/123e4567-e89b-12d3-a456-426614174000`).
- **Digital Health ID**: Represented as a system-bound `identifier` array entry:
  - System: `https://health.gov.pk/fhir/NamingSystem/digital-health-id`
  - Value: `PK-DHID-XXXXXX`
- **Privacy & Security**: Internal credentials, password hashes, and sensitive secrets are strictly excluded. Primary CNIC is **not** exposed as the primary FHIR resource ID.

### Terminology Strategy
- **Preservation of Raw Data**: Where coding systems (e.g., SNOMED CT, LOINC, RxNorm) are not yet integrated into the internal database, raw clinical strings are preserved as `text` fields inside `CodeableConcept` DTOs.
- **No Fabricated Codes**: Synthetic or random SNOMED/ICD codes are **never** invented. Terminology server bindings are reserved for future governance phases.

---

## 4. Security, Consent & Audit Integration

### Access Control Pipeline
```
Request → JWT Authentication
        → RBAC Role Check
        → authorize_patient_access (Role + Active Consent matrix)
        → Resource Mapping & Serialization
        → Audit Log Emission (FHIR_RESOURCE_VIEWED / FHIR_SEARCH_EXECUTED)
        → HTTP 200 OK Response (application/fhir+json)
```

### Security Audit Events
FHIR access triggers specialized audit events:
- `FHIR_RESOURCE_VIEWED`
- `FHIR_SEARCH_EXECUTED`
- `FHIR_BUNDLE_ACCESSED`
- `FHIR_ACCESS_DENIED`
- `FHIR_VALIDATION_FAILED`

Audit logs record actor ID, role, resource type, target resource ID, patient ID, and outcome while strictly excluding tokens, payloads, or credentials.

---

## 5. Sample FHIR R4 JSON Responses (Demo Data)

### 5.1 Patient Resource (`GET /fhir/Patient/{id}`)

```json
{
  "resourceType": "Patient",
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "identifier": [
    {
      "system": "https://health.gov.pk/fhir/NamingSystem/digital-health-id",
      "value": "PK-DHID-872391"
    }
  ],
  "active": true,
  "name": [
    {
      "use": "official",
      "text": "Fatima Ali"
    }
  ],
  "gender": "female",
  "birthDate": "1992-05-14",
  "address": [
    {
      "city": "Lahore",
      "state": "Punjab"
    }
  ],
  "contact": [
    {
      "name": {
        "text": "Tariq Ali"
      },
      "telecom": [
        {
          "system": "phone",
          "value": "+92-300-1234567"
        }
      ]
    }
  ]
}
```

### 5.2 Patient Summary Bundle (`GET /fhir/Patient/{id}/$everything`)

```json
{
  "resourceType": "Bundle",
  "type": "collection",
  "total": 5,
  "entry": [
    {
      "fullUrl": "Patient/a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "resource": {
        "resourceType": "Patient",
        "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "name": [{ "text": "Fatima Ali" }]
      }
    },
    {
      "fullUrl": "Condition/c1c2c3c4-0000-0000-0000-000000000001",
      "resource": {
        "resourceType": "Condition",
        "id": "c1c2c3c4-0000-0000-0000-000000000001",
        "clinicalStatus": { "coding": [{ "code": "active" }] },
        "code": { "text": "Essential Hypertension" },
        "subject": { "reference": "Patient/a1b2c3d4-e5f6-7890-abcd-ef1234567890" }
      }
    }
  ]
}
```

### 5.3 Error Outcome (`OperationOutcome`)

```json
{
  "resourceType": "OperationOutcome",
  "issue": [
    {
      "severity": "error",
      "code": "forbidden",
      "diagnostics": "Access denied: active patient consent required"
    }
  ]
}
```

---

## 6. Interoperability Demonstration Script

Below is a step-by-step cURL verification flow:

### Step 1: Doctor Login
```bash
curl -X POST http://localhost/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"doctor@demo.local","password":"<DEMO_DOCTOR_PASSWORD from .env>"}'
# Export returned JWT TOKEN
export TOKEN="<jwt_token_here>"
```

### Step 2: Fetch FHIR Patient Resource
```bash
curl -X GET http://localhost/fhir/Patient/<patient-uuid> \
  -H "Authorization: Bearer $TOKEN" \
  -H "Accept: application/fhir+json"
```

### Step 3: Export Patient Everything Bundle
```bash
curl -X GET "http://localhost/fhir/Patient/<patient-uuid>/\$everything" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Accept: application/fhir+json"
```

### Step 4: Search Conditions by Patient UUID
```bash
curl -X GET http://localhost/fhir/Condition/<condition-uuid> \
  -H "Authorization: Bearer $TOKEN" \
  -H "Accept: application/fhir+json"
```

---

## 7. Limitations & Deferred Capabilities
1. **FHIR Writes**: FHIR write operations (`POST`, `PUT`, `DELETE`) are intentionally deferred to ensure internal clinical business rules (such as prescription safety alert checks and version increments) are strictly enforced through native REST endpoints.
2. **XML Representation**: FHIR JSON (`application/fhir+json`) is fully supported. XML format (`application/fhir+xml`) is unsupported and returns HTTP 406 Not Acceptable with an `OperationOutcome`.
3. **External Terminology Server**: Direct SNOMED CT / LOINC terminology validation against external terminology servers is reserved for future national infrastructure phases.
