//! Backfill endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct BackfillParams {
    /// Maximum age in days to look back (default: 365)
    #[serde(default = "default_max_days")]
    pub max_days: u32,
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
pub struct BackfillError {
    pub error: String,
    pub retry_after_secs: Option<u64>,
}

/// Trigger a backfill for a repository
/// POST /api/backfill/:owner/:name
pub async fn trigger(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
    Query(params): Query<BackfillParams>,
) -> impl IntoResponse {
    info!(
        "Backfill requested for {}/{} (max_days: {})",
        owner, name, params.max_days
    );

    let backfiller = processor::Backfiller::new(
        state.pool.clone(),
        state.config.github_token.clone(),
        params.max_days,
    );

    match backfiller.backfill_repo(&owner, &name).await {
        Ok(progress) => {
            let response = BackfillResponse {
                success: true,
                message: format!(
                    "Backfill complete for {}/{}",
                    owner, name
                ),
                prs_processed: progress.prs_processed,
                reviews_processed: progress.reviews_processed,
                users_created: progress.users_created,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(processor::backfill::BackfillError::RateLimited(retry_after)) => {
            let response = BackfillError {
                error: "Rate limited by GitHub API".to_string(),
                retry_after_secs: Some(retry_after),
            };
            (StatusCode::TOO_MANY_REQUESTS, Json(response)).into_response()
        }
        Err(e) => {
            error!("Backfill failed: {}", e);
            let response = BackfillError {
                error: e.to_string(),
                retry_after_secs: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Get backfill status for a repository
/// GET /api/backfill/:owner/:name
pub async fn status(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
) -> impl IntoResponse {
    // Check if repo exists and get last sync time
    match db::repos::get_by_name(&state.pool, &owner, &name).await {
        Ok(Some(repo)) => {
            let last_synced = db::repos::get_last_synced_at(&state.pool, repo.id)
                .await
                .ok()
                .flatten();

            Json(serde_json::json!({
                "repo": format!("{}/{}", owner, name),
                "tracked": true,
                "last_synced_at": last_synced,
            }))
            .into_response()
        }
        Ok(None) => Json(serde_json::json!({
            "repo": format!("{}/{}", owner, name),
            "tracked": false,
            "last_synced_at": Option::<String>::None,
        }))
        .into_response(),
        Err(e) => {
            error!("Failed to get repo status: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
