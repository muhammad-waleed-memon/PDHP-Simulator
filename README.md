# Pakistan Digital Health Identity & Longitudinal Medical Record Platform

> **⚠️ Prototype Notice:** This is an engineering prototype for demonstration
> and educational validation. It is **not** a certified medical device, clinically
> validated software, or production-ready national healthcare system.
> Clinical alerts are decision-support demonstrations only.
> This prototype has not undergone professional penetration testing,
> regulatory review, or clinical safety validation.

---

## Project Purpose

A secure, patient-centered digital health identity and longitudinal medical record
platform prototype designed for Pakistan, demonstrating:

- **Digital Health Identity (DHI)** — opaque, server-generated identity in format `PK-HID-XXXX-XXXX-XXXX`
- **Longitudinal medical record keeping** — encounters, conditions, allergies, medications, prescriptions, laboratory reports
- **Role-based access control** — PATIENT, DOCTOR, LAB, FACILITY_ADMIN, SYSTEM_ADMIN
- **Patient consent management** — grant, revoke, and enforce access consent at the backend
- **Clinical safety alerts** — allergy conflict detection and duplicate medication detection
- **FHIR R4 interoperability API** — read-oriented FHIR adapter layer over the internal domain model
- **SHA-256 hash-chained audit logging** — append-only, tamper-evident audit trail

---

## Quick Start

### Prerequisites

- **Docker Desktop** (Windows/Mac) or **Docker Engine + Compose plugin** (Linux)
- **Git**

**No Rust, Node.js, or PostgreSQL installation required on the host machine.**

### Setup and Start

```bash
# 1. Clone the repository
git clone <repository-url>
cd <repository-directory>

# 2. Create your environment file (defaults work for local development)
cp .env.example .env

# 3. Build and start all services
docker compose up --build
```

The application will be available at:

| Service         | URL                          |
|-----------------|------------------------------|
| Frontend        | <http://localhost>           |
| API             | <http://localhost/api/v1>    |
| FHIR            | <http://localhost/fhir>      |
| MinIO Console   | <http://localhost:9001>      |

### Health Checks

```bash
curl http://localhost/health
curl http://localhost/health/ready
```

### Shutdown

```bash
# Stop services (preserves database data)
docker compose down

# Stop services and delete all data (full reset)
docker compose down -v
```

---

## Architecture Summary

