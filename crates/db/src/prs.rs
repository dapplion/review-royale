//! Pull request queries

use common::models::{PrState, PullRequest};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Create or update a pull request
pub async fn upsert(
    pool: &PgPool,
    repo_id: Uuid,
    github_id: i64,
    number: i32,
    title: &str,
    author_id: Uuid,
    state: PrState,
    created_at: DateTime<Utc>,
) -> Result<PullRequest, sqlx::Error> {
    let state_str = match state {
        PrState::Open => "open",
        PrState::Merged => "merged",
        PrState::Closed => "closed",
    };
    
    sqlx::query_as!(
        PullRequest,
        r#"
        INSERT INTO pull_requests (id, repo_id, github_id, number, title, author_id, state, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (github_id) DO UPDATE
        SET title = EXCLUDED.title,
            state = EXCLUDED.state
        RETURNING 
            id, repo_id, github_id, number, title, author_id,
            state as "state: PrState",
            created_at, first_review_at, merged_at, closed_at
        "#,
        Uuid::new_v4(),
        repo_id,
        github_id,
        number,
        title,
        author_id,
        state_str,
        created_at,
    )
    .fetch_one(pool)
    .await
}

/// Record first review time
pub async fn set_first_review(
    pool: &PgPool,
    pr_id: Uuid,
    first_review_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE pull_requests
        SET first_review_at = $2
        WHERE id = $1 AND first_review_at IS NULL
        "#,
        pr_id,
        first_review_at,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get PR by repo and number
pub async fn get_by_number(
    pool: &PgPool,
    repo_id: Uuid,
    number: i32,
) -> Result<Option<PullRequest>, sqlx::Error> {
    sqlx::query_as!(
        PullRequest,
        r#"
        SELECT 
            id, repo_id, github_id, number, title, author_id,
            state as "state: PrState",
            created_at, first_review_at, merged_at, closed_at
        FROM pull_requests
        WHERE repo_id = $1 AND number = $2
        "#,
        repo_id,
        number,
    )
    .fetch_optional(pool)
    .await
}

/// List recent PRs for a repo
pub async fn list_recent(
    pool: &PgPool,
    repo_id: Uuid,
    limit: i32,
) -> Result<Vec<PullRequest>, sqlx::Error> {
    sqlx::query_as!(
        PullRequest,
        r#"
        SELECT 
            id, repo_id, github_id, number, title, author_id,
            state as "state: PrState",
            created_at, first_review_at, merged_at, closed_at
        FROM pull_requests
        WHERE repo_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
        repo_id,
        limit as i64,
    )
    .fetch_all(pool)
    .await
}
