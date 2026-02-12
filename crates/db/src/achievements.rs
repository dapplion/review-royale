//! Achievement queries

use common::models::{Achievement, AchievementRarity, UserAchievement};
use serde::Serialize;
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

fn parse_rarity(s: &str) -> AchievementRarity {
    match s.to_lowercase().as_str() {
        "uncommon" => AchievementRarity::Uncommon,
        "rare" => AchievementRarity::Rare,
        "epic" => AchievementRarity::Epic,
        "legendary" => AchievementRarity::Legendary,
        _ => AchievementRarity::Common,
    }
}

/// List all achievement definitions
pub async fn list_all(pool: &PgPool) -> Result<Vec<Achievement>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, description, emoji, xp_reward, rarity
        FROM achievements
        ORDER BY 
            CASE rarity 
                WHEN 'common' THEN 1 
                WHEN 'uncommon' THEN 2 
                WHEN 'rare' THEN 3 
                WHEN 'epic' THEN 4 
                WHEN 'legendary' THEN 5 
            END,
            name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Achievement {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            emoji: r.get("emoji"),
            xp_reward: r.get("xp_reward"),
            rarity: parse_rarity(r.get::<&str, _>("rarity")),
        })
        .collect())
}

/// Achievement category for grouping in the UI
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementCategory {
    Milestone,
    Speed,
    Quality,
    Streak,
    Special,
}

/// Achievement with category and unlock count
#[derive(Debug, Clone, Serialize)]
pub struct AchievementWithStats {
    #[serde(flatten)]
    pub achievement: Achievement,
    pub category: AchievementCategory,
    pub unlock_count: i64,
}

/// List all achievements with unlock counts
pub async fn list_all_with_stats(pool: &PgPool) -> Result<Vec<AchievementWithStats>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT a.id, a.name, a.description, a.emoji, a.xp_reward, a.rarity,
               COALESCE(c.count, 0) as unlock_count
        FROM achievements a
        LEFT JOIN (
            SELECT achievement_id, COUNT(*) as count
            FROM user_achievements
            GROUP BY achievement_id
        ) c ON c.achievement_id = a.id
        ORDER BY 
            CASE a.rarity 
                WHEN 'common' THEN 1 
                WHEN 'uncommon' THEN 2 
                WHEN 'rare' THEN 3 
                WHEN 'epic' THEN 4 
                WHEN 'legendary' THEN 5 
            END,
            a.name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let id: String = r.get("id");
            let category = categorize_achievement(&id);
            AchievementWithStats {
                achievement: Achievement {
                    id,
                    name: r.get("name"),
                    description: r.get("description"),
                    emoji: r.get("emoji"),
                    xp_reward: r.get("xp_reward"),
                    rarity: parse_rarity(r.get::<&str, _>("rarity")),
                },
                category,
                unlock_count: r.get("unlock_count"),
            }
        })
        .collect())
}

fn categorize_achievement(id: &str) -> AchievementCategory {
    match id {
        // Milestones
        "first_review" | "review_10" | "review_50" | "review_100" | "review_500"
        | "review_1000" | "first_pr" | "pr_merged_10" | "pr_merged_100" => {
            AchievementCategory::Milestone
        }
        // Speed
        "speed_demon" | "first_responder" => AchievementCategory::Speed,
        // Quality
        "nitpicker" | "bug_hunter" | "thorough" => AchievementCategory::Quality,
        // Streaks
        "review_streak_7" | "review_streak_30" => AchievementCategory::Streak,
        // Special/Fun
        _ => AchievementCategory::Special,
    }
}

/// User's progress toward achievements
#[derive(Debug, Clone, Serialize)]
pub struct AchievementProgress {
    pub achievement_id: String,
    pub name: String,
    pub emoji: String,
    pub description: String,
    pub xp_reward: i32,
    pub rarity: AchievementRarity,
    pub category: AchievementCategory,
    /// Current progress value
    pub current: i64,
    /// Target value to unlock
    pub target: i64,
    /// Progress as percentage (0-100)
    pub progress_pct: f64,
    /// Whether the user has unlocked this
    pub unlocked: bool,
}

