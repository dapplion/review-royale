//! Domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Calculate user level from XP
/// Formula: level = floor(sqrt(xp / 100)) + 1
pub fn calculate_level(xp: i64) -> i32 {
    if xp <= 0 {
        return 1;
    }
    ((xp as f64 / 100.0).sqrt().floor() as i32) + 1
}

/// Calculate XP required to reach a given level
/// Inverse of calculate_level: xp = (level - 1)² × 100
pub fn xp_for_level(level: i32) -> i64 {
    if level <= 1 {
        return 0;
    }
    ((level - 1) as i64).pow(2) * 100
}

/// Calculate progress toward next level (0.0 to 1.0)
pub fn level_progress(xp: i64) -> f64 {
    let current_level = calculate_level(xp);
    let current_level_xp = xp_for_level(current_level);
    let next_level_xp = xp_for_level(current_level + 1);

    if next_level_xp == current_level_xp {
        return 0.0;
    }

    (xp - current_level_xp) as f64 / (next_level_xp - current_level_xp) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_level_zero_xp() {
        assert_eq!(calculate_level(0), 1);
    }

    #[test]
    fn test_calculate_level_negative_xp() {
        assert_eq!(calculate_level(-100), 1);
    }

    #[test]
    fn test_calculate_level_99_xp_still_level_1() {
        assert_eq!(calculate_level(99), 1);
    }

    #[test]
    fn test_calculate_level_100_xp_is_level_2() {
        assert_eq!(calculate_level(100), 2);
    }

    #[test]
    fn test_calculate_level_400_xp_is_level_3() {
        assert_eq!(calculate_level(400), 3);
    }

    #[test]
    fn test_calculate_level_900_xp_is_level_4() {
        assert_eq!(calculate_level(900), 4);
    }

    #[test]
    fn test_calculate_level_1600_xp_is_level_5() {
        assert_eq!(calculate_level(1600), 5);
    }

    #[test]
    fn test_calculate_level_8100_xp_is_level_10() {
        assert_eq!(calculate_level(8100), 10);
    }

    #[test]
    fn test_calculate_level_boundary_just_below_level_3() {
        assert_eq!(calculate_level(399), 2);
    }

    #[test]
    fn test_xp_for_level_1() {
        assert_eq!(xp_for_level(1), 0);
    }

    #[test]
    fn test_xp_for_level_2() {
        assert_eq!(xp_for_level(2), 100);
    }

    #[test]
    fn test_xp_for_level_5() {
        assert_eq!(xp_for_level(5), 1600);
    }

    #[test]
    fn test_xp_for_level_10() {
        assert_eq!(xp_for_level(10), 8100);
    }

    #[test]
    fn test_level_progress_at_level_start() {
        // At exactly level 2 (100 XP), progress should be 0.0
        assert!((level_progress(100) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_level_progress_halfway() {
        // Level 2 is 100 XP, Level 3 is 400 XP, range = 300
        // At 250 XP: (250 - 100) / 300 = 150/300 = 0.5
        assert!((level_progress(250) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_level_progress_near_level_up() {
        // At 399 XP, still level 2, almost at level 3 (400)
        // (399 - 100) / 300 = 299/300 ≈ 0.997
        let progress = level_progress(399);
        assert!(progress > 0.99);
        assert!(progress < 1.0);
    }

    #[test]
    fn test_round_trip_level_xp() {
        // For any level, xp_for_level then calculate_level should return that level
        for level in 1..=20 {
            let xp = xp_for_level(level);
            assert_eq!(
                calculate_level(xp),
                level,
                "Round trip failed for level {}",
                level
            );
        }
    }
}

/// A tracked GitHub repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: Uuid,
    pub github_id: i64,
    pub owner: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// A GitHub user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub github_id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub xp: i64,
    pub level: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A pull request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: Uuid,
    pub repo_id: Uuid,
    pub github_id: i64,
    pub number: i32,
    pub title: String,
    pub author_id: Uuid,
    pub state: PrState,
    pub created_at: DateTime<Utc>,
    pub first_review_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

/// A PR review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: Uuid,
    pub pr_id: Uuid,
    pub reviewer_id: Uuid,
    pub github_id: i64,
    pub state: ReviewState,
    pub body: Option<String>,
    pub comments_count: i32,
    pub submitted_at: DateTime<Utc>,
}

/// A commit on a PR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: Uuid,
    pub pr_id: Uuid,
    pub sha: String,
    pub author_id: Option<Uuid>,
    pub committed_at: DateTime<Utc>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    Commented,
    Dismissed,
    Pending,
}

/// An achievement definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub emoji: String,
    pub xp_reward: i32,
    pub rarity: AchievementRarity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AchievementRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// A user's unlocked achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAchievement {
    pub user_id: Uuid,
    pub achievement_id: String,
    pub unlocked_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
}

/// A competitive season
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub id: Uuid,
    pub name: String,
    pub number: i32,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

/// User stats for a specific time period
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserStats {
    pub reviews_given: i32,
    pub prs_reviewed: i32,
    pub first_reviews: i32,
    pub comments_written: i32,
    pub prs_authored: i32,
    pub prs_merged: i32,
    pub avg_time_to_first_review_secs: Option<f64>,
    pub avg_review_depth: Option<f64>,
    pub review_streak_days: i32,
}

/// Leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub user: User,
    pub score: i64,
    pub stats: UserStats,
}
