//! XP recalculation based on new session-based rules

use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::sessions::{calculate_session_xp_with_quality, group_reviews_into_sessions};

/// Recalculate all user XP from scratch based on review sessions
pub async fn recalculate_all_xp(pool: &PgPool) -> Result<RecalculationStats, sqlx::Error> {
    info!("Starting XP recalculation for all users");

    // Step 1: Reset all user XP and review xp_earned
    info!("Resetting all user XP and review xp_earned to 0");
    sqlx::query("UPDATE users SET xp = 0, level = 1")
        .execute(pool)
        .await?;
    // Note: review_sessions column may not exist yet, ignore errors
    let _ = sqlx::query("UPDATE users SET review_sessions = 0")
        .execute(pool)
        .await;
    // Note: xp_earned column may not exist yet, ignore errors
    let _ = sqlx::query("UPDATE reviews SET xp_earned = 0")
        .execute(pool)
        .await;

    // Step 2: Get all reviews and commits
    info!("Fetching all reviews");
    let reviews = db::reviews::list_all(pool).await?;
    info!("Fetched {} reviews", reviews.len());

    info!("Fetching all commits");
    let commits = db::commits::list_all(pool).await?;
    info!("Fetched {} commits", commits.len());

    // Step 3: Group reviews by (pr_id, reviewer_id)
    let mut review_groups: std::collections::HashMap<(Uuid, Uuid), Vec<_>> =
        std::collections::HashMap::new();
    for review in reviews {
        review_groups
            .entry((review.pr_id, review.reviewer_id))
            .or_default()
            .push(review);
    }

    info!(
        "Grouped reviews into {} unique (pr, reviewer) pairs",
        review_groups.len()
    );

    // Step 4: Process each group into sessions and award XP
    let total_reviews_count: usize = review_groups.values().map(|v| v.len()).sum();
    let mut total_sessions = 0;
    let mut total_xp_awarded = 0i64;
    let mut users_updated = std::collections::HashSet::new();
    let mut user_session_counts: std::collections::HashMap<Uuid, i32> =
        std::collections::HashMap::new();

    for ((pr_id, reviewer_id), pr_reviews) in review_groups {
        // Get commits for this PR
        let pr_commits: Vec<_> = commits
            .iter()
            .filter(|c| c.pr_id == pr_id)
            .cloned()
            .collect();

        // Get quality data for this PR/user combination
        let quality_data =
            db::review_comments::get_quality_data_for_pr_user(pool, pr_id, reviewer_id)
                .await
                .ok();

        // Group into sessions
        let sessions = group_reviews_into_sessions(pr_reviews, pr_commits.clone());
        let session_count = sessions.len() as i32;
        total_sessions += session_count as usize;

        // Track sessions per user
        *user_session_counts.entry(reviewer_id).or_insert(0) += session_count;

        // Calculate XP for each session
        for session in sessions {
            // Find the most recent commit before this session
            let commit_before = pr_commits
                .iter()
                .filter(|c| c.committed_at < session.started_at)
                .max_by_key(|c| c.committed_at)
                .map(|c| c.committed_at);

            let xp =
                calculate_session_xp_with_quality(&session, commit_before, quality_data.as_ref());

            if xp > 0 {
                // Award XP to user
                let _ = db::users::add_xp(pool, reviewer_id, xp).await;
                total_xp_awarded += xp;
                users_updated.insert(reviewer_id);

                // Store XP on the first review of the session for period filtering
                if let Some(first_review) = session.reviews.first() {
                    let _ = sqlx::query("UPDATE reviews SET xp_earned = $1 WHERE id = $2")
                        .bind(xp as i32)
                        .bind(first_review.id)
                        .execute(pool)
                        .await;
                }
            }
        }
    }

    // Step 5: Update session counts for all users (if column exists)
    info!("Updating session counts for {} users", user_session_counts.len());
    for (user_id, session_count) in &user_session_counts {
        // Note: review_sessions column may not exist yet, ignore errors
        let _ = sqlx::query("UPDATE users SET review_sessions = $1 WHERE id = $2")
            .bind(*session_count)
            .bind(*user_id)
            .execute(pool)
            .await;
    }

    info!(
        "Recalculation complete: {} sessions, {} XP awarded, {} users updated",
        total_sessions,
        total_xp_awarded,
        users_updated.len()
    );

    Ok(RecalculationStats {
        total_reviews: total_reviews_count,
        total_sessions,
        total_xp_awarded,
        users_updated: users_updated.len(),
    })
}

#[derive(Debug)]
pub struct RecalculationStats {
    pub total_reviews: usize,
    pub total_sessions: usize,
    pub total_xp_awarded: i64,
    pub users_updated: usize,
}
