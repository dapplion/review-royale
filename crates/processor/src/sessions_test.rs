#[cfg(test)]
mod tests {
    use crate::sessions::{
        calculate_session_xp, calculate_session_xp_with_quality, group_reviews_into_sessions,
        ReviewSession,
    };
    use chrono::{DateTime, TimeZone, Utc};
    use common::models::{Commit, Review, ReviewState};
    use db::review_comments::CommentQualityData;
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
        let commit_before = Some(Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap());

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
        let commit_before = Some(Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap());

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
        // 10 base + 8*5 comments + 5 thorough (>5 comments) = 55 XP
        assert_eq!(
            xp, 55,
            "Jimmy's session = 55 XP (base + comments + thorough)"
        );
    }

    #[test]
    fn test_xp_quality_weighted_high_quality() {
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
                5,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
            total_comments: 5,
        };

        // All 5 comments are high quality (score 7-10)
        let quality_data = CommentQualityData {
            by_tier: (0, 0, 5),     // (low, medium, high)
            by_category: (2, 1, 2), // (logic, structural, other)
            categorized_count: 5,
        };

        let xp = calculate_session_xp_with_quality(&session, None, Some(&quality_data));
        // 10 base + 5*8 high quality + 2*3 logic bonus + 1*2 structural bonus = 10 + 40 + 6 + 2 = 58 XP
        assert_eq!(
            xp, 58,
            "5 high-quality comments with category bonuses = 58 XP"
        );
    }

    #[test]
    fn test_xp_quality_weighted_mixed() {
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
                8,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
            total_comments: 8,
        };

        // Mixed quality: 2 low, 3 medium, 3 high, 1 logic bug
        let quality_data = CommentQualityData {
            by_tier: (2, 3, 3),     // (low, medium, high)
            by_category: (1, 0, 7), // (logic, structural, other)
            categorized_count: 8,
        };

        let xp = calculate_session_xp_with_quality(&session, None, Some(&quality_data));
        // 10 base + 2*2 low + 3*5 medium + 3*8 high + 1*3 logic + 5 thorough = 10 + 4 + 15 + 24 + 3 + 5 = 61 XP
        assert_eq!(xp, 61, "Mixed quality 8 comments = 61 XP");
    }

    #[test]
    fn test_xp_quality_weighted_low_quality() {
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
                3,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
            total_comments: 3,
        };

        // All 3 comments are low quality (nits)
        let quality_data = CommentQualityData {
            by_tier: (3, 0, 0),     // (low, medium, high)
            by_category: (0, 0, 3), // (logic, structural, other)
            categorized_count: 3,
        };

        let xp_with_quality =
            calculate_session_xp_with_quality(&session, None, Some(&quality_data));
        let xp_without_quality = calculate_session_xp(&session, None);

        // With quality: 10 base + 3*2 low = 16 XP
        // Without quality: 10 base + 3*5 = 25 XP
        assert_eq!(xp_with_quality, 16, "3 low-quality comments = 16 XP");
        assert_eq!(xp_without_quality, 25, "Same without quality data = 25 XP");
        assert!(
            xp_with_quality < xp_without_quality,
            "Low quality should earn less XP"
        );
    }

    /// Test case from real data: jimmygchen reviewing PR #8754
    /// 18 review events should become 2 sessions because:
    /// - Reviews 1-17 (Feb 9 06:46 to Feb 10 00:32) are on same code version
    /// - Review 18 (Feb 10 03:48) is after new commits at 03:33
    #[test]
    fn test_jimmy_pr8754_real_data() {
        let pr_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();

        // Helper to create reviews more concisely
        let r = |ts: &str| {
            make_review(
                pr_id,
                reviewer_id,
                ts.parse().unwrap(),
                ReviewState::Commented,
                1,
            )
        };

        // Jimmy's 18 review events from the GitHub API
        let reviews = vec![
            r("2026-02-09T06:46:26Z"),
            r("2026-02-09T06:49:48Z"),
            r("2026-02-09T06:52:42Z"),
            r("2026-02-09T06:55:26Z"),
            r("2026-02-09T06:57:03Z"),
            r("2026-02-09T10:46:20Z"),
            r("2026-02-09T10:54:42Z"),
            r("2026-02-09T11:07:20Z"),
            r("2026-02-09T11:07:55Z"),
            r("2026-02-09T11:24:42Z"),
            r("2026-02-09T11:32:29Z"),
            r("2026-02-09T11:34:17Z"),
            r("2026-02-09T11:41:31Z"),
            r("2026-02-09T12:55:35Z"),
            r("2026-02-09T22:38:32Z"),
            r("2026-02-09T22:44:24Z"),
            r("2026-02-10T00:32:44Z"),
            r("2026-02-10T03:48:00Z"), // After new commits
        ];

        // Commits - the key ones are at 03:33 and 03:34 on Feb 10
        let commits = vec![
            make_commit(pr_id, "2026-02-05T02:47:40Z".parse().unwrap()),
            make_commit(pr_id, "2026-02-10T03:33:12Z".parse().unwrap()), // New!
            make_commit(pr_id, "2026-02-10T03:34:40Z".parse().unwrap()), // New!
        ];

        let sessions = group_reviews_into_sessions(reviews, commits);

        // Should be 2 sessions, not 18
        assert_eq!(
            sessions.len(),
            2,
            "Expected 2 sessions, got {}",
            sessions.len()
        );
        assert_eq!(sessions[0].reviews.len(), 17);
        assert_eq!(sessions[0].total_comments, 17);
        assert_eq!(sessions[1].reviews.len(), 1);
    }

    #[test]
    fn test_xp_quality_with_uncategorized() {
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
                6,
            )],
            started_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap(),
            total_comments: 6,
        };

        // Only 4 categorized, 2 uncategorized
        let quality_data = CommentQualityData {
            by_tier: (1, 2, 1),     // (low, medium, high) = 4 total
            by_category: (1, 0, 3), // (logic, structural, other) = 4 total
            categorized_count: 4,
        };

        let xp = calculate_session_xp_with_quality(&session, None, Some(&quality_data));
        // 10 base + 1*2 low + 2*5 medium + 1*8 high + 2*5 uncategorized + 1*3 logic + 5 thorough
        // = 10 + 2 + 10 + 8 + 10 + 3 + 5 = 48 XP
        assert_eq!(xp, 48, "4 categorized + 2 uncategorized = 48 XP");
    }
}
