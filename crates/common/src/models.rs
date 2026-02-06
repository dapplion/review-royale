//! Domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
