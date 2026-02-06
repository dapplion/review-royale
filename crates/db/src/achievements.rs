//! Achievement queries

use common::models::UserAchievement;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Unlock an achievement for a user
pub async fn unlock(
    pool: &PgPool,
    user_id: Uuid,
    achievement_id: &str,
) -> Result<UserAchievement, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO user_achievements (user_id, achievement_id, unlocked_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (user_id, achievement_id) DO UPDATE SET unlocked_at = user_achievements.unlocked_at
        RETURNING user_id, achievement_id, unlocked_at
        "#,
    )
    .bind(user_id)
    .bind(achievement_id)
    .fetch_one(pool)
    .await?;

    Ok(UserAchievement {
        user_id: row.get("user_id"),
        achievement_id: row.get("achievement_id"),
        unlocked_at: row.get("unlocked_at"),
    })
}

/// Check if user has achievement
pub async fn has_achievement(
    pool: &PgPool,
    user_id: Uuid,
    achievement_id: &str,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM user_achievements
            WHERE user_id = $1 AND achievement_id = $2
        ) as exists
        "#,
    )
    .bind(user_id)
    .bind(achievement_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<bool, _>("exists"))
}

/// Get all achievements for a user
pub async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserAchievement>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT user_id, achievement_id, unlocked_at
        FROM user_achievements
        WHERE user_id = $1
        ORDER BY unlocked_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| UserAchievement {
            user_id: r.get("user_id"),
            achievement_id: r.get("achievement_id"),
            unlocked_at: r.get("unlocked_at"),
        })
        .collect())
}

/// Get recent unlocks across all users
pub async fn list_recent_unlocks(
    pool: &PgPool,
    limit: i32,
) -> Result<Vec<UserAchievement>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT user_id, achievement_id, unlocked_at
        FROM user_achievements
        ORDER BY unlocked_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| UserAchievement {
            user_id: r.get("user_id"),
            achievement_id: r.get("achievement_id"),
            unlocked_at: r.get("unlocked_at"),
        })
        .collect())
}

/// Count how many users have a specific achievement
pub async fn count_unlocks(pool: &PgPool, achievement_id: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM user_achievements
        WHERE achievement_id = $1
        "#,
    )
    .bind(achievement_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}
