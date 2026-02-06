//! Database layer for Review Royale

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

pub mod repos;
pub mod users;
pub mod prs;
pub mod reviews;
pub mod achievements;
pub mod leaderboard;

/// Create a database connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    info!("Database connected");
    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    info!("Running migrations...");
    sqlx::migrate!("../../migrations").run(pool).await?;
    info!("Migrations complete");
    Ok(())
}
