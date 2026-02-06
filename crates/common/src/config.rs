//! Application configuration

use std::env;

/// Main application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub github_token: Option<String>,
    pub discord_token: Option<String>,
    pub discord_guild_id: Option<String>,
    pub host: String,
    pub port: u16,
    /// Sync interval in hours (0 = disabled)
    pub sync_interval_hours: u32,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/review_royale".to_string()
            }),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            github_token: env::var("GITHUB_TOKEN").ok(),
            discord_token: env::var("DISCORD_TOKEN").ok(),
            discord_guild_id: env::var("DISCORD_GUILD_ID").ok(),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            sync_interval_hours: env::var("SYNC_INTERVAL_HOURS")
                .ok()
                .and_then(|h| h.parse().ok())
                .unwrap_or(6),
        }
    }
}
