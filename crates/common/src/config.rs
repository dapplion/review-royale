//! Application configuration

use std::env;

/// Main application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub github_app_id: Option<String>,
    pub github_private_key_path: Option<String>,
    pub github_webhook_secret: Option<String>,
    pub discord_token: Option<String>,
    pub discord_guild_id: Option<String>,
    pub host: String,
    pub port: u16,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/review_royale".to_string()),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            github_app_id: env::var("GITHUB_APP_ID").ok(),
            github_private_key_path: env::var("GITHUB_PRIVATE_KEY_PATH").ok(),
            github_webhook_secret: env::var("GITHUB_WEBHOOK_SECRET").ok(),
            discord_token: env::var("DISCORD_TOKEN").ok(),
            discord_guild_id: env::var("DISCORD_GUILD_ID").ok(),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
        }
    }
}
