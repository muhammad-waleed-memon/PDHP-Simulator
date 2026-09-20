//! Pakistan Digital Health Identity & Longitudinal Medical Record Platform
//!
//! Entry point for the Axum HTTP server.
//!
//! Startup sequence:
//!   1. Initialize tracing/logging
//!   2. Load configuration from environment
//!   3. Connect to PostgreSQL
//!   4. Run pending database migrations
//!   5. Build application state
//!   6. Build router with middleware
//!   7. Bind TCP listener
//!   8. Serve requests (with graceful shutdown)

mod allergies;
mod audit;
mod auth;
mod authorization;
mod clinical_alerts;
mod conditions;
mod config;
mod consent;
mod db;
mod doctors;
mod encounters;
mod error;
mod facilities;
mod fhir;
mod health;
mod laboratories;
mod medications;
mod middleware;
mod patient_medications;
mod patients;
mod prescriptions;
mod router;
mod state;
mod timeline;
mod users;

use std::time::Duration;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() {
    // ── 1. Tracing / logging ───────────────────────────────────────────────
    init_tracing();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Pakistan Digital Health Platform starting"
    );

    // ── 2. Configuration ──────────────────────────────────────────────────
    let config = Config::from_env();
    tracing::info!(
        env = %config.app_env,
        host = %config.server_host,
        port = config.server_port,
        "Configuration loaded"
    );

    // ── 3. Database connection pool ───────────────────────────────────────
    let pool = db::create_pool(&config)
        .await
        .expect("Failed to create database connection pool");

    // ── 4. Database migrations ────────────────────────────────────────────
    db::run_migrations(&pool)
        .await
        .expect("Database migration failed");

    // ── 4b. Demo Data Seeding ─────────────────────────────────────────────
    if let Err(e) = db::seed::seed_demo_data_if_needed(&pool, &config).await {
        tracing::error!(error = %e, "Demo data seeding failed");
    }

    // ── 5. Application state ──────────────────────────────────────────────
    let state = AppState::new(config.clone(), pool);

    // ── 6. Router ─────────────────────────────────────────────────────────
    let app = router::build_router(state);

    // ── 7. TCP listener ───────────────────────────────────────────────────
    let bind_addr = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {bind_addr}: {e}"));

    tracing::info!(address = %bind_addr, "HTTP server listening");

    // ── 8. Serve with graceful shutdown ───────────────────────────────────
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");

    tracing::info!("Server shutdown complete");
}

/// Initialize structured tracing using the RUST_LOG environment variable.
fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default filter if RUST_LOG is not set
            "info,backend=debug,tower_http=debug,sqlx=warn"
                .parse()
                .unwrap()
        }))
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}

/// Graceful shutdown signal handler.
///
/// Waits for CTRL+C (SIGINT) or SIGTERM in production.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received — beginning graceful shutdown");
    // Allow in-flight requests up to 10 seconds to complete
    tokio::time::sleep(Duration::from_secs(1)).await;
}
