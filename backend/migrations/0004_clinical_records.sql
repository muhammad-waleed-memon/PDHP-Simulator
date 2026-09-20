-- Migration 0004: Clinical Records, Medications, Encounters, Prescriptions, Lab Reports & Clinical Alerts
-- ============================================================

-- ------------------------------------------------------------
-- 1. MEDICAL CONDITIONS & VERSION HISTORY
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS conditions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    code VARCHAR(100),
    status VARCHAR(50) NOT NULL DEFAULT 'ACTIVE',
    onset_date DATE,
    resolved_date DATE,
    notes TEXT,
    version INT NOT NULL DEFAULT 1,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_conditions_patient_id ON conditions(patient_id);
CREATE INDEX IF NOT EXISTS idx_conditions_status ON conditions(status);

CREATE TRIGGER set_conditions_updated_at
    BEFORE UPDATE ON conditions
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

CREATE TABLE IF NOT EXISTS condition_history (
    history_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    condition_id UUID NOT NULL REFERENCES conditions(id) ON DELETE CASCADE,
    version INT NOT NULL,
    status VARCHAR(50) NOT NULL,
    notes TEXT,
    modified_by UUID REFERENCES users(id) ON DELETE SET NULL,
    modified_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_condition_history_cond_id ON condition_history(condition_id);

-- ------------------------------------------------------------
-- 2. ALLERGIES & VERSION HISTORY
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS allergies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    allergen VARCHAR(255) NOT NULL,
    reaction TEXT,
    severity VARCHAR(50) NOT NULL DEFAULT 'MODERATE',
    status VARCHAR(50) NOT NULL DEFAULT 'ACTIVE',
    recorded_date DATE NOT NULL DEFAULT CURRENT_DATE,
    recorded_by UUID REFERENCES users(id) ON DELETE SET NULL,
    notes TEXT,
    version INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_allergies_patient_id ON allergies(patient_id);
CREATE INDEX IF NOT EXISTS idx_allergies_status ON allergies(status);

CREATE TRIGGER set_allergies_updated_at
    BEFORE UPDATE ON allergies
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

CREATE TABLE IF NOT EXISTS allergy_history (
    history_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    allergy_id UUID NOT NULL REFERENCES allergies(id) ON DELETE CASCADE,
    version INT NOT NULL,
    status VARCHAR(50) NOT NULL,
    severity VARCHAR(50) NOT NULL,
    notes TEXT,
    modified_by UUID REFERENCES users(id) ON DELETE SET NULL,
    modified_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_allergy_history_allergy_id ON allergy_history(allergy_id);

-- ------------------------------------------------------------
-- 3. MEDICATIONS CATALOG
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS medications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    generic_name VARCHAR(255),
    strength VARCHAR(100),
    dosage_form VARCHAR(100),
    route VARCHAR(100),
    status VARCHAR(50) NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_medications_name ON medications(name);
CREATE INDEX IF NOT EXISTS idx_medications_generic_name ON medications(generic_name);

CREATE TRIGGER set_medications_updated_at
    BEFORE UPDATE ON medications
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

-- ------------------------------------------------------------
-- 4. PATIENT MEDICATION HISTORY & VERSION HISTORY
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS patient_medications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    medication_id UUID NOT NULL REFERENCES medications(id) ON DELETE RESTRICT,
    prescribing_doctor_id UUID REFERENCES doctors(id) ON DELETE SET NULL,
    dosage VARCHAR(100) NOT NULL,
    frequency VARCHAR(100) NOT NULL,
    route VARCHAR(100),
    start_date DATE NOT NULL DEFAULT CURRENT_DATE,
    end_date DATE,
    status VARCHAR(50) NOT NULL DEFAULT 'ACTIVE',
    instructions TEXT,
    reason TEXT,
    version INT NOT NULL DEFAULT 1,
    recorded_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_patient_meds_patient_id ON patient_medications(patient_id);
CREATE INDEX IF NOT EXISTS idx_patient_meds_status ON patient_medications(status);

CREATE TRIGGER set_patient_medications_updated_at
    BEFORE UPDATE ON patient_medications
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

CREATE TABLE IF NOT EXISTS patient_medication_history (
    history_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_medication_id UUID NOT NULL REFERENCES patient_medications(id) ON DELETE CASCADE,
    version INT NOT NULL,
    status VARCHAR(50) NOT NULL,
    dosage VARCHAR(100) NOT NULL,
    frequency VARCHAR(100) NOT NULL,
    modified_by UUID REFERENCES users(id) ON DELETE SET NULL,
    modified_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_patient_med_history_med_id ON patient_medication_history(patient_medication_id);

-- ------------------------------------------------------------
-- 5. ENCOUNTERS
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS encounters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    doctor_id UUID NOT NULL REFERENCES doctors(id) ON DELETE RESTRICT,
    facility_id UUID NOT NULL REFERENCES facilities(id) ON DELETE RESTRICT,
    encounter_type VARCHAR(50) NOT NULL DEFAULT 'OUTPATIENT',
    start_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    end_time TIMESTAMPTZ,
    reason TEXT NOT NULL,
    clinical_notes TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'COMPLETED',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_encounters_patient_id ON encounters(patient_id);
CREATE INDEX IF NOT EXISTS idx_encounters_doctor_id ON encounters(doctor_id);
CREATE INDEX IF NOT EXISTS idx_encounters_facility_id ON encounters(facility_id);

CREATE TRIGGER set_encounters_updated_at
    BEFORE UPDATE ON encounters
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

-- ------------------------------------------------------------
-- 6. PRESCRIPTIONS & PRESCRIPTION ITEMS
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS prescriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    doctor_id UUID NOT NULL REFERENCES doctors(id) ON DELETE RESTRICT,
    encounter_id UUID REFERENCES encounters(id) ON DELETE SET NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status VARCHAR(50) NOT NULL DEFAULT 'ACTIVE',
    clinical_notes TEXT,
    override_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_prescriptions_patient_id ON prescriptions(patient_id);
CREATE INDEX IF NOT EXISTS idx_prescriptions_doctor_id ON prescriptions(doctor_id);

CREATE TRIGGER set_prescriptions_updated_at
    BEFORE UPDATE ON prescriptions
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

CREATE TABLE IF NOT EXISTS prescription_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    prescription_id UUID NOT NULL REFERENCES prescriptions(id) ON DELETE CASCADE,
    medication_id UUID NOT NULL REFERENCES medications(id) ON DELETE RESTRICT,
    dosage VARCHAR(100) NOT NULL,
    frequency VARCHAR(100) NOT NULL,
    route VARCHAR(100),
    duration VARCHAR(100) NOT NULL,
    quantity INT NOT NULL DEFAULT 1,
    instructions TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_prescription_items_prescription_id ON prescription_items(prescription_id);

-- ------------------------------------------------------------
-- 7. LABORATORY REPORTS
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS lab_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    encounter_id UUID REFERENCES encounters(id) ON DELETE SET NULL,
    laboratory_id UUID REFERENCES facilities(id) ON DELETE SET NULL,
    test_name VARCHAR(255) NOT NULL,
    result TEXT NOT NULL,
    unit VARCHAR(50),
    reference_range VARCHAR(100),
    abnormal_flag BOOLEAN NOT NULL DEFAULT FALSE,
    report_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status VARCHAR(50) NOT NULL DEFAULT 'FINAL',
    entered_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_lab_reports_patient_id ON lab_reports(patient_id);
CREATE INDEX IF NOT EXISTS idx_lab_reports_encounter_id ON lab_reports(encounter_id);

CREATE TRIGGER set_lab_reports_updated_at
    BEFORE UPDATE ON lab_reports
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();
