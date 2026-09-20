# SECURITY
# Pakistan Digital Health Identity & Longitudinal Medical Record Platform
# ============================================================

> **Prototype Notice:** This document describes the security model of a
> software prototype. This system has not undergone professional penetration
> testing, regulatory review, or clinical security validation. It should not
> be used in production without a comprehensive independent security audit.

---

## 1. Threat Model Assumptions

### In Scope (Prototype)
- Unauthorized access to patient records by unauthenticated users
- Role escalation (patient accessing doctor-only functions)
- Horizontal privilege escalation (Patient A accessing Patient B's records)
- Credential theft via weak password storage
- Token theft / session hijacking
- Audit trail tampering
- Unauthorized access without patient consent

### Out of Scope (Prototype Limitations)
- Nation-state level attacks
- Advanced persistent threats
- Physical server security
- Network-level attacks (requires production infrastructure)
- SIM-swap / identity fraud against registration
- Zero-day exploits in dependencies

---

## 2. Authentication

### Password Storage
- Passwords are hashed using **Argon2id** (Password Hashing Competition winner)
- Parameters: memory=65536 KiB, iterations=3, parallelism=4
- Salt is randomly generated per-password
- Plaintext passwords are never stored, logged, or returned in API responses

### JWT Tokens
- Algorithm: **HS256** (symmetric; suitable for monolith prototype)
- Production recommendation: migrate to RS256 (asymmetric) for microservices
- Expiry: configurable via `JWT_EXPIRES_IN_SECONDS` (default: 1 hour)
- Claims include: `sub` (user ID), `role`, `exp`, `iat`, `jti` (token ID)
- Token invalidation: logout records `jti` in a blocklist (in-memory or database)

### Session Security
- Tokens transmitted via `Authorization: Bearer` header only
- No tokens stored in cookies in the current prototype
- Production recommendation: httpOnly secure cookies + CSRF protection

---

## 3. Authorization

### Role-Based Access Control (RBAC)
Enforced at the **backend middleware layer**. Frontend UI may hide elements, but the backend always independently validates.

| Role | Clinical Access | Admin Access |
|---|---|---|
| PATIENT | Own records only | None |
| DOCTOR | Consented patient records | None |
| LAB | Authorized lab records only | None |
| FACILITY_ADMIN | None (clinical) | Facility administration |
| SYSTEM_ADMIN | None (clinical) | System administration |

### Resource-Level Authorization
Patient data access requires **one of**:
1. The requesting user IS the patient
2. A valid, active, non-revoked, non-expired patient consent record exists
3. A valid break-glass access record exists (logged separately)

Backend rejects access with `403 Forbidden` if none of these conditions are met.

---

## 4. Consent Model

Patient consent is enforced at the backend, not the frontend.

### Consent States
- `ACTIVE`: Access permitted
- `EXPIRED`: Consent past `expires_at`; access denied
- `REVOKED`: Patient revoked consent; access denied immediately

### What Consent Covers
- Provider access to patient records
- Specific scope (FULL, MEDICATIONS_ONLY, LABS_ONLY, ENCOUNTERS_ONLY)
- Time-limited duration (optional `expires_at`)

### Consent Is Not a Bypass
Even with active consent, RBAC still applies. A lab technician cannot read encounters even with patient consent — that requires the DOCTOR role.

---

## 5. Break-Glass Emergency Access

Emergency access is explicitly triggered, not implicit.

### Requirements
- Doctor must explicitly POST to the break-glass endpoint
- A **mandatory reason** must be provided
- Access is **time-limited** (maximum 4 hours, configurable)
- All access during break-glass is tagged as `BREAK_GLASS` in audit logs
- Patient and system administrators receive a notification
- Break-glass events are reviewed in the audit system

### What It Is NOT
- An automatic override when consent is missing
- A permanent privilege elevation
- Available to non-DOCTOR roles

---

## 6. Audit Logging

### Append-Only Design
Audit records are never modified or deleted by the application.

### What Is Audited
- Successful and failed authentication
- Patient record access (view, create, modify)
- Prescription creation
- Lab report creation
- Consent grant, revocation
- Break-glass activation
- Administrative actions
- Authorization failures

### Integrity
Each audit record contains:
- `record_hash`: SHA-256 of the record content
- `previous_hash`: Hash of the previous record (chain)

This enables integrity verification of the audit chain. Any tampering breaks the chain.

### What Is NOT Logged
- Passwords or password hashes
- JWT secrets
- Complete medical record content (only metadata/references)

---

## 7. Data Protection

### Secrets Management
- All secrets via environment variables
- No secrets in source code
- No secrets in Docker images
- `.env` file is in `.gitignore`
- `.env.example` contains only safe development placeholder values

### Database Security
- Parameterized queries via SQLx (no raw string interpolation)
- Database user has minimum required permissions
- No `postgres` superuser in application path

### Transport Security
- Production: HTTPS/TLS termination at Nginx (certificate management required)
- Prototype: HTTP on localhost only (acceptable for local demonstration)
- Production recommendation: Let's Encrypt or organizational CA

---

## 8. Input Validation

- All API request bodies validated using Rust `validator` crate
- String lengths enforced at the type level
- Required fields enforced
- UUID format validated
- Date range validation where applicable
- Validation errors return structured `400 Bad Request` responses without exposing internals

---

## 9. Error Handling

API errors use a consistent format:

```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Authentication required"
  }
}
```

The following are **never** exposed in error responses:
- Stack traces
- Database error details
- SQL queries
- Internal file paths
- Secret values

---

## 10. CORS Configuration

CORS is configured to allow only explicitly listed origins (`CORS_ALLOWED_ORIGINS`).

Wildcard `*` CORS is not used in any non-development mode.

---

## 11. Rate Limiting

Authentication endpoints (`/api/v1/auth/login`, `/api/v1/auth/register`) are rate-limited at the Nginx layer:
- Current local prototype configuration: 60 requests per minute per IP (burst=30)
- Production recommendation: tighten to 10 requests per 60-second window
- Returns HTTP 429 Too Many Requests on violation
- Configurable via `AUTH_RATE_LIMIT_REQUESTS` and `AUTH_RATE_LIMIT_WINDOW_SECONDS`

---

## 12. FHIR Access Controls

FHIR endpoints (`/fhir/*`) enforce the same security pipeline as internal APIs:

1. **JWT Authentication**: All FHIR requests require a valid Bearer token
2. **RBAC**: Role check (PATIENT, DOCTOR, LAB can access; role-specific rules apply)
3. **Consent Enforcement**: Patient consent is checked before any FHIR resource is returned
4. **Audit Logging**: Every FHIR access generates an audit event (`FHIR_RESOURCE_VIEWED`, `FHIR_BUNDLE_ACCESSED`, `FHIR_ACCESS_DENIED`, etc.)
5. **OperationOutcome**: All FHIR errors return standards-compliant `OperationOutcome` JSON payloads

FHIR is **read-only** (write operations are intentionally deferred to enforce internal domain validation rules).

---

## 13. Prototype Limitations

The following security measures are **not yet implemented** and are required for production:

| Limitation | Required Before Production |
|---|---|
| HTTPS/TLS | Certificate management + Nginx TLS config |
| httpOnly Cookies | Replace bearer tokens for browser sessions |
| CSRF Protection | Required with cookie-based auth |
| Key Management | HSM or cloud KMS for JWT and encryption keys |
| Dependency Audit | `cargo audit` integrated into CI |
| Penetration Testing | Professional engagement required |
| DRAP/Health Data Regulations | Legal and regulatory review for Pakistan |
| Backup & Recovery | Tested backup procedures required |
| Incident Response | IR plan and runbook required |
| Multi-factor Authentication | Required for clinical users |
| Encryption at Rest | Database and object storage encryption |

---

## 13. Responsible Disclosure

If you discover a security vulnerability in this prototype:

1. Do not create a public GitHub issue.
2. Contact the project maintainers privately.
3. Provide a clear description of the vulnerability.
4. Allow reasonable time for a fix before public disclosure.

---

## 14. Clinical Safety Disclaimer

This system is a **software prototype for engineering validation**.

- Clinical alerts are decision-support demonstrations only
- No clinical safety validation has been performed
- This system must not be used for real patient care
- All clinical decisions must be made by qualified medical professionals
- The authors accept no liability for clinical decisions made using this software

---

## 15. Final Prototype Status

This security documentation describes the security model of the **Pakistan Digital Health Platform — Local Docker Prototype**.

Phases 0–7 are complete. The system has been verified locally but:
- Has NOT undergone professional penetration testing
- Has NOT been reviewed by a certified healthcare security auditor
- Is NOT certified under any healthcare data protection regulation (HIPAA, DRAP, etc.)
- Is NOT suitable for production deployment or real patient data without independent review
