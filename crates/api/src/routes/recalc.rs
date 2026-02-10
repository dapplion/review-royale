//! XP recalculation routes

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct RecalcResponse {
    pub status: String,
    pub total_reviews: usize,
    pub total_sessions: usize,
    pub total_xp_awarded: i64,
    pub users_updated: usize,
}

pub async fn trigger(State(state): State<Arc<AppState>>) -> ApiResult<Json<RecalcResponse>> {
    info!("Recalculation triggered via API");

    let stats = processor::recalculate_all_xp(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Recalculation failed: {}", e)))?;

    Ok(Json(RecalcResponse {
        status: "complete".to_string(),
        total_reviews: stats.total_reviews,
        total_sessions: stats.total_sessions,
        total_xp_awarded: stats.total_xp_awarded,
        users_updated: stats.users_updated,
    }))
}
