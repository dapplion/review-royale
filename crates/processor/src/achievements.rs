//! Achievement checking and unlocking

use chrono::{Duration, Utc};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Achievement definitions
pub mod defs {
    // Reviewer achievements
    pub const FIRST_REVIEW: &str = "first_review";
    pub const REVIEW_STREAK_7: &str = "review_streak_7";
    pub const SPEED_DEMON: &str = "speed_demon";
    pub const REVIEW_10: &str = "review_10";
    pub const REVIEW_50: &str = "review_50";
    pub const REVIEW_100: &str = "review_100";

    // Author achievements
    pub const FIRST_PR: &str = "first_pr";
    pub const PR_MERGED_10: &str = "pr_merged_10";
}

/// Checks and awards achievements
pub struct AchievementChecker {
    pool: PgPool,
}

impl AchievementChecker {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Check achievements for a reviewer
    pub async fn check_reviewer(&self, user_id: &Uuid) -> Result<Vec<String>, common::Error> {
        let mut unlocked = Vec::new();

        // Check review count milestones
        let since = Utc::now() - Duration::days(365 * 10); // All time
        let count = db::reviews::count_by_user(&self.pool, *user_id, since)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        // First review
        if count == 1 && self.try_unlock(user_id, defs::FIRST_REVIEW).await? {
            unlocked.push(defs::FIRST_REVIEW.to_string());
        }

        // 10 reviews
        if count >= 10 && self.try_unlock(user_id, defs::REVIEW_10).await? {
            unlocked.push(defs::REVIEW_10.to_string());
        }

        // 50 reviews
        if count >= 50 && self.try_unlock(user_id, defs::REVIEW_50).await? {
            unlocked.push(defs::REVIEW_50.to_string());
        }

        // 100 reviews
        if count >= 100 && self.try_unlock(user_id, defs::REVIEW_100).await? {
            unlocked.push(defs::REVIEW_100.to_string());
        }

        // Speed demon: 10+ fast reviews (within 1 hour of commits)
        let fast_count = db::reviews::count_fast_reviews(&self.pool, *user_id)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        if fast_count >= 10 && self.try_unlock(user_id, defs::SPEED_DEMON).await? {
            unlocked.push(defs::SPEED_DEMON.to_string());
        }

        // 7-day review streak
        let has_streak = db::reviews::has_7_day_streak(&self.pool, *user_id)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        if has_streak && self.try_unlock(user_id, defs::REVIEW_STREAK_7).await? {
            unlocked.push(defs::REVIEW_STREAK_7.to_string());
        }

        Ok(unlocked)
    }

    /// Check achievements for a PR author
    pub async fn check_author(&self, user_id: &Uuid) -> Result<Vec<String>, common::Error> {
        let mut unlocked = Vec::new();

        // Count PRs authored by this user
        let pr_count = db::prs::count_by_author(&self.pool, *user_id)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        // First PR
        if pr_count == 1 && self.try_unlock(user_id, defs::FIRST_PR).await? {
            unlocked.push(defs::FIRST_PR.to_string());
        }

        // Count merged PRs
        let merged_count = db::prs::count_merged_by_author(&self.pool, *user_id)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        // 10 merged PRs
        if merged_count >= 10 && self.try_unlock(user_id, defs::PR_MERGED_10).await? {
            unlocked.push(defs::PR_MERGED_10.to_string());
        }

        Ok(unlocked)
    }

    /// Try to unlock an achievement, returns true if newly unlocked
    async fn try_unlock(
        &self,
        user_id: &Uuid,
        achievement_id: &str,
    ) -> Result<bool, common::Error> {
        // Check if already has it
        let has = db::achievements::has_achievement(&self.pool, *user_id, achievement_id)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        if has {
            return Ok(false);
        }

        // Unlock it
        db::achievements::unlock(&self.pool, *user_id, achievement_id)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        info!(
            "🏆 Achievement unlocked: {} for user {:?}",
            achievement_id, user_id
        );
        Ok(true)
    }
}
