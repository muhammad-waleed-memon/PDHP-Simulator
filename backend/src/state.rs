//! Shared application state passed to all Axum handlers via `Extension`.
//!
//! `AppState` is cheaply cloneable (all inner types use `Arc`) and is the
//! single source of truth for shared resources.

use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;

/// Inner application state (Arc-wrapped for cheap cloning).
struct Inner {
    /// Application configuration
    config: Config,
    /// PostgreSQL connection pool
    db: PgPool,
}

/// Shared application state — cheap to clone, passed to every handler.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

impl AppState {
    /// Construct the application state from resolved dependencies.
    pub fn new(config: Config, db: PgPool) -> Self {
        AppState {
            inner: Arc::new(Inner { config, db }),
        }
    }

    /// Access the application configuration.
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Access the database connection pool.
    pub fn db(&self) -> &PgPool {
        &self.inner.db
    }
}
