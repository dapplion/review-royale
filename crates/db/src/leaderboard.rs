//! Leaderboard queries

use chrono::{DateTime, Utc};
use common::models::{LeaderboardEntry, User, UserStats};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Get the review leaderboard for a time period
pub async fn get_leaderboard(
    pool: &PgPool,
    repo_id: Option<Uuid>,
    since: DateTime<Utc>,
    limit: i32,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    // First, get first reviews per PR (the reviewer who submitted first)
    // Then count how many times each user was first
    // Use period-specific XP (sum of xp_earned) for sorting and display
    let rows = sqlx::query(
        r#"
        WITH first_reviews AS (
            SELECT DISTINCT ON (r.pr_id) 
                r.pr_id,
                r.reviewer_id
            FROM reviews r
            JOIN pull_requests pr ON pr.id = r.pr_id
            WHERE r.submitted_at >= $1
              AND ($2::uuid IS NULL OR pr.repo_id = $2)
            ORDER BY r.pr_id, r.submitted_at ASC
        ),
        user_stats AS (
            SELECT 
                u.id,
                COUNT(r.id)::int as reviews_given,
                COALESCE(SUM(r.comments_count), 0)::int as comments_written,
                COALESCE(SUM(r.xp_earned), 0)::bigint as period_xp,
                COALESCE((SELECT COUNT(*) FROM first_reviews fr WHERE fr.reviewer_id = u.id), 0)::int as first_reviews
            FROM users u
            LEFT JOIN reviews r ON r.reviewer_id = u.id AND r.submitted_at >= $1
            LEFT JOIN pull_requests pr ON pr.id = r.pr_id
            WHERE ($2::uuid IS NULL OR pr.repo_id = $2)
            GROUP BY u.id
            HAVING COUNT(r.id) > 0
        )
        SELECT 
            u.id, u.github_id, u.login, u.avatar_url, 
            us.period_xp as xp, u.level,
            u.created_at, u.updated_at,
            us.reviews_given,
            us.comments_written,
            us.first_reviews
        FROM users u
        JOIN user_stats us ON us.id = u.id
        ORDER BY us.period_xp DESC, us.reviews_given DESC
        LIMIT $3
        "#,
    )
    .bind(since)
    .bind(repo_id)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| {
            let period_xp: i64 = row.get("xp");
            let user = User {
                id: row.get("id"),
                github_id: row.get("github_id"),
                login: row.get("login"),
                avatar_url: row.get("avatar_url"),
                xp: period_xp, // Use period XP for display
                level: row.get("level"), // Level is always all-time
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            LeaderboardEntry {
                rank: (idx + 1) as i32,
                score: period_xp,
                user,
                stats: UserStats {
                    reviews_given: row.get("reviews_given"),
                    comments_written: row.get("comments_written"),
                    first_reviews: row.get("first_reviews"),
                    ..Default::default()
                },
            }
        })
        .collect();

    Ok(entries)
}

/// Get a user's rank on the leaderboard
pub async fn get_user_rank(
    pool: &PgPool,
    user_id: Uuid,
    repo_id: Option<Uuid>,
    since: DateTime<Utc>,
) -> Result<Option<i32>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        WITH ranked AS (
            SELECT 
                u.id,
                ROW_NUMBER() OVER (ORDER BY COUNT(r.id) DESC) as rank
            FROM users u
            LEFT JOIN reviews r ON r.reviewer_id = u.id AND r.submitted_at >= $2
            LEFT JOIN pull_requests pr ON pr.id = r.pr_id
            WHERE ($3::uuid IS NULL OR pr.repo_id = $3)
            GROUP BY u.id
            HAVING COUNT(r.id) > 0
        )
        SELECT rank::int FROM ranked WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(since)
    .bind(repo_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.get::<i32, _>("rank")))
}
