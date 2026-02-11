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
        name: None,
        description: None,
        emoji: None,
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

/// Get all achievements for a user with full details
pub async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserAchievement>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT ua.user_id, ua.achievement_id, ua.unlocked_at,
               a.name, a.description, a.emoji
        FROM user_achievements ua
        JOIN achievements a ON a.id = ua.achievement_id
        WHERE ua.user_id = $1
        ORDER BY ua.unlocked_at DESC
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
            name: Some(r.get("name")),
            description: Some(r.get("description")),
            emoji: Some(r.get("emoji")),
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
        SELECT ua.user_id, ua.achievement_id, ua.unlocked_at,
               a.name, a.description, a.emoji
        FROM user_achievements ua
        JOIN achievements a ON a.id = ua.achievement_id
        ORDER BY ua.unlocked_at DESC
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
            name: Some(r.get("name")),
            description: Some(r.get("description")),
            emoji: Some(r.get("emoji")),
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

/// Extended achievement with user info for notifications
pub struct AchievementNotification {
    pub user_id: Uuid,
    pub user_login: String,
    pub achievement_id: String,
    pub achievement_name: String,
    pub achievement_emoji: String,
    pub achievement_description: String,
    pub unlocked_at: chrono::DateTime<chrono::Utc>,
}

/// Get pending achievement notifications (unlocked but not yet notified)
pub async fn get_pending_notifications(
    pool: &PgPool,
    limit: i32,
) -> Result<Vec<AchievementNotification>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT ua.user_id, u.login as user_login,
               ua.achievement_id, a.name as achievement_name,
               a.emoji as achievement_emoji, a.description as achievement_description,
               ua.unlocked_at
        FROM user_achievements ua
        JOIN users u ON u.id = ua.user_id
        JOIN achievements a ON a.id = ua.achievement_id
        WHERE ua.notified_at IS NULL
        ORDER BY ua.unlocked_at ASC
        LIMIT $1
        "#,
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| AchievementNotification {
            user_id: r.get("user_id"),
            user_login: r.get("user_login"),
            achievement_id: r.get("achievement_id"),
            achievement_name: r.get("achievement_name"),
            achievement_emoji: r.get("achievement_emoji"),
            achievement_description: r.get("achievement_description"),
            unlocked_at: r.get("unlocked_at"),
        })
        .collect())
}

/// Mark an achievement as notified
pub async fn mark_notified(
    pool: &PgPool,
    user_id: Uuid,
    achievement_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE user_achievements
        SET notified_at = NOW()
        WHERE user_id = $1 AND achievement_id = $2
        "#,
    )
    .bind(user_id)
    .bind(achievement_id)
    .execute(pool)
    .await?;

    Ok(())
}
