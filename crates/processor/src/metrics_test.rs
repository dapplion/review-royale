#[cfg(test)]
mod tests {
    use crate::metrics::*;
    use chrono::{TimeZone, Utc};
    use common::models::{PrState, PullRequest};
    use uuid::Uuid;

    fn make_pr(
        created_at: chrono::DateTime<Utc>,
        first_review_at: Option<chrono::DateTime<Utc>>,
    ) -> PullRequest {
        PullRequest {
            id: Uuid::new_v4(),
            repo_id: Uuid::new_v4(),
            github_id: 1,
            number: 1,
            title: "Test PR".to_string(),
            author_id: Uuid::new_v4(),
            state: PrState::Open,
            created_at,
            first_review_at,
            merged_at: None,
            closed_at: None,
        }
    }

    // time_to_first_review tests
    #[test]
    fn test_time_to_first_review_none_if_no_review() {
        let pr = make_pr(Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(), None);
        assert_eq!(time_to_first_review(&pr), None);
    }

    #[test]
    fn test_time_to_first_review_30_minutes() {
        let created = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let first_review = Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap();
        let pr = make_pr(created, Some(first_review));

        assert_eq!(time_to_first_review(&pr), Some(30 * 60)); // 30 minutes in seconds
    }

    #[test]
    fn test_time_to_first_review_2_days() {
        let created = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let first_review = Utc.with_ymd_and_hms(2026, 1, 3, 10, 0, 0).unwrap();
        let pr = make_pr(created, Some(first_review));

        assert_eq!(time_to_first_review(&pr), Some(2 * 24 * 60 * 60)); // 2 days in seconds
    }

    // is_fast_review tests
    #[test]
    fn test_is_fast_review_under_1_hour() {
        let pr_created = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let review_submitted = Utc.with_ymd_and_hms(2026, 1, 1, 10, 30, 0).unwrap();

        assert!(is_fast_review(pr_created, review_submitted));
    }

    #[test]
    fn test_is_fast_review_exactly_1_hour_not_fast() {
        let pr_created = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let review_submitted = Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap();

        assert!(!is_fast_review(pr_created, review_submitted));
    }

    #[test]
    fn test_is_fast_review_over_1_hour() {
        let pr_created = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let review_submitted = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();

        assert!(!is_fast_review(pr_created, review_submitted));
    }

    #[test]
    fn test_is_fast_review_59_minutes() {
        let pr_created = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let review_submitted = Utc.with_ymd_and_hms(2026, 1, 1, 10, 59, 0).unwrap();

        assert!(is_fast_review(pr_created, review_submitted));
    }

    // is_first_review tests
    #[test]
    fn test_is_first_review_no_previous_review() {
        let pr = make_pr(Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(), None);
        let review_time = Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap();

        assert!(is_first_review(&pr, review_time));
    }

    #[test]
    fn test_is_first_review_matches_first_review_time() {
        let first_review_time = Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap();
        let pr = make_pr(
            Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            Some(first_review_time),
        );

        assert!(is_first_review(&pr, first_review_time));
    }

    #[test]
    fn test_is_first_review_after_first_review() {
        let first_review_time = Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap();
        let later_review_time = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let pr = make_pr(
            Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap(),
            Some(first_review_time),
        );

        assert!(!is_first_review(&pr, later_review_time));
    }

    // review_depth_score tests
    #[test]
    fn test_review_depth_score_no_comments() {
        let score = review_depth_score(0);
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_review_depth_score_1_comment() {
        let score = review_depth_score(1);
        // 0.5 + ln(1) * 0.5 = 0.5 + 0 = 0.5
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_review_depth_score_increasing_with_comments() {
        let score_1 = review_depth_score(1);
        let score_5 = review_depth_score(5);
        let score_10 = review_depth_score(10);

        assert!(score_5 > score_1);
        assert!(score_10 > score_5);
    }

    #[test]
    fn test_review_depth_score_diminishing_returns() {
        // Verify logarithmic growth (diminishing returns)
        let diff_1_to_5 = review_depth_score(5) - review_depth_score(1);
        let diff_5_to_9 = review_depth_score(9) - review_depth_score(5);

        // Adding 4 comments from 5→9 should give less bonus than 1→5
        assert!(diff_5_to_9 < diff_1_to_5);
    }

    // is_stale tests
    #[test]
    fn test_is_stale_just_created() {
        let now = Utc::now();
        assert!(!is_stale(now, 7));
    }

    #[test]
    fn test_is_stale_old_activity() {
        let old_time = Utc::now() - chrono::Duration::days(10);
        assert!(is_stale(old_time, 7));
    }

    #[test]
    fn test_is_stale_exactly_threshold() {
        let old_time = Utc::now() - chrono::Duration::days(7);
        assert!(is_stale(old_time, 7));
    }

    #[test]
    fn test_staleness_days_calculation() {
        let old_time = Utc::now() - chrono::Duration::days(5);
        assert_eq!(staleness_days(old_time), 5);
    }
}
