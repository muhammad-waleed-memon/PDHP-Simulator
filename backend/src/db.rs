//! Database connection pool initialization and management.
//!
//! Uses SQLx with PostgreSQL. The pool is configured from the application
//! `Config` and shared through `AppState`.

use sqlx::{postgres::PgPoolOptions, PgPool};

pub mod seed;

use crate::config::Config;

/// Initialize the PostgreSQL connection pool.
///
/// This function will retry-connect according to SQLx defaults. The pool
/// validates connectivity with a test query before returning.
///
/// # Errors
/// Returns an error if the database is unreachable or the connection URL
/// is invalid.
pub async fn create_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    tracing::info!(
        max_connections = config.database_max_connections,
        min_connections = config.database_min_connections,
        "Initializing database connection pool"
    );

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&config.database_url)
        .await?;

    // Verify connectivity with a simple query
    sqlx::query("SELECT 1").execute(&pool).await.map_err(|e| {
        tracing::error!(error = %e, "Database connectivity check failed");
        e
    })?;

    tracing::info!("Database connection pool established successfully");
    Ok(pool)
}

/// Run pending SQLx migrations from the `./migrations` directory.
///
/// Migrations are run at application startup so the schema is always
/// up-to-date when the application begins serving requests.
///
/// # Errors
/// Returns an error if any migration fails.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    tracing::info!("Running database migrations");
    sqlx::migrate!("./migrations").run(pool).await?;
    tracing::info!("Database migrations completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    // Database tests require a live PostgreSQL instance.
    // These run in CI/CD where the database service is available.
    // For unit tests, we only test non-database logic.
}
