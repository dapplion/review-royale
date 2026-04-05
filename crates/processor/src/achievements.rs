//! Achievement checking and unlocking

use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Checks and awards achievements
pub struct AchievementChecker {
    pool: PgPool,
}

impl AchievementChecker {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Check and unlock all eligible achievements for a user.
    pub async fn check_user(&self, user_id: &Uuid) -> Result<Vec<String>, common::Error> {
        let progress = db::achievements::get_user_progress(&self.pool, *user_id)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        let mut unlocked = Vec::new();
        for achievement in progress {
            if achievement.unlocked || achievement.target <= 0 || achievement.current < achievement.target {
                continue;
            }

            if self.try_unlock(user_id, &achievement.achievement_id).await? {
                unlocked.push(achievement.achievement_id);
            }
        }

        Ok(unlocked)
    }

    /// Check achievements for a reviewer
    pub async fn check_reviewer(&self, user_id: &Uuid) -> Result<Vec<String>, common::Error> {
        self.check_user(user_id).await
    }

    /// Check achievements for a PR author
    pub async fn check_author(&self, user_id: &Uuid) -> Result<Vec<String>, common::Error> {
        self.check_user(user_id).await
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
