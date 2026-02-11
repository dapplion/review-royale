//! Database layer for Review Royale

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

pub mod achievements;
pub mod commits;
pub mod leaderboard;
pub mod prs;
pub mod repos;
pub mod review_comments;
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

/// Run database schema setup
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Running schema setup...");

    let schema = include_str!("../../../migrations/schema.sql");
    sqlx::raw_sql(schema).execute(pool).await?;

    info!("Schema setup complete");
    Ok(())
}
