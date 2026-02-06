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
    let rows = sqlx::query(
        r#"
        SELECT 
            u.id, u.github_id, u.login, u.avatar_url, u.xp, u.level,
            u.created_at, u.updated_at,
            COUNT(r.id)::int as reviews_given,
            COALESCE(SUM(r.comments_count), 0)::int as comments_written
        FROM users u
        LEFT JOIN reviews r ON r.reviewer_id = u.id AND r.submitted_at >= $1
        LEFT JOIN pull_requests pr ON pr.id = r.pr_id
        WHERE ($2::uuid IS NULL OR pr.repo_id = $2)
        GROUP BY u.id
        HAVING COUNT(r.id) > 0
        ORDER BY COUNT(r.id) DESC, u.xp DESC
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
            let user = User {
                id: row.get("id"),
                github_id: row.get("github_id"),
                login: row.get("login"),
                avatar_url: row.get("avatar_url"),
                xp: row.get("xp"),
                level: row.get("level"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            LeaderboardEntry {
                rank: (idx + 1) as i32,
                score: row.get::<i32, _>("reviews_given") as i64,
                user,
                stats: UserStats {
                    reviews_given: row.get("reviews_given"),
                    comments_written: row.get("comments_written"),
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
