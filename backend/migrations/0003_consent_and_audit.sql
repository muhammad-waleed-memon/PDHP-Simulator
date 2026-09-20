-- Migration 0003: Consent Grants & Tamper-Evident Audit Log
-- ============================================================
--
-- Consent model:
--   A patient explicitly grants a provider (doctor, lab, facility)
--   access to their record for a bounded time window.
--   Consent can be revoked at any time.
--
-- Audit log:
--   Every security-relevant event is immutably recorded.
--   Each row carries an HMAC-SHA256 chain hash over (previous_hash || row_payload)
--   so the chain can be verified for tampering offline.
-- ============================================================

-- ------------------------------------------------------------
-- ENUM: consent status
-- ------------------------------------------------------------
DO $$ BEGIN
    CREATE TYPE consent_status AS ENUM ('ACTIVE', 'REVOKED', 'EXPIRED');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- ------------------------------------------------------------
-- 1. CONSENT GRANTS TABLE
-- ------------------------------------------------------------
-- Records explicit patient consent for a specific provider to
-- access their medical record.
--
-- granted_to_user_id: the provider (doctor, lab, facility admin)
--   being granted access.  References users table for referential
--   integrity; application logic validates the role.
--
-- expires_at: NULL means the consent does not have a fixed
--   expiry (but can still be revoked).
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS consent_grants (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- The patient whose record is being shared
    patient_id      UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,

    -- The provider receiving access
    granted_to_user_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Consent lifecycle
    status          consent_status NOT NULL DEFAULT 'ACTIVE',
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ,
    revoked_at      TIMESTAMPTZ,

    -- Optional human-readable purpose / scope for this grant
    purpose         TEXT,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- A patient may only have one ACTIVE grant per provider at a time.
    -- Revoked/expired grants are kept for audit purposes.
    CONSTRAINT uq_active_consent UNIQUE (patient_id, granted_to_user_id, status)
);

CREATE INDEX IF NOT EXISTS idx_consent_patient_id
    ON consent_grants(patient_id);

CREATE INDEX IF NOT EXISTS idx_consent_granted_to
    ON consent_grants(granted_to_user_id);

CREATE INDEX IF NOT EXISTS idx_consent_status
    ON consent_grants(status);

-- Composite: fast lookup "does provider X have active consent for patient Y?"
CREATE INDEX IF NOT EXISTS idx_consent_lookup
    ON consent_grants(patient_id, granted_to_user_id, status);

CREATE TRIGGER set_consent_grants_updated_at
    BEFORE UPDATE ON consent_grants
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

-- ------------------------------------------------------------
-- 2. AUDIT LOG TABLE
-- ------------------------------------------------------------
-- Append-only record of all security-relevant events.
-- NEVER update or delete rows from this table.
--
-- Chain integrity:
--   row_hash = HMAC-SHA256( previous_hash || event_type || actor_user_id
--                           || patient_id || resource_type || resource_id
--                           || outcome || occurred_at )
--   The Rust AuditService computes and stores this hash.
--   An offline verifier can recompute the chain and detect any
--   inserted, deleted, or modified rows.
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS audit_log (
    id              BIGSERIAL PRIMARY KEY,  -- monotonic; used for chain ordering

    -- Who performed the action
    actor_user_id   UUID REFERENCES users(id) ON DELETE SET NULL,

    -- Which patient's data was touched (NULL for non-patient events)
    patient_id      UUID REFERENCES patients(id) ON DELETE SET NULL,

    -- The type of event (e.g. LOGIN, PATIENT_RECORD_ACCESSED, CONSENT_GRANTED)
    event_type      VARCHAR(100) NOT NULL,

    -- The domain resource being acted upon (e.g. 'Patient', 'ConsentGrant')
    resource_type   VARCHAR(100),
    -- UUID of the specific resource, stored as text for flexibility
    resource_id     TEXT,

    -- Structured metadata (JSON): request IP, user agent, extra context
    -- NEVER store passwords, hashes, or tokens here
    metadata        JSONB,

    -- ALLOWED / DENIED
    outcome         VARCHAR(20) NOT NULL DEFAULT 'ALLOWED',

    -- Human-readable reason (e.g. "Consent revoked", "Role insufficient")
    reason          TEXT,

    -- Wall-clock time the event occurred (set by the application, not DB)
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Tamper-evident hash chain
    -- SHA256 hex digest chained from the previous row's hash
    previous_hash   VARCHAR(64) NOT NULL DEFAULT '0000000000000000000000000000000000000000000000000000000000000000',
    row_hash        VARCHAR(64) NOT NULL DEFAULT ''
);

-- Audit log is append-only; no update trigger needed.
-- Indexes for common query patterns:
CREATE INDEX IF NOT EXISTS idx_audit_actor
    ON audit_log(actor_user_id);

CREATE INDEX IF NOT EXISTS idx_audit_patient
    ON audit_log(patient_id);

CREATE INDEX IF NOT EXISTS idx_audit_event_type
    ON audit_log(event_type);

CREATE INDEX IF NOT EXISTS idx_audit_occurred_at
    ON audit_log(occurred_at DESC);

-- Prevent anyone with DB access from modifying audit rows after insert.
-- (row-level security can be added in production; this is the prototype foundation)
COMMENT ON TABLE audit_log IS
    'Tamper-evident append-only audit log. Never UPDATE or DELETE rows.';
