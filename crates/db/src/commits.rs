//! Commit queries

use chrono::{DateTime, Utc};
use common::models::Commit;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Insert a commit
pub async fn insert(
    pool: &PgPool,
    pr_id: Uuid,
    sha: &str,
    author_id: Option<Uuid>,
    committed_at: DateTime<Utc>,
    message: Option<&str>,
) -> Result<Commit, sqlx::Error> {
    let id = Uuid::new_v4();
    let row = sqlx::query(
        r#"
        INSERT INTO commits (id, pr_id, sha, author_id, committed_at, message)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (pr_id, sha) DO UPDATE
        SET author_id = EXCLUDED.author_id,
            committed_at = EXCLUDED.committed_at,
            message = EXCLUDED.message
        RETURNING id, pr_id, sha, author_id, committed_at, message, created_at
        "#,
    )
    .bind(id)
    .bind(pr_id)
    .bind(sha)
    .bind(author_id)
    .bind(committed_at)
    .bind(message)
    .fetch_one(pool)
    .await?;

    Ok(Commit {
        id: row.get("id"),
        pr_id: row.get("pr_id"),
        sha: row.get("sha"),
        author_id: row.get("author_id"),
        committed_at: row.get("committed_at"),
        message: row.get("message"),
        created_at: row.get("created_at"),
    })
}

/// Get commits for a PR, ordered by time
pub async fn list_for_pr(pool: &PgPool, pr_id: Uuid) -> Result<Vec<Commit>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, pr_id, sha, author_id, committed_at, message, created_at
        FROM commits
        WHERE pr_id = $1
        ORDER BY committed_at ASC
        "#,
    )
    .bind(pr_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Commit {
            id: r.get("id"),
            pr_id: r.get("pr_id"),
            sha: r.get("sha"),
            author_id: r.get("author_id"),
            committed_at: r.get("committed_at"),
            message: r.get("message"),
            created_at: r.get("created_at"),
        })
        .collect())
}

/// List all commits (for recalculation)
pub async fn list_all(pool: &PgPool) -> Result<Vec<Commit>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, pr_id, sha, author_id, committed_at, message, created_at
        FROM commits
        ORDER BY committed_at ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Commit {
            id: r.get("id"),
            pr_id: r.get("pr_id"),
            sha: r.get("sha"),
            author_id: r.get("author_id"),
            committed_at: r.get("committed_at"),
            message: r.get("message"),
            created_at: r.get("created_at"),
        })
        .collect())
}
