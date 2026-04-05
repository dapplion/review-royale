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

/// Count fast reviews by a user (submitted within 1 hour of PR creation)
pub async fn count_fast_reviews(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM reviews r
        JOIN pull_requests pr ON r.pr_id = pr.id
        WHERE r.reviewer_id = $1
          AND r.submitted_at >= pr.created_at
          AND r.submitted_at < pr.created_at + INTERVAL '1 hour'
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}

/// Count PRs where the user was the first reviewer.
pub async fn count_first_responder_reviews(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        WITH first_reviews AS (
            SELECT DISTINCT ON (r.pr_id) r.pr_id, r.reviewer_id
            FROM reviews r
            ORDER BY r.pr_id, r.submitted_at ASC, r.github_id ASC
        )
        SELECT COUNT(*) as count
        FROM first_reviews
        WHERE reviewer_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}

/// Get the user's maximum consecutive-day review streak.
pub async fn max_review_streak(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        WITH review_dates AS (
            SELECT DISTINCT DATE(submitted_at) as review_date
            FROM reviews
            WHERE reviewer_id = $1
        ),
        streaks AS (
            SELECT review_date,
                   review_date - (ROW_NUMBER() OVER (ORDER BY review_date))::int AS streak_group
            FROM review_dates
        ),
        streak_lengths AS (
            SELECT streak_group, COUNT(*) as streak_len
            FROM streaks
            GROUP BY streak_group
        )
        SELECT COALESCE(MAX(streak_len), 0) as max_streak
        FROM streak_lengths
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("max_streak"))
}

/// Check if user has a 7-day review streak (at least one review on 7 consecutive days)
pub async fn has_7_day_streak(pool: &PgPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
    Ok(max_review_streak(pool, user_id).await? >= 7)
}

/// Count the number of 30+ day gaps followed by a new review day.
pub async fn count_comebacks(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        WITH review_dates AS (
            SELECT DISTINCT DATE(submitted_at) as review_date
            FROM reviews
            WHERE reviewer_id = $1
        ),
        review_gaps AS (
            SELECT
                review_date,
                LAG(review_date) OVER (ORDER BY review_date) as previous_review_date
            FROM review_dates
        )
        SELECT COUNT(*) FILTER (
            WHERE previous_review_date IS NOT NULL
              AND review_date - previous_review_date >= 30
        ) as count
        FROM review_gaps
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}

/// Get the maximum number of reviews the user submitted in a single day.
pub async fn max_reviews_in_single_day(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COALESCE(MAX(day_count), 0) as max_reviews
        FROM (
            SELECT DATE(submitted_at) as review_date, COUNT(*) as day_count
            FROM reviews
            WHERE reviewer_id = $1
            GROUP BY DATE(submitted_at)
        ) daily_counts
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("max_reviews"))
}

/// Count merged PRs where the user's approval was the last review before merge.
pub async fn count_closing_approvals(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        WITH last_reviews_before_merge AS (
            SELECT DISTINCT ON (r.pr_id) r.pr_id, r.reviewer_id, r.state
            FROM reviews r
            JOIN pull_requests pr ON pr.id = r.pr_id
            WHERE pr.state = 'merged'
              AND (pr.merged_at IS NULL OR r.submitted_at <= pr.merged_at)
            ORDER BY r.pr_id, r.submitted_at DESC, r.github_id DESC
        )
        SELECT COUNT(*) as count
        FROM last_reviews_before_merge
        WHERE reviewer_id = $1
          AND state = 'approved'
        "#,
    )
    .bind(user_id)
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
