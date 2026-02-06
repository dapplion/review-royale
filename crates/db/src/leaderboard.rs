//! Leaderboard queries

use common::models::{LeaderboardEntry, User, UserStats};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Get the review leaderboard for a time period
pub async fn get_leaderboard(
    pool: &PgPool,
    repo_id: Option<Uuid>,
    since: DateTime<Utc>,
    limit: i32,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    // This is a simplified version - real implementation would join more tables
    let rows = sqlx::query!(
        r#"
        SELECT 
            u.id, u.github_id, u.login, u.avatar_url, u.xp, u.level,
            u.created_at, u.updated_at,
            COUNT(r.id)::int as "reviews_given!",
            COALESCE(SUM(r.comments_count), 0)::int as "comments_written!"
        FROM users u
        LEFT JOIN reviews r ON r.reviewer_id = u.id AND r.submitted_at >= $1
        LEFT JOIN pull_requests pr ON pr.id = r.pr_id
        WHERE ($2::uuid IS NULL OR pr.repo_id = $2)
        GROUP BY u.id
        HAVING COUNT(r.id) > 0
        ORDER BY COUNT(r.id) DESC, u.xp DESC
        LIMIT $3
        "#,
        since,
        repo_id,
        limit as i64,
    )
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| {
            let user = User {
                id: row.id,
                github_id: row.github_id,
                login: row.login,
                avatar_url: row.avatar_url,
                xp: row.xp,
                level: row.level,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };
            LeaderboardEntry {
                rank: (idx + 1) as i32,
                score: row.reviews_given as i64,
                user,
                stats: UserStats {
                    reviews_given: row.reviews_given,
                    comments_written: row.comments_written,
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
    let result = sqlx::query!(
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
        SELECT rank::int as "rank!" FROM ranked WHERE id = $1
        "#,
        user_id,
        since,
        repo_id,
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(result.map(|r| r.rank))
}
