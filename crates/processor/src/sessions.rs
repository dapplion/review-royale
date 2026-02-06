//! Review session grouping logic

use chrono::{DateTime, Duration, Utc};
use common::models::{Commit, Review};
use uuid::Uuid;

/// A grouped review session
#[derive(Debug, Clone)]
pub struct ReviewSession {
    pub pr_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviews: Vec<Review>,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub total_comments: i32,
}

/// Group reviews into sessions based on commits and time gaps
pub fn group_reviews_into_sessions(
    reviews: Vec<Review>,
    commits: Vec<Commit>,
) -> Vec<ReviewSession> {
    if reviews.is_empty() {
        return Vec::new();
    }

    // Sort reviews by submitted_at
    let mut sorted_reviews = reviews;
    sorted_reviews.sort_by_key(|r| r.submitted_at);

    // Sort commits by committed_at
    let mut sorted_commits = commits;
    sorted_commits.sort_by_key(|c| c.committed_at);

    let mut sessions = Vec::new();
    let mut current_session_reviews = Vec::new();
    let mut last_review_time: Option<DateTime<Utc>> = None;

    for review in sorted_reviews {
        let should_start_new_session = if let Some(last_time) = last_review_time {
            // Check 24-hour gap
            let time_gap = review.submitted_at.signed_duration_since(last_time);
            if time_gap > Duration::hours(24) {
                true
            } else {
                // Check if commits pushed between last review and this one
                let commits_between = sorted_commits.iter().any(|c| {
                    c.pr_id == review.pr_id
                        && c.committed_at > last_time
                        && c.committed_at < review.submitted_at
                });
                commits_between
            }
        } else {
            false
        };

        if should_start_new_session && !current_session_reviews.is_empty() {
            // Finalize current session
            if let Some(session) = finalize_session(current_session_reviews.clone()) {
                sessions.push(session);
            }
            current_session_reviews.clear();
        }

        current_session_reviews.push(review.clone());
        last_review_time = Some(review.submitted_at);
    }

    // Finalize last session
    if !current_session_reviews.is_empty() {
        if let Some(session) = finalize_session(current_session_reviews) {
            sessions.push(session);
        }
    }

    sessions
}

fn finalize_session(reviews: Vec<Review>) -> Option<ReviewSession> {
    if reviews.is_empty() {
        return None;
    }

    let pr_id = reviews[0].pr_id;
    let reviewer_id = reviews[0].reviewer_id;
    let started_at = reviews.iter().map(|r| r.submitted_at).min()?;
    let ended_at = reviews.iter().map(|r| r.submitted_at).max()?;
    let total_comments: i32 = reviews.iter().map(|r| r.comments_count).sum();

    Some(ReviewSession {
        pr_id,
        reviewer_id,
        reviews,
        started_at,
        ended_at,
        total_comments,
    })
}

/// Calculate XP for a review session
pub fn calculate_session_xp(
    session: &ReviewSession,
    commit_before_session: Option<DateTime<Utc>>,
) -> i64 {
    // Check minimum threshold: at least 1 comment or state change
    let has_state_change = session.reviews.iter().any(|r| {
        r.state == common::models::ReviewState::Approved
            || r.state == common::models::ReviewState::ChangesRequested
    });

    if session.total_comments == 0 && !has_state_change {
        // Rubber stamp - no credit
        return 0;
    }

    // Check for quick approval (< 1 min, 0 comments) = rubber stamp
    if has_state_change
        && session.total_comments == 0
        && session.ended_at.signed_duration_since(session.started_at) < Duration::minutes(1)
    {
        return 0;
    }

    let mut xp: i64 = 10; // Base XP

    // Comments: +5 XP per comment (already counted in comments_count)
    xp += session.total_comments as i64 * 5;

    // Fast review: +10 XP if reviewed <1 hour after commits pushed
    if let Some(commit_time) = commit_before_session {
        let review_delay = session.started_at.signed_duration_since(commit_time);
        if review_delay < Duration::hours(1) && review_delay > Duration::seconds(0) {
            xp += 10;
        }
    }

    // Thorough: +5 XP if >5 comments
    if session.total_comments > 5 {
        xp += 5;
    }

    // Deep review: +10 XP if >10 comments
    if session.total_comments > 10 {
        xp += 10;
    }

    xp
}
