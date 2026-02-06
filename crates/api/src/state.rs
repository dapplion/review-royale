//! Application state

use common::Config;
use sqlx::PgPool;

/// Shared application state
pub struct AppState {
    pub config: Config,
    pub pool: PgPool,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Self {
        Self { config, pool }
    }
}
