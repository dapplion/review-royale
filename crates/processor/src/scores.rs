//! XP and score calculation

use chrono::{DateTime, Utc};
use common::models::PullRequest;
use sqlx::PgPool;

use crate::metrics;

/// Calculates XP and scores
pub struct ScoreCalculator {
    _pool: PgPool,
}

impl ScoreCalculator {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }

    /// Calculate XP for a review
    /// Formula:
    /// - Base: 10 XP per review
    /// - First review on PR: +15 XP
    /// - Fast review (<1 hour): +10 XP
    /// - Per comment: +5 XP (capped at +50)
    pub fn calculate_review_xp(
        &self,
        pr: &PullRequest,
        review_submitted: DateTime<Utc>,
        comments_count: i32,
    ) -> i64 {
        let mut xp: i64 = 10; // Base XP for any review

        // Bonus for being first reviewer
        if metrics::is_first_review(pr, review_submitted) {
            xp += 15;
        }

        // Bonus for fast review (under 1 hour)
        if metrics::is_fast_review(pr.created_at, review_submitted) {
            xp += 10;
        }

        // Bonus for comments (capped at 50 XP = 10 comments)
        let comment_xp = (comments_count as i64 * 5).min(50);
        xp += comment_xp;

        xp
    }
}
