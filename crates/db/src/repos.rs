//! Repository queries

use common::models::Repository;
use sqlx::PgPool;
use uuid::Uuid;

/// Get or create a repository
pub async fn upsert(
    pool: &PgPool,
    github_id: i64,
    owner: &str,
    name: &str,
) -> Result<Repository, sqlx::Error> {
    sqlx::query_as!(
        Repository,
        r#"
        INSERT INTO repositories (id, github_id, owner, name, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (github_id) DO UPDATE
        SET owner = EXCLUDED.owner, name = EXCLUDED.name
        RETURNING id, github_id, owner, name, created_at
        "#,
        Uuid::new_v4(),
        github_id,
        owner,
        name,
    )
    .fetch_one(pool)
    .await
}

/// Get repository by owner/name
pub async fn get_by_name(
    pool: &PgPool,
    owner: &str,
    name: &str,
) -> Result<Option<Repository>, sqlx::Error> {
    sqlx::query_as!(
        Repository,
        "SELECT id, github_id, owner, name, created_at FROM repositories WHERE owner = $1 AND name = $2",
        owner,
        name,
    )
    .fetch_optional(pool)
    .await
}

/// List all tracked repositories
pub async fn list(pool: &PgPool) -> Result<Vec<Repository>, sqlx::Error> {
    sqlx::query_as!(
        Repository,
        "SELECT id, github_id, owner, name, created_at FROM repositories ORDER BY owner, name"
    )
    .fetch_all(pool)
    .await
}
