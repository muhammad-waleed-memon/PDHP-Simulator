# ARCHITECTURE
# Pakistan Digital Health Identity & Longitudinal Medical Record Platform
# ============================================================

> **Prototype Notice:** This is an engineering prototype for demonstration and
> validation. It is not a certified medical device, clinically validated software,
> or production-ready national healthcare system.

---

## 1. System Overview

```
┌─────────────────────────────────────────────────────────┐
│                        Browser                          │
└─────────────────────────┬───────────────────────────────┘
                          │ HTTP :80
┌─────────────────────────▼───────────────────────────────┐
│                      Nginx                              │
│         (Reverse proxy + static file serving)           │
│  /api/*  →  backend:8080                                │
│  /fhir/* →  backend:8080                                │
│  /*      →  frontend static files                       │
└──────────┬──────────────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────────────┐
│               Rust / Axum Backend                       │
│                   (Port 8080)                           │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │               Router / Middleware               │    │
│  │  Auth JWT Extractor | RBAC | Audit | CORS       │    │
│  └──────────┬────────────────────┬─────────────────┘    │
│             │                    │                       │
│  ┌──────────▼──────┐   ┌─────────▼───────────────────┐  │
│  │   /api/v1/*     │   │       /fhir/*               │  │
│  │  REST Handlers  │   │   FHIR Adapter Layer        │  │
│  └──────────┬──────┘   └─────────────────────────────┘  │
│             │                                            │
│  ┌──────────▼────────────────────────────────────────┐   │
│  │              Service / Domain Layer               │   │
│  │  auth | patients | doctors | encounters |        │   │
│  │  conditions | allergies | medications |          │   │
│  │  prescriptions | labs | consent | audit |        │   │
│  │  clinical_alerts | fhir                          │   │
│  └──────────┬────────────────────────────────────────┘  │
└─────────────┼───────────────────────────────────────────┘
              │
   ┌──────────┴──────────┐
   │                     │
┌──▼──────────┐   ┌──────▼───────┐
│ PostgreSQL  │   │    MinIO     │
│  Port 5432  │   │  Port 9000   │
│ (Primary DB)│   │(Object store)│
└─────────────┘   └──────────────┘
```

---

## 2. Repository Structure

```
/
├── backend/                    # Rust / Axum application
│   ├── Cargo.toml
│   ├── Dockerfile
│   ├── migrations/             # SQLx database migrations
│   └── src/
│       ├── main.rs             # Entry point
│       ├── config.rs           # Environment configuration
│       ├── state.rs            # Shared application state
│       ├── error.rs            # Centralized error types
│       ├── router.rs           # Route assembly
│       ├── db.rs               # Database pool
│       ├── health.rs           # /health endpoints
│       ├── auth/               # Authentication & JWT
│       ├── users/              # User model & management
│       ├── patients/           # Patient identity & records
│       ├── doctors/            # Doctor/provider identity
│       ├── facilities/         # Healthcare facilities
│       ├── encounters/         # Clinical encounters
│       ├── conditions/         # Medical conditions
│       ├── allergies/          # Patient allergies
│       ├── medications/        # Medication catalog
│       ├── prescriptions/      # Prescriptions + items
│       ├── laboratories/       # Lab reports
│       ├── consent/            # Patient consent management
│       ├── audit/              # Audit logging
│       ├── clinical_alerts/    # Alert engine
│       └── fhir/               # FHIR adapter layer
│
├── frontend/                   # React + TypeScript + Vite
│   ├── Dockerfile
│   ├── package.json
│   ├── vite.config.ts
│   ├── tailwind.config.ts
│   └── src/
│       ├── main.tsx
│       ├── App.tsx             # Main application with role-based routing
│       ├── index.css           # Global styles
│       ├── api/                # API client (client.ts)
│       ├── components/         # ClinicalDashboard, FhirViewer, etc.
│       └── context/            # AuthContext.tsx
│
├── infrastructure/
│   └── nginx/
│       └── nginx.conf          # Reverse proxy + rate limiting
│
├── docs/
│   └── fhir.md                 # FHIR implementation guidance
│
├── scripts/
│   ├── seed.sh                 # Demo seeding reference (auto-seeded by backend)
│   └── reset-db.sh             # Development database reset
│
├── docker-compose.yml
├── .env.example
├── .gitignore
├── README.md
├── PROJECT_STATUS.md
├── ARCHITECTURE.md
└── SECURITY.md
```

