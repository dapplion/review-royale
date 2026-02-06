//! Achievement queries

use common::models::{Achievement, AchievementRarity, UserAchievement};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unlock an achievement for a user
pub async fn unlock(
    pool: &PgPool,
    user_id: Uuid,
    achievement_id: &str,
) -> Result<UserAchievement, sqlx::Error> {
    sqlx::query_as!(
        UserAchievement,
        r#"
        INSERT INTO user_achievements (user_id, achievement_id, unlocked_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (user_id, achievement_id) DO NOTHING
        RETURNING user_id, achievement_id, unlocked_at
        "#,
        user_id,
        achievement_id,
    )
    .fetch_one(pool)
    .await
}

/// Check if user has achievement
pub async fn has_achievement(
    pool: &PgPool,
    user_id: Uuid,
    achievement_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM user_achievements
            WHERE user_id = $1 AND achievement_id = $2
        ) as "exists!"
        "#,
        user_id,
        achievement_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(result.exists)
}

/// Get all achievements for a user
pub async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserAchievement>, sqlx::Error> {
    sqlx::query_as!(
        UserAchievement,
        r#"
        SELECT user_id, achievement_id, unlocked_at
        FROM user_achievements
        WHERE user_id = $1
        ORDER BY unlocked_at DESC
        "#,
        user_id,
    )
    .fetch_all(pool)
    .await
}

/// Get recent unlocks across all users
pub async fn list_recent_unlocks(
    pool: &PgPool,
    limit: i32,
) -> Result<Vec<UserAchievement>, sqlx::Error> {
    sqlx::query_as!(
        UserAchievement,
        r#"
        SELECT user_id, achievement_id, unlocked_at
        FROM user_achievements
        ORDER BY unlocked_at DESC
        LIMIT $1
        "#,
        limit as i64,
    )
    .fetch_all(pool)
    .await
}

/// Count how many users have a specific achievement
pub async fn count_unlocks(
    pool: &PgPool,
    achievement_id: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM user_achievements
        WHERE achievement_id = $1
        "#,
        achievement_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(result.count)
}
