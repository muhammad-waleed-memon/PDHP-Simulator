//! Deterministic demo data seeder.
//! Populates fictional DEMO-ONLY accounts and facility records on startup.

use crate::auth::crypto::hash_password;
use crate::auth::dhid::generate_digital_health_id;
use crate::config::Config;
use crate::users::UserRole;
use sqlx::{FromRow, PgPool};
use tracing::info;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(FromRow)]
struct IdRow {
    id: Uuid,
}

/// Populate initial demo data if configured and accounts do not already exist.
pub async fn seed_demo_data_if_needed(
    pool: &PgPool,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config.seed_demo_data {
        info!("Demo data seeding is disabled (SEED_DEMO_DATA=false)");
        return Ok(());
    }

    info!("Initializing deterministic demo data seeding...");

    // 1. Seed Default Facility
    let facility_id = seed_facility(pool).await?;

    // 2. Seed Accounts with configured passwords
    seed_account(
        pool,
        "patient@demo.local",
        &config.demo_patient_password,
        "DEMO Patient (Fictional)",
        UserRole::Patient,
    )
    .await?;

    let doctor_user_id = seed_account(
        pool,
        "doctor@demo.local",
        &config.demo_doctor_password,
        "DEMO Dr. Tariq Mahmood (Fictional)",
        UserRole::Doctor,
    )
    .await?;

    seed_doctor_profile(pool, doctor_user_id, facility_id).await?;

    seed_account(
        pool,
        "lab@demo.local",
        &config.demo_lab_password,
        "DEMO Lab Technician (Fictional)",
        UserRole::Lab,
    )
    .await?;

    seed_account(
        pool,
        "facilityadmin@demo.local",
        &config.demo_facility_admin_password,
        "DEMO Facility Admin (Fictional)",
        UserRole::FacilityAdmin,
    )
    .await?;

    seed_account(
        pool,
        "sysadmin@demo.local",
        &config.demo_system_admin_password,
        "DEMO System Admin (Fictional)",
        UserRole::SystemAdmin,
    )
    .await?;

    // 3. Seed Demo Patient Profile with DHID
    seed_demo_patient_profile(pool).await?;

    // 4. Seed Common Medications Catalog
    seed_medications(pool).await?;

    info!("Demo data seeding completed successfully.");
    Ok(())
}

async fn seed_medications(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM medications")
        .fetch_one(pool)
        .await?;

    if count == 0 {
        let sample_meds = vec![
            (
                "Amoxicillin 500mg",
                "Amoxicillin",
                "500mg",
                "Capsule",
                "Oral",
            ),
            (
                "Paracetamol 500mg",
                "Acetaminophen",
                "500mg",
                "Tablet",
                "Oral",
            ),
            (
                "Metformin 500mg",
                "Metformin HCl",
                "500mg",
                "Tablet",
                "Oral",
            ),
            ("Omeprazole 20mg", "Omeprazole", "20mg", "Capsule", "Oral"),
            (
                "Ciprofloxacin 500mg",
                "Ciprofloxacin",
                "500mg",
                "Tablet",
                "Oral",
            ),
            (
                "Aspirin 75mg",
                "Acetylsalicylic acid",
                "75mg",
                "Tablet",
                "Oral",
            ),
            (
                "Penicillin V 250mg",
                "Penicillin V Potassium",
                "250mg",
                "Tablet",
                "Oral",
            ),
        ];

        for (name, generic, strength, form, route) in sample_meds {
            sqlx::query(
                r#"
                INSERT INTO medications (name, generic_name, strength, dosage_form, route)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(name)
            .bind(generic)
            .bind(strength)
            .bind(form)
            .bind(route)
            .execute(pool)
            .await?;
        }

        info!("Seeded sample medication catalog");
    }

    Ok(())
}

async fn seed_facility(pool: &PgPool) -> Result<Uuid, Box<dyn std::error::Error>> {
    let name = "PIMS Islamabad Main Hospital (DEMO)";
    let existing = sqlx::query_as::<_, IdRow>("SELECT id FROM facilities WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = existing {
        return Ok(row.id);
    }

    let facility = sqlx::query_as::<_, IdRow>(
        r#"
        INSERT INTO facilities (name, facility_type, license_number, address, city, province, contact_number, status)
        VALUES ($1, 'Tertiary Care Hospital', 'DEMO-FAC-001', 'G-8/3 Sector', 'Islamabad', 'ICT', '+92-51-9261170', 'ACTIVE')
        RETURNING id
        "#
    )
    .bind(name)
    .fetch_one(pool)
    .await?;

    info!("Seeded DEMO facility: {name}");
    Ok(facility.id)
}

async fn seed_account(
    pool: &PgPool,
    email: &str,
    password: &str,
    full_name: &str,
    role: UserRole,
) -> Result<Uuid, Box<dyn std::error::Error>> {
    let existing = sqlx::query_as::<_, IdRow>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = existing {
        return Ok(row.id);
    }

    let hash = hash_password(password)?;

    let user = sqlx::query_as::<_, IdRow>(
        r#"
        INSERT INTO users (email, password_hash, full_name, role, status)
        VALUES ($1, $2, $3, $4::user_role, 'ACTIVE'::user_status)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(&hash)
    .bind(full_name)
    .bind(role)
    .fetch_one(pool)
    .await?;

    info!("Seeded DEMO user account: {email}");
    Ok(user.id)
}

async fn seed_doctor_profile(
    pool: &PgPool,
    user_id: Uuid,
    facility_id: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing = sqlx::query_as::<_, IdRow>("SELECT id FROM doctors WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    if existing.is_none() {
        sqlx::query(
            r#"
            INSERT INTO doctors (user_id, facility_id, pmdc_license_number, specialization, qualification, status)
            VALUES ($1, $2, 'DEMO-PMDC-12345', 'Internal Medicine', 'MBBS, FCPS', 'VERIFIED'::doctor_status)
            "#
        )
        .bind(user_id)
        .bind(facility_id)
        .execute(pool)
        .await?;

        info!("Seeded DEMO doctor profile for user {user_id}");
    }

    Ok(())
}

async fn seed_demo_patient_profile(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let patient_user =
        sqlx::query_as::<_, IdRow>("SELECT id FROM users WHERE email = 'patient@demo.local'")
            .fetch_optional(pool)
            .await?;

    if let Some(u) = patient_user {
        let existing_patient =
            sqlx::query_as::<_, IdRow>("SELECT id FROM patients WHERE user_id = $1")
                .bind(u.id)
                .fetch_optional(pool)
                .await?;

        if existing_patient.is_none() {
            let dhid = generate_digital_health_id();
            let dob = chrono::NaiveDate::from_ymd_opt(1990, 5, 14).unwrap();

            sqlx::query(
                r#"
                INSERT INTO patients (
                    digital_health_id, user_id, full_name, date_of_birth, gender, blood_group,
                    phone_number, address, city, province, emergency_contact_name, emergency_contact_phone, status
                )
                VALUES ($1, $2, 'DEMO Patient (Fictional)', $3, 'Male', 'O+', '+92-300-1234567', 'House 12, Street 4, F-7/2', 'Islamabad', 'ICT', 'Emergency Contact', '+92-300-9876543', 'ACTIVE')
                "#
            )
            .bind(&dhid)
            .bind(u.id)
            .bind(dob)
            .execute(pool)
            .await?;

            info!("Seeded DEMO patient record with Digital Health ID: {dhid}");
        }
    }

    Ok(())
}
