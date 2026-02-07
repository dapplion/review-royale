//! Background sync service

use crate::Backfiller;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

/// Configuration for the sync service
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Interval between sync runs
    pub interval: Duration,
    /// Maximum age for initial backfill (days)
    pub max_age_days: u32,
    /// GitHub token for API access
    pub github_token: Option<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(6 * 60 * 60), // 6 hours
            max_age_days: 365,
            github_token: None,
        }
    }
}

/// Background sync service that periodically updates all tracked repos
pub struct SyncService {
    pool: PgPool,
    config: SyncConfig,
}

impl SyncService {
    pub fn new(pool: PgPool, config: SyncConfig) -> Self {
        Self { pool, config }
    }

    /// Start the background sync loop
    pub async fn run(self) {
        info!(
            "Starting sync service (interval: {:?})",
            self.config.interval
        );

        let mut ticker = interval(self.config.interval);

        // Skip the first immediate tick - let the server start up first
        ticker.tick().await;

        loop {
            ticker.tick().await;
            info!("Starting scheduled sync of all tracked repos");

            if let Err(e) = self.sync_all().await {
                error!("Sync failed: {}", e);
            }
        }
    }

    /// Sync all tracked repositories
    async fn sync_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let repos = db::repos::list(&self.pool).await?;

        if repos.is_empty() {
            info!("No tracked repos to sync");
            return Ok(());
        }

        info!("Syncing {} tracked repos", repos.len());

        let backfiller = Backfiller::new(
            self.pool.clone(),
            self.config.github_token.clone(),
            self.config.max_age_days,
        );

        for repo in repos {
            info!("Syncing {}/{}", repo.owner, repo.name);

            match backfiller
                .backfill_repo(&repo.owner, &repo.name)
                .await
            {
                Ok(progress) => {
                    info!(
                        "Synced {}/{}: {} PRs, {} reviews",
                        repo.owner, repo.name, progress.prs_processed, progress.reviews_processed
                    );
                }
                Err(crate::BackfillError::RateLimited(retry_after)) => {
                    warn!(
                        "Rate limited while syncing {}/{}. Pausing for {} seconds",
                        repo.owner, repo.name, retry_after
                    );
                    tokio::time::sleep(Duration::from_secs(retry_after)).await;
                }
                Err(e) => {
                    error!("Failed to sync {}/{}: {}", repo.owner, repo.name, e);
                    // Continue with other repos
                }
            }

            // Small delay between repos to be nice to GitHub
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        info!("Sync complete");
        Ok(())
    }

    /// Run a single sync (for manual triggers)
    pub async fn sync_once(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.sync_all().await
    }
}
