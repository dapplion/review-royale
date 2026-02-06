//! Review queries

#![allow(clippy::too_many_arguments)]

use chrono::{DateTime, Utc};
use common::models::{Review, ReviewState};
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn parse_review_state(s: &str) -> ReviewState {
    match s {
        "approved" => ReviewState::Approved,
        "changes_requested" => ReviewState::ChangesRequested,
        "commented" => ReviewState::Commented,
        "dismissed" => ReviewState::Dismissed,
        _ => ReviewState::Pending,
    }
}

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

    let id = Uuid::new_v4();
    let row = sqlx::query(
        r#"
        INSERT INTO reviews (id, pr_id, reviewer_id, github_id, state, body, comments_count, submitted_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (github_id) DO UPDATE
        SET state = EXCLUDED.state,
            body = EXCLUDED.body,
            comments_count = EXCLUDED.comments_count
        RETURNING id, pr_id, reviewer_id, github_id, state, body, comments_count, submitted_at
        "#,
    )
    .bind(id)
    .bind(pr_id)
    .bind(reviewer_id)
    .bind(github_id)
    .bind(state_str)
    .bind(body)
    .bind(comments_count)
    .bind(submitted_at)
    .fetch_one(pool)
    .await?;

    Ok(Review {
        id: row.get("id"),
        pr_id: row.get("pr_id"),
        reviewer_id: row.get("reviewer_id"),
        github_id: row.get("github_id"),
        state: parse_review_state(row.get("state")),
        body: row.get("body"),
        comments_count: row.get("comments_count"),
        submitted_at: row.get("submitted_at"),
    })
}

/// Get reviews for a PR
pub async fn list_for_pr(pool: &PgPool, pr_id: Uuid) -> Result<Vec<Review>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, pr_id, reviewer_id, github_id, state, body, comments_count, submitted_at
        FROM reviews
        WHERE pr_id = $1
        ORDER BY submitted_at ASC
        "#,
    )
    .bind(pr_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Review {
            id: r.get("id"),
            pr_id: r.get("pr_id"),
            reviewer_id: r.get("reviewer_id"),
            github_id: r.get("github_id"),
            state: parse_review_state(r.get("state")),
            body: r.get("body"),
            comments_count: r.get("comments_count"),
            submitted_at: r.get("submitted_at"),
        })
        .collect())
}

/// Count reviews by a user in a time period
pub async fn count_by_user(
    pool: &PgPool,
    user_id: Uuid,
    since: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM reviews
        WHERE reviewer_id = $1 AND submitted_at >= $2
        "#,
    )
    .bind(user_id)
    .bind(since)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}

/// List all reviews (for recalculation)
pub async fn list_all(pool: &PgPool) -> Result<Vec<Review>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, pr_id, reviewer_id, github_id, state, body, comments_count, submitted_at
        FROM reviews
        ORDER BY submitted_at ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Review {
            id: r.get("id"),
            pr_id: r.get("pr_id"),
            reviewer_id: r.get("reviewer_id"),
            github_id: r.get("github_id"),
            state: parse_review_state(r.get("state")),
            body: r.get("body"),
            comments_count: r.get("comments_count"),
            submitted_at: r.get("submitted_at"),
        })
        .collect())
}
