//! Repository queries

use chrono::{DateTime, Utc};
use common::models::Repository;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Get or create a repository
pub async fn upsert(
    pool: &PgPool,
    github_id: i64,
    owner: &str,
    name: &str,
) -> Result<Repository, sqlx::Error> {
    let id = Uuid::new_v4();
    let row = sqlx::query(
        r#"
        INSERT INTO repositories (id, github_id, owner, name, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (github_id) DO UPDATE
        SET owner = EXCLUDED.owner, name = EXCLUDED.name
        RETURNING id, github_id, owner, name, created_at
        "#,
    )
    .bind(id)
    .bind(github_id)
    .bind(owner)
    .bind(name)
    .fetch_one(pool)
    .await?;

    Ok(Repository {
        id: row.get("id"),
        github_id: row.get("github_id"),
        owner: row.get("owner"),
        name: row.get("name"),
        created_at: row.get("created_at"),
    })
}

/// Get repository by owner/name
pub async fn get_by_name(
    pool: &PgPool,
    owner: &str,
    name: &str,
) -> Result<Option<Repository>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, github_id, owner, name, created_at FROM repositories WHERE owner = $1 AND name = $2",
    )
    .bind(owner)
    .bind(name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Repository {
        id: r.get("id"),
        github_id: r.get("github_id"),
        owner: r.get("owner"),
        name: r.get("name"),
        created_at: r.get("created_at"),
    }))
}

/// List all tracked repositories
pub async fn list(pool: &PgPool) -> Result<Vec<Repository>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, github_id, owner, name, created_at FROM repositories ORDER BY owner, name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Repository {
            id: r.get("id"),
            github_id: r.get("github_id"),
            owner: r.get("owner"),
            name: r.get("name"),
            created_at: r.get("created_at"),
        })
        .collect())
}

/// Get last sync timestamp for a repository
pub async fn get_last_synced_at(
    pool: &PgPool,
    repo_id: Uuid,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let row = sqlx::query("SELECT last_synced_at FROM repositories WHERE id = $1")
        .bind(repo_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.and_then(|r| r.get("last_synced_at")))
}

/// Update last sync timestamp for a repository
pub async fn set_last_synced_at(
    pool: &PgPool,
    repo_id: Uuid,
    synced_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE repositories SET last_synced_at = $1 WHERE id = $2")
        .bind(synced_at)
        .bind(repo_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Reset last sync timestamp for a repository (for force backfill)
pub async fn reset_last_synced_at(pool: &PgPool, repo_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE repositories SET last_synced_at = NULL WHERE id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    Ok(())
}
