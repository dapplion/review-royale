//! Backfill historical data from GitHub API

use chrono::Utc;
use common::models::{PrState, ReviewState};
use github::{GitHubClient, GithubPr};
use sqlx::PgPool;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::scores::ScoreCalculator;

#[derive(Error, Debug)]
pub enum BackfillError {
    #[error("GitHub API error: {0}")]
    GitHub(#[from] github::client::ClientError),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Rate limited, retry after {0} seconds")]
    RateLimited(u64),
}

/// Progress update for backfill operations
#[derive(Debug, Clone)]
pub struct BackfillProgress {
    pub prs_processed: u32,
    pub prs_total: u32,
    pub reviews_processed: u32,
    pub users_created: u32,
    pub current_pr: Option<i32>,
}

/// Backfill historical data from GitHub
pub struct Backfiller {
    pool: PgPool,
    client: GitHubClient,
    score_calculator: ScoreCalculator,
    max_age_days: u32,
}

impl Backfiller {
    pub fn new(pool: PgPool, github_token: Option<String>, max_age_days: u32) -> Self {
        let client = GitHubClient::new(github_token);
        Self {
            pool: pool.clone(),
            client,
            score_calculator: ScoreCalculator::new(pool),
            max_age_days,
        }
    }

    /// Backfill a repository, fetching PRs updated since last sync (or max_age_days if first run)
    /// If `force` is true, ignores last_synced_at and fetches full history up to max_age_days
    pub async fn backfill_repo(
        &self,
        owner: &str,
        name: &str,
        force: bool,
    ) -> Result<BackfillProgress, BackfillError> {
        info!(
            "Starting backfill for {}/{} (force: {})",
            owner, name, force
        );

        // Get or create the repository
        let gh_repo = self.client.get_repo(owner, name).await?;
        let repo = db::repos::upsert(&self.pool, gh_repo.id, owner, name).await?;

        // Get last sync time (ignore if force=true)
        let last_synced = if force {
            info!("Force mode: ignoring last_synced_at");
            None
        } else {
            db::repos::get_last_synced_at(&self.pool, repo.id).await?
        };
        let sync_start = Utc::now();

        info!(
            "Last sync: {:?}, fetching PRs updated since then (max {} days)",
            last_synced, self.max_age_days
        );

        // Fetch PRs
        let prs = self
            .client
            .fetch_prs_since(owner, name, last_synced, self.max_age_days)
            .await?;

        let mut progress = BackfillProgress {
            prs_processed: 0,
            prs_total: prs.len() as u32,
            reviews_processed: 0,
            users_created: 0,
            current_pr: None,
        };

        info!("Processing {} PRs", prs.len());

        for pr in prs {
            progress.current_pr = Some(pr.number);
            match self.process_pr(&repo.id, owner, name, &pr).await {
                Ok((reviews_count, new_users)) => {
                    progress.reviews_processed += reviews_count;
                    progress.users_created += new_users;
                }
                Err(BackfillError::RateLimited(retry_after)) => {
                    warn!(
                        "Rate limited, stopping backfill. Retry after {} seconds",
                        retry_after
                    );
                    // Save progress before stopping
                    db::repos::set_last_synced_at(&self.pool, repo.id, sync_start).await?;
                    return Err(BackfillError::RateLimited(retry_after));
                }
                Err(e) => {
                    warn!("Error processing PR #{}: {}", pr.number, e);
                    // Continue with other PRs
                }
            }
            progress.prs_processed += 1;

            // Log progress every 10 PRs
            if progress.prs_processed.is_multiple_of(10) {
                info!(
                    "Progress: {}/{} PRs, {} reviews",
                    progress.prs_processed, progress.prs_total, progress.reviews_processed
                );
            }
        }

        // Update last sync time
        db::repos::set_last_synced_at(&self.pool, repo.id, sync_start).await?;

        info!(
            "Backfill complete: {} PRs, {} reviews, {} new users",
            progress.prs_processed, progress.reviews_processed, progress.users_created
        );

        Ok(progress)
    }

