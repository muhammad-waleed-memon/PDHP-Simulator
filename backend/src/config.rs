//! Application configuration loaded from environment variables.
//!
//! All configuration is validated at startup. The application will fail
//! fast with a clear error if required variables are missing.

use std::env;

/// Top-level application configuration.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Config {
    /// Database connection URL (PostgreSQL)
    pub database_url: String,
    /// Maximum connections in the database pool
    pub database_max_connections: u32,
    /// Minimum idle connections in the pool
    pub database_min_connections: u32,

    /// Application environment (development | production)
    pub app_env: AppEnv,
    /// Host address to bind the HTTP server
    pub server_host: String,
    /// Port to bind the HTTP server
    pub server_port: u16,

    /// JWT signing secret — must be at least 32 bytes
    pub jwt_secret: String,
    /// Access token validity in seconds
    pub jwt_expires_in_seconds: u64,
    /// Refresh token validity in seconds
    pub jwt_refresh_expires_in_seconds: u64,

    /// Comma-separated list of CORS allowed origins
    pub cors_allowed_origins: Vec<String>,

    /// Whether to seed demo data on startup
    pub seed_demo_data: bool,

    /// Configurable demo passwords for development/demo seeding
    pub demo_patient_password: String,
    pub demo_doctor_password: String,
    pub demo_lab_password: String,
    pub demo_facility_admin_password: String,
    pub demo_system_admin_password: String,
}

/// Application environment variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Production,
    Test,
}

#[allow(dead_code)]
impl AppEnv {
    pub fn is_development(&self) -> bool {
        matches!(self, AppEnv::Development)
    }

    pub fn is_production(&self) -> bool {
        matches!(self, AppEnv::Production)
    }
}

impl std::fmt::Display for AppEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEnv::Development => write!(f, "development"),
            AppEnv::Production => write!(f, "production"),
            AppEnv::Test => write!(f, "test"),
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Panics with a descriptive message if any required variable is missing
    /// or invalid, so the application fails fast at startup rather than at
    /// runtime.
    pub fn from_env() -> Self {
        // Load .env file if it exists (development convenience)
        // In Docker, variables are injected directly — dotenvy handles both cases.
        if let Err(e) = dotenvy::dotenv() {
            // Not an error in production where env vars are injected by the runtime
            tracing::debug!("dotenvy: {e} (this is normal in Docker)");
        }

        let database_url = require_env("DATABASE_URL");
        let jwt_secret = require_env("JWT_SECRET");

        // Validate JWT secret strength
        if jwt_secret.len() < 32 {
            panic!(
                "JWT_SECRET must be at least 32 characters long. \
                 Current length: {}. Set a strong secret in your .env file.",
                jwt_secret.len()
            );
        }

        let app_env = match env::var("APP_ENV")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "production" | "prod" => AppEnv::Production,
            "test" => AppEnv::Test,
            _ => AppEnv::Development,
        };

        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Config {
            database_url,
            database_max_connections: parse_env("DATABASE_MAX_CONNECTIONS", 20),
            database_min_connections: parse_env("DATABASE_MIN_CONNECTIONS", 2),

            app_env,
            server_host: env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: parse_env("APP_PORT", 8080),

            jwt_secret,
            jwt_expires_in_seconds: parse_env("JWT_EXPIRES_IN_SECONDS", 3600),
            jwt_refresh_expires_in_seconds: parse_env("JWT_REFRESH_EXPIRES_IN_SECONDS", 604800),

            cors_allowed_origins,

            seed_demo_data: env::var("SEED_DEMO_DATA")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),

            demo_patient_password: env::var("DEMO_PATIENT_PASSWORD")
                .unwrap_or_else(|_| "PatientDemo123!".to_string()),
            demo_doctor_password: env::var("DEMO_DOCTOR_PASSWORD")
                .unwrap_or_else(|_| "DoctorDemo123!".to_string()),
            demo_lab_password: env::var("DEMO_LAB_PASSWORD")
                .unwrap_or_else(|_| "LabUserDemo123!".to_string()),
            demo_facility_admin_password: env::var("DEMO_FACILITY_ADMIN_PASSWORD")
                .unwrap_or_else(|_| "FacilityAdminDemo123!".to_string()),
            demo_system_admin_password: env::var("DEMO_SYSTEM_ADMIN_PASSWORD")
                .unwrap_or_else(|_| "SysAdminDemo123!".to_string()),
        }
    }

    /// Bind address string for the HTTP server.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}

/// Require an environment variable, panicking with a clear message if missing.
fn require_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| {
        panic!(
            "Required environment variable '{name}' is not set. \
             Copy .env.example to .env and fill in the required values."
        )
    })
}

/// Parse an environment variable as a type that implements `FromStr + Default`,
/// using the provided default value if the variable is not set.
fn parse_env<T>(name: &str, default: T) -> T
where
    T: std::str::FromStr + Copy + std::fmt::Debug,
    T::Err: std::fmt::Debug,
{
    match env::var(name) {
        Ok(val) => val.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to parse env var '{}' value '{}': {:?}. Using default: {:?}",
                name,
                val,
                e,
                default
            );
            default
        }),
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_env_display() {
        assert_eq!(AppEnv::Development.to_string(), "development");
        assert_eq!(AppEnv::Production.to_string(), "production");
        assert_eq!(AppEnv::Test.to_string(), "test");
    }

    #[test]
    fn app_env_predicates() {
        assert!(AppEnv::Development.is_development());
        assert!(!AppEnv::Development.is_production());
        assert!(AppEnv::Production.is_production());
        assert!(!AppEnv::Production.is_development());
    }
}