```
Browser → Nginx (port 80) → Rust/Axum Backend (port 8080)
                                      ↓              ↓
                               PostgreSQL 16      MinIO
                               (port 5432)    (port 9000)
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the complete architecture documentation.

---

## Technology Stack

| Layer          | Technology                              |
|----------------|-----------------------------------------|
| Backend        | Rust, Axum, Tokio, SQLx                 |
| Database       | PostgreSQL 16                           |
| Auth           | Argon2id password hashing + JWT (HS256) |
| Frontend       | React 18, TypeScript, Vite, Tailwind CSS|
| Reverse Proxy  | Nginx 1.25                              |
| Object Storage | MinIO                                   |
| Containers     | Docker + Docker Compose                 |

---

## Environment Variables

Copy `.env.example` to `.env` before running. The defaults work for local development.

Key variables:

| Variable                     | Description                                               |
|------------------------------|-----------------------------------------------------------|
| `DATABASE_URL`               | PostgreSQL connection string                              |
| `JWT_SECRET`                 | JWT signing secret — **change in any non-demo use**       |
| `APP_ENV`                    | `development` enables demo seeding; `production` disables |
| `RUST_LOG`                   | Log level filter                                          |
| `SEED_DEMO_DATA`             | `true` seeds demo accounts on startup                     |
| `DEMO_PATIENT_PASSWORD`      | Password for `patient@demo.local`                         |
| `DEMO_DOCTOR_PASSWORD`       | Password for `doctor@demo.local`                          |
| `DEMO_LAB_PASSWORD`          | Password for `lab@demo.local`                             |
| `DEMO_FACILITY_ADMIN_PASSWORD` | Password for `facilityadmin@demo.local`               |
| `DEMO_SYSTEM_ADMIN_PASSWORD` | Password for `sysadmin@demo.local`                        |
| `MINIO_ACCESS_KEY`           | MinIO access key                                          |
| `MINIO_SECRET_KEY`           | MinIO secret key                                          |

See `.env.example` for the complete list with descriptions.

---

## Database

- PostgreSQL 16 managed by Docker Compose
- Migrations are managed by **SQLx** (located in `backend/migrations/`)
- Migrations run **automatically** on backend startup
- Demo data is seeded automatically when `APP_ENV=development` and `SEED_DEMO_DATA=true`

### Reset Database (Development)

```bash
# This destroys all data and starts fresh
docker compose down -v
docker compose up --build
```

---

## Demo Accounts

The application seeds demo accounts automatically in development mode.

> **⚠️ Fictional Demo Notice:** All demo accounts use fictional `@demo.local`
> email addresses. Passwords are set via environment variables.
> No real patient or provider information is used.

| Role            | Email                         | Password Env Variable            |
|-----------------|-------------------------------|----------------------------------|
| Patient         | `patient@demo.local`          | `DEMO_PATIENT_PASSWORD`          |
| Doctor          | `doctor@demo.local`           | `DEMO_DOCTOR_PASSWORD`           |
| Lab Technician  | `lab@demo.local`              | `DEMO_LAB_PASSWORD`              |
| Facility Admin  | `facilityadmin@demo.local`    | `DEMO_FACILITY_ADMIN_PASSWORD`   |
| System Admin    | `sysadmin@demo.local`         | `DEMO_SYSTEM_ADMIN_PASSWORD`     |

Default passwords are shown in `.env.example`. Change them by editing your `.env` file.

---

## API Reference

### Authentication

```
POST /api/v1/auth/register    Register a new user
POST /api/v1/auth/login       Login and receive a JWT token
GET  /api/v1/auth/me          Get current authenticated user profile
```

All protected endpoints require: `Authorization: Bearer <token>`

### Internal REST APIs

```
/api/v1/patients/             Patient identity & Digital Health ID
/api/v1/doctors/              Doctor/provider identity
/api/v1/facilities/           Healthcare facilities
/api/v1/encounters/           Clinical encounters
/api/v1/conditions/           Medical conditions
/api/v1/allergies/            Patient allergies
/api/v1/medications/          Medication catalog
/api/v1/prescriptions/        Prescriptions
/api/v1/lab-reports/          Laboratory reports
/api/v1/timeline/             Aggregated patient timeline
/api/v1/consent/              Patient consent management
/api/v1/audit/                Audit log access
```

### FHIR R4 Interoperability (Read-Only)

```
GET /fhir/Patient/{id}
GET /fhir/Patient/{id}/$everything       Full patient summary bundle
GET /fhir/Practitioner/{id}
GET /fhir/Organization/{id}
GET /fhir/Encounter/{id}
GET /fhir/Condition/{id}
GET /fhir/AllergyIntolerance/{id}
GET /fhir/Medication/{id}
GET /fhir/MedicationRequest/{id}
GET /fhir/DiagnosticReport/{id}
```

All FHIR endpoints require authentication, RBAC authorization, and patient consent.
See [docs/fhir.md](docs/fhir.md) for full FHIR documentation.

---

## Frontend Portals

| Portal         | Users           | Key Features                                          |
|----------------|-----------------|-------------------------------------------------------|
| Patient Portal | PATIENT         | Profile, Digital Health ID, consent management, medical history, FHIR view |
| Doctor Portal  | DOCTOR          | Patient lookup, encounters, prescriptions, safety alerts |
| Lab Portal     | LAB             | Lab report creation and viewing                       |
| Admin Portal   | FACILITY_ADMIN, SYSTEM_ADMIN | User management, audit log viewer        |

---

## Main Workflows

### PATIENT Workflow
1. Register or login → receive JWT
2. View Digital Health ID (`PK-HID-XXXX-XXXX-XXXX`)
3. View patient profile (`GET /api/v1/patients/me`)
4. Grant or revoke consent to a doctor (`POST /api/v1/consent/grant`)
5. View clinical history — conditions, allergies, medications, prescriptions, lab reports
6. View aggregated longitudinal timeline
7. Access FHIR Patient resource (`GET /fhir/Patient/{id}`)

### DOCTOR Workflow
1. Login → receive JWT
2. Search patients by Digital Health ID
3. Access patient records (requires active patient consent)
4. Create encounters, record conditions, allergies
5. Issue prescriptions (backend checks for allergy conflicts and duplicate medications)
6. View lab reports for consented patients
7. View FHIR `$everything` bundle for consented patient

### LAB Workflow
1. Login → receive JWT
2. Access authorized patient information
3. Create laboratory reports (`POST /api/v1/lab-reports`)
4. View existing lab reports

### FACILITY_ADMIN / SYSTEM_ADMIN
1. Login → receive JWT
2. Create facilities (`POST /api/v1/facilities`)
3. View audit logs (`GET /api/v1/audit`)
4. Administrative operations per RBAC policy

---

## Testing

```bash
# Run all backend unit tests (requires Rust installed locally, or run inside Docker)
cd backend
cargo test

