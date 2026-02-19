//! Repository routes

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

use crate::error::{ApiResult, DbResultExt, OptionExt};
use crate::state::AppState;
use common::models::Repository;

/// Allowed orgs for auto-discovery
const ALLOWED_ORGS: &[&str] = &["sigp", "ethereum", "chainsafe", "offchainlabs"];

pub async fn list(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Repository>>> {
    let repos = db::repos::list(&state.pool).await.db_err()?;
    Ok(Json(repos))
}

/// Extended repo response with sync status
#[derive(Serialize)]
pub struct RepoWithSyncStatus {
    #[serde(flatten)]
    pub repo: Repository,
    pub sync_status: SyncStatus,
}

#[derive(Serialize)]
pub struct SyncStatus {
    pub syncing: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub oldest_data_at: Option<DateTime<Utc>>,
    pub target_date: DateTime<Utc>,
    pub progress_pct: f64,
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
) -> ApiResult<Json<RepoWithSyncStatus>> {
    // Check if repo exists
    let repo = db::repos::get_by_name(&state.pool, &owner, &name)
        .await
        .db_err()?;

    match repo {
        Some(repo) => {
            // Repo exists, return with sync status
            let sync_status = get_sync_status(&state, &repo).await;
            Ok(Json(RepoWithSyncStatus { repo, sync_status }))
        }
        None => {
            // Check if org is allowed for auto-discovery
            let owner_lc = owner.to_lowercase();
            if !ALLOWED_ORGS.contains(&owner_lc.as_str()) {
                return Err(crate::error::ApiError::NotFound(format!(
                    "Repository {}/{} not found",
                    owner, name
                )));
            }

            // Auto-create and start syncing
            info!("Auto-discovering repo {}/{}", owner, name);

            // Try to create the repo (will fail if GitHub repo doesn't exist)
            let github = github::GitHubClient::new(state.config.github_token.clone());
            let gh_repo = github
                .get_repo(&owner, &name)
                .await
                .map_err(|e| crate::error::ApiError::GitHub(e.to_string()))?;

            // Create repo in DB
            let repo = db::repos::create(&state.pool, gh_repo.id, &owner, &name)
                .await
                .db_err()?;

            // Spawn background sync
            let pool = state.pool.clone();
            let token = state.config.github_token.clone();
            let owner_clone = owner.clone();
            let name_clone = name.clone();
            tokio::spawn(async move {
                info!(
                    "Starting background sync for {}/{}",
                    owner_clone, name_clone
                );
                let backfiller = processor::Backfiller::new(pool, token, 365);
                if let Err(e) = backfiller.backfill_repo(&owner_clone, &name_clone).await {
                    tracing::error!(
                        "Background sync failed for {}/{}: {}",
                        owner_clone,
                        name_clone,
                        e
                    );
                }
            });

            let sync_status = SyncStatus {
                syncing: true,
                last_synced_at: None,
                oldest_data_at: None,
                target_date: Utc::now() - chrono::Duration::days(365),
                progress_pct: 0.0,
            };

            Ok(Json(RepoWithSyncStatus { repo, sync_status }))
        }
    }
}

async fn get_sync_status(state: &AppState, repo: &Repository) -> SyncStatus {
    let last_synced = db::repos::get_last_synced_at(&state.pool, repo.id)
        .await
        .ok()
        .flatten();

    let oldest_data = db::repos::get_oldest_pr_date(&state.pool, repo.id)
        .await
        .ok()
        .flatten();

    let target_date = Utc::now() - chrono::Duration::days(365);

    // Calculate progress: how much of the 365-day window we have data for
    let progress_pct = match oldest_data {
        Some(oldest) => {
            let total_days = 365.0;
            let days_covered = (Utc::now() - oldest).num_days() as f64;
            (days_covered / total_days * 100.0).min(100.0)
        }
        None => 0.0,
    };

    // Consider syncing if last sync was recent (within 5 min) and progress < 100%
    let syncing = last_synced
        .map(|t| (Utc::now() - t).num_minutes() < 5 && progress_pct < 100.0)
        .unwrap_or(false);

    SyncStatus {
        syncing,
        last_synced_at: last_synced,
        oldest_data_at: oldest_data,
        target_date,
        progress_pct,
    }
}

/// Open PR response
#[derive(Serialize)]
pub struct OpenPrResponse {
    pub number: i32,
    pub title: String,
    pub author: String,
    pub author_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub age_hours: i64,
    pub first_review_at: Option<DateTime<Utc>>,
    pub hours_to_first_review: Option<i64>,
    pub review_count: i32,
    pub approvals: i32,
    pub changes_requested: i32,
    pub comments_count: i32,
    pub status: String, // "approved", "changes_requested", "reviewed", "needs_review"
    pub reviewers: Vec<String>,
    pub url: String,
}

/// Open PRs summary
#[derive(Serialize)]
pub struct OpenPrsSummary {
    pub total: i64,
    pub needs_review: i64,
    pub approved: i64,
    pub changes_requested: i64,
    pub prs: Vec<OpenPrResponse>,
}

/// Get open PRs for a repository
pub async fn open_prs(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
) -> ApiResult<Json<OpenPrsSummary>> {
    let repo = db::repos::get_by_name(&state.pool, &owner, &name)
        .await
        .db_err()?
        .not_found(format!("Repository {}/{} not found", owner, name))?;

    let prs = db::prs::list_open_with_stats(&state.pool, repo.id)
        .await
        .db_err()?;

    let now = Utc::now();
    let mut needs_review = 0i64;
    let mut approved = 0i64;
    let mut changes_requested = 0i64;

    let prs: Vec<OpenPrResponse> = prs
        .into_iter()
        .map(|pr| {
            let age_hours = (now - pr.created_at).num_hours();
            let hours_to_first_review = pr.first_review_at.map(|t| (t - pr.created_at).num_hours());

            let status = if pr.approvals > 0 && pr.changes_requested == 0 {
                approved += 1;
                "approved"
            } else if pr.changes_requested > 0 {
                changes_requested += 1;
                "changes_requested"
            } else if pr.review_count > 0 {
                "reviewed"
            } else {
                needs_review += 1;
                "needs_review"
            };

            OpenPrResponse {
                number: pr.number,
                title: pr.title,
                author: pr.author_login,
                author_avatar: pr.author_avatar,
                created_at: pr.created_at,
                age_hours,
                first_review_at: pr.first_review_at,
                hours_to_first_review,
                review_count: pr.review_count,
                approvals: pr.approvals,
                changes_requested: pr.changes_requested,
                comments_count: pr.comments_count,
                status: status.to_string(),
                reviewers: pr.reviewers,
                url: format!("https://github.com/{}/{}/pull/{}", owner, name, pr.number),
            }
        })
        .collect();

    Ok(Json(OpenPrsSummary {
        total: prs.len() as i64,
        needs_review,
        approved,
        changes_requested,
        prs,
    }))
}
