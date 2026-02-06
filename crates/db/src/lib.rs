//! Database layer for Review Royale

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

pub mod achievements;
pub mod commits;
pub mod leaderboard;
pub mod prs;
pub mod repos;
pub mod reviews;
pub mod users;

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

/// Run database migrations from SQL files
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Running migrations...");

    // Read and execute migration files in order
    let migration_001 = include_str!("../../../migrations/001_initial.sql");
    sqlx::raw_sql(migration_001).execute(pool).await?;

    let migration_002 = include_str!("../../../migrations/002_sync_tracking.sql");
    sqlx::raw_sql(migration_002).execute(pool).await?;

    info!("Migrations complete");
    Ok(())
}