---

## 3. Backend Module Architecture

Each domain module follows a consistent internal structure:

```
module_name/
├── mod.rs          # Public interface + sub-module declarations
├── routes.rs       # Axum route handlers (HTTP layer only)
├── service.rs      # Business logic (no HTTP types)
├── model.rs        # Database model structs (SQLx)
└── dto.rs          # Request/Response DTOs (Serde)
```

**Key principle:** Route handlers only deserialize, delegate to service, serialize response. Business logic lives in the service layer.

---

## 4. Database Architecture

### Primary Key Strategy
- All tables use `UUID` (v4) as primary key
- Digital Health ID is a separate application-level opaque identifier (e.g., `DHI-XXXXXXXX`)

### Core Tables

```sql
-- Identity & Auth
users               -- Login credentials + role
patients            -- Patient identity + digital health ID
doctors             -- Doctor/provider identity
facilities          -- Healthcare organizations

-- Medical Records
conditions          -- Diagnosed conditions (versioned)
allergies           -- Patient allergy records
medications         -- Medication catalog
patient_medications -- Active/historical medication assignments
encounters          -- Clinical encounters
prescriptions       -- Prescriptions header
prescription_items  -- Individual prescription line items
lab_reports         -- Laboratory test results

-- Governance
consents            -- Patient consent grants/revocations
audit_logs          -- Append-only security audit trail
```

### Versioning Approach
Clinically significant records (`conditions`, `prescriptions`) implement soft-history:
- `is_current: bool` flag
- `version: i32` counter
- `previous_version_id: UUID` link

### Timestamp Convention
All tables include:
- `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`

---

## 5. Authentication

```
POST /api/v1/auth/register
POST /api/v1/auth/login
POST /api/v1/auth/logout
POST /api/v1/auth/refresh
```

### Flow
```
Client → POST /auth/login { username, password }
       → Argon2 verify password hash
       → Generate JWT (HS256) with claims: user_id, role, exp
       → Return { access_token, refresh_token, expires_in }

Client → GET /api/v1/patients (Authorization: Bearer <token>)
       → JWT middleware extracts + validates token
       → RBAC middleware checks role permission
       → Handler executes
```

### JWT Claims
```json
{
  "sub": "<user_uuid>",
  "role": "DOCTOR",
  "exp": 1234567890,
  "iat": 1234567890,
  "jti": "<token_uuid>"
}
```

### Password Storage
Passwords are hashed with **Argon2id** (PHC winner). Plaintext passwords are never stored, logged, or transmitted after receipt.

---

## 6. Role-Based Access Control (RBAC)

### Roles

| Role | Description |
|---|---|
| `PATIENT` | Access own records; manage own consent |
| `DOCTOR` | Access authorized patient records; create encounters/prescriptions |
| `LAB` | Create/view permitted lab reports |
| `FACILITY_ADMIN` | Administrative functions; no standing clinical access |
| `SYSTEM_ADMIN` | Infrastructure/account management; no standing clinical content access |

### Enforcement
RBAC is enforced at the **backend middleware layer**, not the frontend. The frontend may hide UI elements, but the backend always validates independently.

### Resource-Level Access
Beyond role checks, patient data requires one of:
1. The user IS the patient
2. The patient has granted consent to this doctor
3. A valid break-glass access record exists

---

## 7. Digital Health ID

Format: `PK-HID-XXXX-XXXX-XXXX` where each `XXXX` segment is a cryptographically random uppercase hexadecimal group.

- Generated server-side at patient registration
- Unique constraint enforced at the database level (`UNIQUE` index on `digital_health_id`)
- Not derived from CNIC or any personal identifier
- Stable (never reassigned after issuance)
- Used for cross-facility patient lookup

---

## 8. Consent Model

```
Patient ──grants──► Consent Record
                        │
                        ├── provider_id (Doctor UUID)
                        ├── facility_id (optional)
                        ├── purpose (TREATMENT | RESEARCH | EMERGENCY)
                        ├── scope (FULL | MEDICATIONS | LABS | ENCOUNTERS)
                        ├── granted_at
                        ├── expires_at (optional)
                        ├── revoked_at (optional)
                        └── status (ACTIVE | EXPIRED | REVOKED)
```

Backend enforces: before any doctor accesses protected patient data, consent status is checked. A `REVOKED` or `EXPIRED` consent results in `403 Forbidden`.

