//! Backfill endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::{ApiError, ApiResult, DbResultExt};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct BackfillParams {
    /// Maximum age in days to look back (default: 365)
    #[serde(default = "default_max_days")]
    pub max_days: u32,
    /// Force full backfill, ignoring last_synced_at
    #[serde(default)]
    pub force: bool,
}

fn default_max_days() -> u32 {
    365
}

#[derive(Debug, Serialize)]
pub struct BackfillResponse {
    pub success: bool,
    pub message: String,
    pub prs_processed: u32,
    pub reviews_processed: u32,
    pub users_created: u32,
}

#[derive(Debug, Serialize)]
pub struct BackfillStatus {
    pub repo: String,
    pub tracked: bool,
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Trigger a backfill for a repository
/// POST /api/backfill/:owner/:name
pub async fn trigger(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
    Query(params): Query<BackfillParams>,
) -> ApiResult<Json<BackfillResponse>> {
    info!(
        "Sync requested for {}/{} (max_days: {}, force: {})",
        owner, name, params.max_days, params.force
    );

    // If force=true, reset last_synced_at to trigger full backfill
    if params.force {
        if let Some(repo) = db::repos::get_by_name(&state.pool, &owner, &name)
            .await
            .ok()
            .flatten()
        {
            info!(
                "Force backfill: resetting last_synced_at for {}/{}",
                owner, name
            );
            let _ = db::repos::reset_last_synced_at(&state.pool, repo.id).await;
        }
    }

    let backfiller = processor::Backfiller::new(
        state.pool.clone(),
        state.config.github_token.clone(),
        params.max_days,
    );

    match backfiller.backfill_repo(&owner, &name).await {
        Ok(progress) => Ok(Json(BackfillResponse {
            success: true,
            message: format!("Backfill complete for {}/{}", owner, name),
            prs_processed: progress.prs_processed,
            reviews_processed: progress.reviews_processed,
            users_created: progress.users_created,
        })),
        Err(processor::backfill::BackfillError::RateLimited(retry_after)) => {
            Err(ApiError::RateLimited(retry_after))
        }
        Err(e) => Err(ApiError::GitHub(e.to_string())),
    }
}

/// Get backfill status for a repository
/// GET /api/backfill/:owner/:name
pub async fn status(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
) -> ApiResult<Json<BackfillStatus>> {
    // Check if repo exists and get last sync time
    let repo = db::repos::get_by_name(&state.pool, &owner, &name)
        .await
        .db_err()?;

    match repo {
        Some(repo) => {
            let last_synced = db::repos::get_last_synced_at(&state.pool, repo.id)
                .await
                .ok()
                .flatten();

            Ok(Json(BackfillStatus {
                repo: format!("{}/{}", owner, name),
                tracked: true,
                last_synced_at: last_synced,
            }))
        }
        None => Ok(Json(BackfillStatus {
            repo: format!("{}/{}", owner, name),
            tracked: false,
            last_synced_at: None,
        })),
    }
}
