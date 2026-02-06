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
    pub fn calculate_review_xp(&self, pr: &PullRequest, review_submitted: DateTime<Utc>) -> i64 {
        let mut xp: i64 = 10; // Base XP for any review

        // Bonus for being first reviewer
        if metrics::is_first_review(pr, review_submitted) {
            xp += 5;
        }

        // Bonus for fast review (under 4 hours)
        if metrics::is_fast_review(pr.created_at, review_submitted) {
            xp += 3;
        }

        xp
    }
}