---

## 9. Break-Glass Emergency Access

```
Doctor → POST /api/v1/emergency/break-glass
         { patient_id, reason, expected_duration_hours }
       → Validates doctor role
       → Creates timed BreakGlassRecord (max 4 hours)
       → Creates AUDIT event with reason
       → Returns temporary access token
       → All subsequent access tagged as BREAK_GLASS
       → Generates alert notification for patient + admin
```

Break-glass is explicitly triggered, time-limited, reason-required, and generates immutable audit records distinguishable from normal access.

---

## 10. Audit Logging

Every security-sensitive action generates an `audit_log` record:

```sql
CREATE TABLE audit_logs (
    id              UUID PRIMARY KEY,
    event_id        UUID UNIQUE NOT NULL,    -- correlation
    actor_id        UUID,                    -- who did it
    actor_role      TEXT,
    action          TEXT NOT NULL,           -- e.g. PATIENT_VIEWED
    resource_type   TEXT,                    -- e.g. patient
    resource_id     UUID,
    outcome         TEXT NOT NULL,           -- SUCCESS | FAILURE
    reason          TEXT,                    -- for break-glass
    ip_address      TEXT,
    request_id      UUID,
    previous_hash   TEXT,                    -- chain integrity
    record_hash     TEXT,                    -- hash of this record
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

The audit log is **append-only** — records are never modified or deleted. Hash chaining provides integrity verification.

---

## 11. Clinical Alert Engine

The alert engine is a deterministic rule engine (not AI). It checks prescriptions against:

1. **Allergy Conflict:** Does any ingredient of the prescribed medication match a recorded patient allergy?
2. **Duplicate Medication:** Is an equivalent medication already in the active prescription list?

Alerts are **warnings** — the clinician retains final authority. The alert and the clinician's override decision are both audited.

---

## 12. FHIR Integration

### Approach
The internal domain model is **not** FHIR. An adapter layer maps internal records to FHIR R4 JSON on demand.

```
Internal Record → FHIR Mapper → FHIR Resource JSON
```

### Implemented Resources (Phase 5 — Complete)
- `Patient`
- `Practitioner` / `PractitionerRole`
- `Organization` / `Location`
- `Encounter`
- `Condition`
- `AllergyIntolerance`
- `Medication` / `MedicationStatement` / `MedicationRequest`
- `Observation`
- `DiagnosticReport`
- `Bundle` (for `$everything` export)
- `OperationOutcome` (error responses)

### Endpoints
```
GET  /fhir/Patient/{id}
GET  /fhir/Patient/{id}/$everything
GET  /fhir/Practitioner/{id}
GET  /fhir/PractitionerRole/{id}
GET  /fhir/Organization/{id}
GET  /fhir/Location/{id}
GET  /fhir/Encounter/{id}
GET  /fhir/Condition/{id}
GET  /fhir/AllergyIntolerance/{id}
GET  /fhir/Medication/{id}
GET  /fhir/MedicationStatement/{id}
GET  /fhir/MedicationRequest/{id}
GET  /fhir/Observation/{id}
GET  /fhir/DiagnosticReport/{id}
```

### Compliance Notice
This prototype implements a **subset of FHIR R4** for demonstration purposes. It has not been validated against an official FHIR test server. Full compliance is a future production milestone.

---

## 13. Docker Deployment

```yaml
services:
  postgres:     # PostgreSQL 16 with persistent volume
  backend:      # Rust release binary (non-root)
  frontend:     # Nginx serving React build
  nginx:        # Reverse proxy (port 80)
  minio:        # Object storage (port 9000)
```

### Startup Order
```
postgres (healthy) → backend → frontend + nginx
```

Database readiness is verified using `pg_isready` in the healthcheck, not merely container startup.

---

## 14. Prototype Status & Future Evolution

**This is a complete Local Docker Prototype.** It is not production-ready, nationally deployed, or clinically validated.

The architecture uses a **Modular Monolith** backend. Individual modules are designed so they could be extracted into separate services if the system were developed further:

| Module | Potential Future Service |
|---|---|
| auth | Auth Service |
| fhir | FHIR Gateway Service |
| audit | Audit Streaming Service |
| notifications | Notification Service |
| laboratories | Lab Integration Service |

The PostgreSQL schema is designed to support this extraction without breaking foreign key semantics.

**The prototype is complete. No further phases are planned.**
