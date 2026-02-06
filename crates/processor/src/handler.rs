//! Webhook event handler

use common::models::{PrState, ReviewState};
use github::{PullRequestEvent, PullRequestReviewEvent, WebhookPayload};
use sqlx::PgPool;
use tracing::{debug, info, warn};

use crate::achievements::AchievementChecker;
use crate::scores::ScoreCalculator;

/// Handles incoming webhook events
pub struct EventHandler {
    pool: PgPool,
    achievement_checker: AchievementChecker,
    score_calculator: ScoreCalculator,
}

impl EventHandler {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            achievement_checker: AchievementChecker::new(pool.clone()),
            score_calculator: ScoreCalculator::new(pool),
        }
    }

    /// Process a webhook payload
    pub async fn handle(&self, payload: WebhookPayload) -> Result<(), common::Error> {
        match payload {
            WebhookPayload::Ping { zen } => {
                info!("Received ping: {}", zen);
                Ok(())
            }
            WebhookPayload::PullRequest(event) => self.handle_pull_request(event).await,
            WebhookPayload::PullRequestReview(event) => self.handle_review(event).await,
            WebhookPayload::PullRequestReviewComment(_event) => {
                debug!("Received PR review comment event");
                // TODO: Track review comments
                Ok(())
            }
            WebhookPayload::IssueComment(_event) => {
                debug!("Received issue comment event");
                // TODO: Track PR comments
                Ok(())
            }
            WebhookPayload::CheckRun(_event) => {
                debug!("Received check run event");
                // TODO: Track CI status
                Ok(())
            }
            WebhookPayload::Unknown { event_type } => {
                warn!("Ignoring unknown event type: {}", event_type);
                Ok(())
            }
        }
    }

    async fn handle_pull_request(&self, event: PullRequestEvent) -> Result<(), common::Error> {
        info!(
            "PR #{} in {}: {} by {}",
            event.number, event.repository.full_name, event.action, event.pull_request.user.login
        );

        // Upsert repository
        let repo = db::repos::upsert(
            &self.pool,
            event.repository.id,
            &event.repository.owner.login,
            &event.repository.name,
        )
        .await
        .map_err(|e| common::Error::Database(e.to_string()))?;

        // Upsert author
        let author = db::users::upsert(
            &self.pool,
            event.pull_request.user.id,
            &event.pull_request.user.login,
            event.pull_request.user.avatar_url.as_deref(),
        )
        .await
        .map_err(|e| common::Error::Database(e.to_string()))?;

        // Determine PR state
        let state = if event.pull_request.merged == Some(true) {
            PrState::Merged
        } else if event.pull_request.state == "closed" {
            PrState::Closed
        } else {
            PrState::Open
        };

        // Upsert PR
        let _pr = db::prs::upsert(
            &self.pool,
            repo.id,
            event.pull_request.id,
            event.number,
            &event.pull_request.title,
            author.id,
            state,
            event.pull_request.created_at,
        )
        .await
        .map_err(|e| common::Error::Database(e.to_string()))?;

        // Check for achievements
        self.achievement_checker.check_author(&author.id).await?;

        Ok(())
    }

    async fn handle_review(&self, event: PullRequestReviewEvent) -> Result<(), common::Error> {
        info!(
            "Review on PR #{} in {}: {} by {}",
            event.pull_request.number,
            event.repository.full_name,
            event.action,
            event.review.user.login
        );

        if event.action != "submitted" {
            return Ok(());
        }

        // Upsert repository
        let repo = db::repos::upsert(
            &self.pool,
            event.repository.id,
            &event.repository.owner.login,
            &event.repository.name,
        )
        .await
        .map_err(|e| common::Error::Database(e.to_string()))?;

        // Upsert reviewer
        let reviewer = db::users::upsert(
            &self.pool,
            event.review.user.id,
            &event.review.user.login,
            event.review.user.avatar_url.as_deref(),
        )
        .await
        .map_err(|e| common::Error::Database(e.to_string()))?;

        // Upsert author
        let author = db::users::upsert(
            &self.pool,
            event.pull_request.user.id,
            &event.pull_request.user.login,
            event.pull_request.user.avatar_url.as_deref(),
        )
        .await
        .map_err(|e| common::Error::Database(e.to_string()))?;

        // Get or create the PR
        let pr = db::prs::upsert(
            &self.pool,
            repo.id,
            event.pull_request.id,
            event.pull_request.number,
            &event.pull_request.title,
            author.id,
            PrState::Open,
            event.pull_request.created_at,
        )
        .await
        .map_err(|e| common::Error::Database(e.to_string()))?;

        // Parse review state
        let review_state = match event.review.state.to_lowercase().as_str() {
            "approved" => ReviewState::Approved,
            "changes_requested" => ReviewState::ChangesRequested,
            "commented" => ReviewState::Commented,
            "dismissed" => ReviewState::Dismissed,
            _ => ReviewState::Pending,
        };

        let submitted_at = event.review.submitted_at.unwrap_or_else(chrono::Utc::now);

        // Insert review
        let _review = db::reviews::insert(
            &self.pool,
            pr.id,
            reviewer.id,
            event.review.id,
            review_state,
            event.review.body.as_deref(),
            0, // TODO: Count comments
            submitted_at,
        )
        .await
        .map_err(|e| common::Error::Database(e.to_string()))?;

        // Record first review if this is it
        if pr.first_review_at.is_none() {
            db::prs::set_first_review(&self.pool, pr.id, submitted_at)
                .await
                .map_err(|e| common::Error::Database(e.to_string()))?;

            info!(
                "First review on PR #{} by {} (took {:?})",
                pr.number,
                reviewer.login,
                submitted_at - pr.created_at
            );
        }

        // Award XP for reviewing
        let xp = self.score_calculator.calculate_review_xp(&pr, submitted_at);
        db::users::add_xp(&self.pool, reviewer.id, xp)
            .await
            .map_err(|e| common::Error::Database(e.to_string()))?;

        info!("Awarded {} XP to {} for review", xp, reviewer.login);

        // Check for achievements
        self.achievement_checker
            .check_reviewer(&reviewer.id)
            .await?;

        Ok(())
    }
}
