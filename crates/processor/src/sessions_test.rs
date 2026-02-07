#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use common::models::{Commit, Review, ReviewState};
    use uuid::Uuid;

    fn make_review(
        pr_id: Uuid,
        reviewer_id: Uuid,
        submitted_at: DateTime<Utc>,
        state: ReviewState,
        comments: i32,
    ) -> Review {
        Review {
            id: Uuid::new_v4(),
            pr_id,
            reviewer_id,
            github_id: 1,
            state,
            body: None,
            comments_count: comments,
            submitted_at,
        }
    }

    fn make_commit(pr_id: Uuid, committed_at: DateTime<Utc>) -> Commit {
        Commit {
            id: Uuid::new_v4(),
            pr_id,
            sha: format!("sha{}", committed_at.timestamp()),
            author_id: None,
            committed_at,
            message: None,
            created_at: committed_at,
        }
    }

    #[test]
    fn test_single_review_session() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let reviews = vec![make_review(
            pr_id,
            reviewer_id,
            Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ReviewState::Approved,
            5,
        )];

        let sessions = group_reviews_into_sessions(reviews, vec![]);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].total_comments, 5);
    }

    #[test]
    fn test_multiple_comments_same_hour_one_session() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        // 9 comments over 90 minutes = 1 session
        let reviews = vec![
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 15, 0).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 45, 0).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 11, 10, 0).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 11, 20, 0).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 11, 25, 0).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 11, 30, 0).unwrap(),
                ReviewState::ChangesRequested,
                0,
            ),
        ];

        let sessions = group_reviews_into_sessions(reviews, vec![]);

        assert_eq!(sessions.len(), 1, "9 comments in 90 min = 1 session");
        assert_eq!(sessions[0].total_comments, 8);
    }

    #[test]
    fn test_24h_gap_creates_new_session() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let reviews = vec![
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
                ReviewState::Commented,
                3,
            ),
            // 25 hours later
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 2, 11, 0, 0).unwrap(),
                ReviewState::Approved,
                2,
            ),
        ];

        let sessions = group_reviews_into_sessions(reviews, vec![]);

        assert_eq!(sessions.len(), 2, "25h gap = 2 sessions");
        assert_eq!(sessions[0].total_comments, 3);
        assert_eq!(sessions[1].total_comments, 2);
    }

    #[test]
    fn test_commits_create_session_boundary() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let reviews = vec![
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
                ReviewState::Commented,
                3,
            ),
            // Review 30 min later, BUT author pushed commits in between
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
                ReviewState::Approved,
                2,
            ),
        ];

        // Commits pushed between the two reviews
        let commits = vec![make_commit(
            pr_id,
            Utc.with_ymd_and_hms(2026, 1, 1, 10, 15, 0).unwrap(),
        )];

        let sessions = group_reviews_into_sessions(reviews, commits);

        assert_eq!(
            sessions.len(),
            2,
            "Commits between reviews = 2 sessions (new code to review)"
        );
        assert_eq!(sessions[0].total_comments, 3);
        assert_eq!(sessions[1].total_comments, 2);
    }

    #[test]
    fn test_xp_base_review() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let session = ReviewSession {
            pr_id,
            reviewer_id,
            reviews: vec![make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
                ReviewState::Approved,
                0,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 5, 0).unwrap(),
            total_comments: 0,
        };

        let xp = calculate_session_xp(&session, None);
        assert_eq!(xp, 10, "Base review with state change = 10 XP");
    }

    #[test]
    fn test_xp_rubber_stamp_no_credit() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let session = ReviewSession {
            pr_id,
            reviewer_id,
            reviews: vec![make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
                ReviewState::Approved,
                0,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 30).unwrap(), // 30 sec
            total_comments: 0,
        };

        let xp = calculate_session_xp(&session, None);
        assert_eq!(xp, 0, "Rubber stamp (0 comments, <1 min) = 0 XP");
    }

    #[test]
    fn test_xp_with_comments() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let session = ReviewSession {
            pr_id,
            reviewer_id,
            reviews: vec![make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
                ReviewState::ChangesRequested,
                7,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
            total_comments: 7,
        };

        let xp = calculate_session_xp(&session, None);
        // 10 base + 7*5 comments + 5 thorough (>5 comments) = 50 XP
        assert_eq!(xp, 50, "Base + 7 comments + thorough = 50 XP");
    }

    #[test]
    fn test_xp_deep_review() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let session = ReviewSession {
            pr_id,
            reviewer_id,
            reviews: vec![make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
                ReviewState::ChangesRequested,
                12,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap(),
            total_comments: 12,
        };

        let xp = calculate_session_xp(&session, None);
        // 10 base + 12*5 comments + 5 thorough + 10 deep (>10 comments) = 85 XP
        assert_eq!(xp, 85, "Base + 12 comments + thorough + deep = 85 XP");
    }

    #[test]
    fn test_xp_fast_review_bonus() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let session = ReviewSession {
            pr_id,
            reviewer_id,
            reviews: vec![make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
                ReviewState::Approved,
                3,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 45, 0).unwrap(),
            total_comments: 3,
        };

        // Commits pushed 30 min before review
        let commit_before =
            Some(Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap());

        let xp = calculate_session_xp(&session, commit_before);
        // 10 base + 3*5 comments + 10 fast (<1h) = 35 XP
        assert_eq!(xp, 35, "Base + 3 comments + fast = 35 XP");
    }

    #[test]
    fn test_xp_no_fast_bonus_if_too_slow() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let session = ReviewSession {
            pr_id,
            reviewer_id,
            reviews: vec![make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
                ReviewState::Approved,
                3,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 12, 15, 0).unwrap(),
            total_comments: 3,
        };

        // Commits pushed 2 hours before review
        let commit_before =
            Some(Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap());

        let xp = calculate_session_xp(&session, commit_before);
        // 10 base + 3*5 comments (no fast bonus) = 25 XP
        assert_eq!(xp, 25, "Base + 3 comments (no fast bonus) = 25 XP");
    }

    #[test]
    fn test_realistic_scenario_jimmy_pr_7944() {
        // Simulate jimmy's 9 comments on PR #7944 in 90 minutes
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        let reviews = vec![
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 3, 14, 30).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 3, 21, 1).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 3, 33, 57).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 3, 48, 54).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 3, 50, 30).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 3, 50, 40).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 3, 55, 40).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 4, 46, 37).unwrap(),
                ReviewState::Commented,
                1,
            ),
            make_review(
                pr_id,
                reviewer_id,
                Utc.with_ymd_and_hms(2026, 1, 30, 4, 49, 27).unwrap(),
                ReviewState::ChangesRequested,
                0,
            ),
        ];

        let sessions = group_reviews_into_sessions(reviews, vec![]);

        assert_eq!(sessions.len(), 1, "9 review events = 1 session");
        assert_eq!(sessions[0].total_comments, 8);

        let xp = calculate_session_xp(&sessions[0], None);
        // 10 base + 8*5 comments = 50 XP
        assert_eq!(xp, 50, "Jimmy's session = 50 XP not 90 XP");
    }
}