# Check code formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings

# Run inside Docker (no local Rust needed)
docker compose exec backend sh -c "cargo test"
```

See the Phase 6 verification test suite: integration tests are in
`scripts/` and were executed against the running Docker stack during Phase 6.

---

## Troubleshooting

### Docker Desktop is not running
Start Docker Desktop before running `docker compose` commands.

### Containers fail to start — port already in use
Check for processes using port 80, 5432, 8080, 9000, or 9001:
```powershell
# Windows
netstat -ano | findstr :80
# Linux/Mac
lsof -i :80
```
Edit `.env` to change `NGINX_HTTP_PORT` to an available port.

### Backend container unhealthy
The backend requires PostgreSQL to be healthy before starting.
Check PostgreSQL status:
```bash
docker compose logs postgres
docker compose ps
```

### Backend cannot reach PostgreSQL
Verify the `DATABASE_URL` in your `.env` uses the Docker service name (`postgres`), not `localhost`:
```
DATABASE_URL=postgres://dhp_user:dhp_password@postgres:5432/digital_health_platform
```

### Frontend cannot reach backend
Check the Nginx proxy configuration and ensure the backend container is healthy:
```bash
docker compose logs nginx
docker compose logs backend
```

### Migration failures
Check backend logs for SQL error details:
```bash
docker compose logs backend
```
If the schema is corrupted, reset the database:
```bash
docker compose down -v
docker compose up --build
```

### Stale Docker volumes (data from previous run causing issues)
```bash
# WARNING: Destroys all data permanently
docker compose down -v --remove-orphans
docker compose up --build
```

### Environment variable not taking effect
Ensure you have copied `.env.example` to `.env` and edited it.
Restart the affected service after changes:
```bash
docker compose restart backend
```

---

## Prototype Limitations

This prototype is **not** suitable for production use without:

1. Professional security audit and penetration testing
2. Clinical validation by licensed medical professionals
3. Regulatory approval (DRAP, relevant Pakistani health authorities)
4. HTTPS/TLS certificate configuration
5. httpOnly secure cookie session management (currently uses Bearer tokens)
6. Multi-factor authentication for clinical users
7. Encryption at rest for database and object storage
8. Disaster recovery and tested backup procedures
9. Load testing and performance validation
10. Incident response plan and runbook

See [SECURITY.md](SECURITY.md) for the full security model and limitations.

---

## Development Phases

| Phase | Description                                              | Status       |
|-------|----------------------------------------------------------|--------------|
| 0     | Architecture & Planning                                  | ✅ Complete  |
| 1     | Foundation (Backend + DB + Docker + Frontend shell)      | ✅ Complete  |
| 2     | Database Schema + Authentication + Digital Health ID     | ✅ Complete  |
| 3     | Consent, RBAC, Authorization & Audit Logging            | ✅ Complete  |
| 4     | Clinical Records & Longitudinal Medical Workflow         | ✅ Complete  |
| 5     | FHIR R4 Interoperability & Exchange API                  | ✅ Complete  |
| 6     | Integration Testing, Security Verification & Bug Fixes  | ✅ Complete  |
| 7     | Final Documentation, Cleanup & Project Handoff           | ✅ Complete  |

**Final Status: Local Docker Prototype — Complete**

---

## License

This is a prototype developed for educational and demonstration purposes only.
Not for clinical use. Not for production deployment without independent review.
