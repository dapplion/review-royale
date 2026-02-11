//! Tests using real GitHub data fixtures from Jimmy's reviews
//!
//! These tests verify that review session grouping works correctly
//! with actual data patterns from the lighthouse repository.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::sessions::group_reviews_into_sessions;
use common::models::{Commit, Review, ReviewState};

/// Review data from GitHub API (simplified)
#[derive(Debug, Deserialize)]
struct ReviewFixture {
    id: i64,
    state: String,
    submitted_at: String,
    body: Option<String>,
}

/// Commit data from GitHub API (simplified)
#[derive(Debug, Deserialize)]
struct CommitFixture {
    sha: String,
    date: String,
    message: String,
}

fn load_reviews(json: &str) -> Vec<Review> {
    let pr_id = Uuid::new_v4();
    let reviewer_id = Uuid::new_v4();

    let fixtures: Vec<ReviewFixture> = serde_json::from_str(json).expect("Invalid JSON");

    fixtures
        .into_iter()
        .map(|f| Review {
            id: Uuid::new_v4(),
            pr_id,
            reviewer_id,
            github_id: f.id,
            state: match f.state.as_str() {
                "APPROVED" => ReviewState::Approved,
                "CHANGES_REQUESTED" => ReviewState::ChangesRequested,
                _ => ReviewState::Commented,
            },
            body: f.body,
            comments_count: 1, // Each review event = 1 comment for simplicity
            submitted_at: f.submitted_at.parse::<DateTime<Utc>>().expect("Invalid date"),
        })
        .collect()
}

fn load_commits(json: &str, pr_id: Uuid) -> Vec<Commit> {
    let fixtures: Vec<CommitFixture> = serde_json::from_str(json).expect("Invalid JSON");

    fixtures
        .into_iter()
        .map(|f| Commit {
            id: Uuid::new_v4(),
            pr_id,
            sha: f.sha,
            author_id: None,
            committed_at: f.date.parse::<DateTime<Utc>>().expect("Invalid date"),
            message: Some(f.message),
            created_at: Utc::now(),
        })
        .collect()
}

/// PR #8754: Jimmy made 18 review events over ~21 hours
///
/// Reviews timeline:
/// - Feb 9 06:46-06:57: 5 reviews (morning burst)
/// - Feb 9 10:46-12:55: 9 reviews (midday session)
/// - Feb 9 22:38-22:44: 2 reviews (evening)
/// - Feb 10 00:32: 1 review (late night)
/// - Feb 10 03:48: 1 review (after new commits at 03:33)
///
/// Commits at Feb 10 03:33-03:34 create a session boundary.
///
/// Expected: 2 sessions
/// - Session 1: 17 reviews (Feb 9 06:46 to Feb 10 00:32)
/// - Session 2: 1 review (Feb 10 03:48, after commits)
#[test]
fn test_pr_8754_jimmy_18_reviews_2_sessions() {
    let reviews_json = include_str!("fixtures/pr_8754_reviews.json");
    let commits_json = include_str!("fixtures/pr_8754_commits.json");

    let reviews = load_reviews(reviews_json);
    let pr_id = reviews[0].pr_id;
    let commits = load_commits(commits_json, pr_id);

    assert_eq!(reviews.len(), 18, "Should have 18 review events");
    assert_eq!(commits.len(), 30, "Should have 30 commits");

    let sessions = group_reviews_into_sessions(reviews, commits);

    assert_eq!(
        sessions.len(),
        2,
        "18 reviews with commits between should yield 2 sessions"
    );

    // First session: all reviews before the commits at Feb 10 03:33
    assert_eq!(
        sessions[0].reviews.len(),
        17,
        "First session should have 17 reviews"
    );

    // Second session: review after commits
    assert_eq!(
        sessions[1].reviews.len(),
        1,
        "Second session should have 1 review (after commits)"
    );

    // Verify session boundaries
    let session1_end = sessions[0].ended_at;
    let session2_start = sessions[1].started_at;
    assert!(
        session2_start > session1_end,
        "Session 2 should start after session 1 ends"
    );
}

/// PR #7944: Jimmy made 14 review events over ~2 months
///
/// Reviews timeline:
/// - Nov 26, 2025: 3 reviews (06:41-06:44) in 3 minutes
/// - Jan 12, 2026: 2 reviews (06:38-06:44) - 47 days later
/// - Jan 30, 2026: 9 reviews (03:14-04:49) - 18 days later
///
/// No commits between Jimmy's reviews (last commit was Oct 9, 2025).
/// Session boundaries are created by >24h gaps.
///
/// Expected: 3 sessions (one per date cluster)
#[test]
fn test_pr_7944_jimmy_14_reviews_3_sessions() {
    let reviews_json = include_str!("fixtures/pr_7944_reviews.json");
    let commits_json = include_str!("fixtures/pr_7944_commits.json");

    let reviews = load_reviews(reviews_json);
    let pr_id = reviews[0].pr_id;
    let commits = load_commits(commits_json, pr_id);

    assert_eq!(reviews.len(), 14, "Should have 14 review events");

    let sessions = group_reviews_into_sessions(reviews, commits);

    assert_eq!(
        sessions.len(),
        3,
        "14 reviews across 3 date clusters should yield 3 sessions"
    );

    // Session 1: Nov 26 (3 reviews)
    assert_eq!(
        sessions[0].reviews.len(),
        3,
        "First session (Nov 26) should have 3 reviews"
    );

    // Session 2: Jan 12 (2 reviews)
    assert_eq!(
        sessions[1].reviews.len(),
        2,
        "Second session (Jan 12) should have 2 reviews"
    );

    // Session 3: Jan 30 (9 reviews)
    assert_eq!(
        sessions[2].reviews.len(),
        9,
        "Third session (Jan 30) should have 9 reviews"
    );
}

/// Verify that sessions are ordered chronologically
#[test]
fn test_sessions_are_chronologically_ordered() {
    let reviews_json = include_str!("fixtures/pr_7944_reviews.json");
    let commits_json = include_str!("fixtures/pr_7944_commits.json");

    let reviews = load_reviews(reviews_json);
    let pr_id = reviews[0].pr_id;
    let commits = load_commits(commits_json, pr_id);

    let sessions = group_reviews_into_sessions(reviews, commits);

    for i in 1..sessions.len() {
        assert!(
            sessions[i].started_at > sessions[i - 1].ended_at,
            "Session {} should start after session {} ends",
            i,
            i - 1
        );
    }
}

/// Verify comment counting within sessions
#[test]
fn test_session_comment_counts() {
    let reviews_json = include_str!("fixtures/pr_8754_reviews.json");
    let commits_json = include_str!("fixtures/pr_8754_commits.json");

    let reviews = load_reviews(reviews_json);
    let pr_id = reviews[0].pr_id;
    let commits = load_commits(commits_json, pr_id);

    let sessions = group_reviews_into_sessions(reviews, commits);

    // Each review has 1 comment in our fixture loading
    assert_eq!(sessions[0].total_comments, 17);
    assert_eq!(sessions[1].total_comments, 1);
}
