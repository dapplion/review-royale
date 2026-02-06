//! Metrics computation

use chrono::{DateTime, Duration, Utc};
use common::models::PullRequest;

/// Calculate time to first review in seconds
pub fn time_to_first_review(pr: &PullRequest) -> Option<i64> {
    pr.first_review_at.map(|first_review| {
        (first_review - pr.created_at).num_seconds()
    })
}

/// Calculate if a review was "fast" (under 4 hours)
pub fn is_fast_review(pr_created: DateTime<Utc>, review_submitted: DateTime<Utc>) -> bool {
    let diff = review_submitted - pr_created;
    diff < Duration::hours(4)
}

/// Calculate if this was the first review on a PR
pub fn is_first_review(pr: &PullRequest, review_submitted: DateTime<Utc>) -> bool {
    match pr.first_review_at {
        Some(first) => first == review_submitted,
        None => true, // If no first_review_at set, this is the first
    }
}

/// Calculate review depth score based on comment count
pub fn review_depth_score(comments_count: i32) -> f64 {
    // Logarithmic scaling: more comments = higher score, but diminishing returns
    if comments_count == 0 {
        0.5 // Base score for a review with no comments
    } else {
        0.5 + (comments_count as f64).ln() * 0.5
    }
}

/// Calculate staleness in days
pub fn staleness_days(last_activity: DateTime<Utc>) -> i64 {
    (Utc::now() - last_activity).num_days()
}

/// Check if a PR is stale (no activity for X days)
pub fn is_stale(last_activity: DateTime<Utc>, threshold_days: i64) -> bool {
    staleness_days(last_activity) >= threshold_days
}
