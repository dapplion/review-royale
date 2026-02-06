//! Review queries

use common::models::{Review, ReviewState};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Insert a new review
pub async fn insert(
    pool: &PgPool,
    pr_id: Uuid,
    reviewer_id: Uuid,
    github_id: i64,
    state: ReviewState,
    body: Option<&str>,
    comments_count: i32,
    submitted_at: DateTime<Utc>,
) -> Result<Review, sqlx::Error> {
    let state_str = match state {
        ReviewState::Approved => "approved",
        ReviewState::ChangesRequested => "changes_requested",
        ReviewState::Commented => "commented",
        ReviewState::Dismissed => "dismissed",
        ReviewState::Pending => "pending",
    };
    
    sqlx::query_as!(
        Review,
        r#"
        INSERT INTO reviews (id, pr_id, reviewer_id, github_id, state, body, comments_count, submitted_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (github_id) DO UPDATE
        SET state = EXCLUDED.state,
            body = EXCLUDED.body,
            comments_count = EXCLUDED.comments_count
        RETURNING 
            id, pr_id, reviewer_id, github_id,
            state as "state: ReviewState",
            body, comments_count, submitted_at
        "#,
        Uuid::new_v4(),
        pr_id,
        reviewer_id,
        github_id,
        state_str,
        body,
        comments_count,
        submitted_at,
    )
    .fetch_one(pool)
    .await
}

/// Get reviews for a PR
pub async fn list_for_pr(pool: &PgPool, pr_id: Uuid) -> Result<Vec<Review>, sqlx::Error> {
    sqlx::query_as!(
        Review,
        r#"
        SELECT 
            id, pr_id, reviewer_id, github_id,
            state as "state: ReviewState",
            body, comments_count, submitted_at
        FROM reviews
        WHERE pr_id = $1
        ORDER BY submitted_at ASC
        "#,
        pr_id,
    )
    .fetch_all(pool)
    .await
}

/// Count reviews by a user in a time period
pub async fn count_by_user(
    pool: &PgPool,
    user_id: Uuid,
    since: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM reviews
        WHERE reviewer_id = $1 AND submitted_at >= $2
        "#,
        user_id,
        since,
    )
    .fetch_one(pool)
    .await?;
    Ok(result.count)
}
