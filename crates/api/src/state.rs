//! Application state

use common::Config;
use processor::EventHandler;
use sqlx::PgPool;

/// Shared application state
pub struct AppState {
    pub config: Config,
    pub pool: PgPool,
    pub event_handler: EventHandler,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Self {
        let event_handler = EventHandler::new(pool.clone());
        Self {
            config,
            pool,
            event_handler,
        }
    }
}