/// Get a user's progress toward all achievements
pub async fn get_user_progress(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<AchievementProgress>, sqlx::Error> {
    // Get all achievements
    let achievements = list_all(pool).await?;

    // Get user's unlocked achievements
    let unlocked: std::collections::HashSet<String> = list_for_user(pool, user_id)
        .await?
        .into_iter()
        .map(|a| a.achievement_id)
        .collect();

    // Get user stats for progress calculation
    let stats = sqlx::query(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM reviews r 
             JOIN pull_requests pr ON pr.id = r.pr_id 
             WHERE r.reviewer_id = $1) as total_reviews,
            (SELECT COUNT(*) FROM reviews r 
             JOIN pull_requests pr ON pr.id = r.pr_id 
             WHERE r.reviewer_id = $1 AND r.first_reviewer) as first_reviews,
            (SELECT COUNT(*) FROM reviews r 
             JOIN pull_requests pr ON pr.id = r.pr_id 
             WHERE r.reviewer_id = $1 AND r.comments_count >= 10) as deep_reviews,
            (SELECT COUNT(*) FROM pull_requests WHERE author_id = $1) as prs_authored,
            (SELECT COUNT(*) FROM pull_requests WHERE author_id = $1 AND state = 'merged') as prs_merged
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let total_reviews: i64 = stats.get("total_reviews");
    let first_reviews: i64 = stats.get("first_reviews");
    let deep_reviews: i64 = stats.get("deep_reviews");
    let prs_authored: i64 = stats.get("prs_authored");
    let prs_merged: i64 = stats.get("prs_merged");

    let mut progress_list: Vec<AchievementProgress> = achievements
        .into_iter()
        .map(|a| {
            let (current, target) = get_progress_values(
                &a.id,
                total_reviews,
                first_reviews,
                deep_reviews,
                prs_authored,
                prs_merged,
            );
            let is_unlocked = unlocked.contains(&a.id);
            let progress_pct = if target > 0 {
                ((current as f64 / target as f64) * 100.0).min(100.0)
            } else {
                0.0
            };

            AchievementProgress {
                achievement_id: a.id.clone(),
                name: a.name,
                emoji: a.emoji,
                description: a.description,
                xp_reward: a.xp_reward,
                rarity: a.rarity,
                category: categorize_achievement(&a.id),
                current,
                target,
                progress_pct,
                unlocked: is_unlocked,
            }
        })
        .collect();

    // Sort: unlocked last, then by progress (closest to unlock first)
    progress_list.sort_by(|a, b| match (a.unlocked, b.unlocked) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => b
            .progress_pct
            .partial_cmp(&a.progress_pct)
            .unwrap_or(std::cmp::Ordering::Equal),
    });

    Ok(progress_list)
}

/// Get current/target values for a specific achievement
fn get_progress_values(
    achievement_id: &str,
    total_reviews: i64,
    first_reviews: i64,
    deep_reviews: i64,
    prs_authored: i64,
    prs_merged: i64,
) -> (i64, i64) {
    match achievement_id {
        // Review milestones
        "first_review" => (total_reviews.min(1), 1),
        "review_10" => (total_reviews.min(10), 10),
        "review_50" => (total_reviews.min(50), 50),
        "review_100" => (total_reviews.min(100), 100),
        "review_500" => (total_reviews.min(500), 500),
        "review_1000" => (total_reviews.min(1000), 1000),
        // Speed achievements
        "first_responder" => (first_reviews.min(25), 25),
        "speed_demon" => (0, 10), // Would need to query fast reviews separately
        // Quality achievements
        "thorough" => (deep_reviews.min(5), 5),
        "bug_hunter" => (0, 10), // Would need AI categorization data
        "nitpicker" => (0, 50),  // Would need nit comment tracking
        // PR author achievements
        "first_pr" => (prs_authored.min(1), 1),
        "pr_merged_10" => (prs_merged.min(10), 10),
        "pr_merged_100" => (prs_merged.min(100), 100),
        // Streaks and special - can't easily calculate without more queries
        "review_streak_7" => (0, 7),
        "review_streak_30" => (0, 30),
        "comeback_kid" => (0, 1),
        "review_rampage" => (0, 5),
        "the_closer" => (0, 10),
        "helpful" => (0, 10),
        _ => (0, 1),
    }
}