    async fn process_pr(
        &self,
        repo_id: &uuid::Uuid,
        owner: &str,
        repo_name: &str,
        pr: &GithubPr,
    ) -> Result<(u32, u32), BackfillError> {
        debug!("Processing PR #{}: {}", pr.number, pr.title);

        let mut new_users = 0u32;

        // Upsert author
        let (author, created) = db::users::upsert_returning_created(
            &self.pool,
            pr.user.id,
            &pr.user.login,
            pr.user.avatar_url.as_deref(),
        )
        .await?;
        if created {
            new_users += 1;
        }

        // Determine PR state
        let state = if pr.merged_at.is_some() {
            PrState::Merged
        } else if pr.state == "closed" {
            PrState::Closed
        } else {
            PrState::Open
        };

        // Upsert PR
        let db_pr = db::prs::upsert(
            &self.pool,
            *repo_id,
            pr.id,
            pr.number,
            &pr.title,
            author.id,
            state,
            pr.created_at,
        )
        .await?;

        // Update merged_at/closed_at if applicable
        if pr.merged_at.is_some() || pr.closed_at.is_some() {
            db::prs::update_timestamps(&self.pool, db_pr.id, pr.merged_at, pr.closed_at).await?;
        }

        // Fetch reviews
        let reviews = match self.client.list_reviews(owner, repo_name, pr.number).await {
            Ok(r) => r,
            Err(github::client::ClientError::RateLimited { retry_after }) => {
                return Err(BackfillError::RateLimited(retry_after));
            }
            Err(e) => {
                warn!("Failed to fetch reviews for PR #{}: {}", pr.number, e);
                return Ok((0, new_users));
            }
        };

        // Fetch review comments to count per review
        let comments = match self
            .client
            .list_review_comments(owner, repo_name, pr.number)
            .await
        {
            Ok(c) => c,
            Err(github::client::ClientError::RateLimited { retry_after }) => {
                return Err(BackfillError::RateLimited(retry_after));
            }
            Err(e) => {
                debug!("Failed to fetch comments for PR #{}: {}", pr.number, e);
                Vec::new()
            }
        };

        // Count comments per review ID
        let mut comment_counts: std::collections::HashMap<i64, i32> =
            std::collections::HashMap::new();
        for comment in &comments {
            if let Some(review_id) = comment.pull_request_review_id {
                *comment_counts.entry(review_id).or_insert(0) += 1;
            }
        }

        let mut reviews_count = 0u32;
        let mut first_review_at = None;

        for review in reviews {
            // Skip reviews without a user (ghost accounts)
            let Some(ref user) = review.user else {
                continue;
            };

            // Skip pending reviews
            let Some(submitted_at) = review.submitted_at else {
                continue;
            };

            // Upsert reviewer
            let (reviewer, created) = db::users::upsert_returning_created(
                &self.pool,
                user.id,
                &user.login,
                user.avatar_url.as_deref(),
            )
            .await?;
            if created {
                new_users += 1;
            }

            // Parse review state
            let review_state = match review.state.to_lowercase().as_str() {
                "approved" => ReviewState::Approved,
                "changes_requested" => ReviewState::ChangesRequested,
                "commented" => ReviewState::Commented,
                "dismissed" => ReviewState::Dismissed,
                _ => ReviewState::Pending,
            };

            // Get comment count for this review
            let comments_count = comment_counts.get(&review.id).copied().unwrap_or(0);

            // Insert review (ignore if already exists)
            match db::reviews::insert(
                &self.pool,
                db_pr.id,
                reviewer.id,
                review.id,
                review_state,
                review.body.as_deref(),
                comments_count,
                submitted_at,
            )
            .await
            {
                Ok(_) => {
                    reviews_count += 1;

                    // Track first review (by submitted_at)
                    if first_review_at.is_none() || submitted_at < first_review_at.unwrap() {
                        first_review_at = Some(submitted_at);
                    }

                    // Award XP
                    let xp = self.score_calculator.calculate_review_xp(
                        &db_pr,
                        submitted_at,
                        comments_count,
                    );
                    let _ = db::users::add_xp(&self.pool, reviewer.id, xp).await;
                }
                Err(e) => {
                    // Likely duplicate, ignore
                    debug!("Review insert error (probably duplicate): {}", e);
                }
            }
        }

        // Set first review time if we found reviews
        if let Some(first_at) = first_review_at {
            if db_pr.first_review_at.is_none() {
                let _ = db::prs::set_first_review(&self.pool, db_pr.id, first_at).await;
            }
        }

        Ok((reviews_count, new_users))
    }
}
