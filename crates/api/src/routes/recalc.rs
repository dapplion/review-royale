//! XP recalculation routes

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

use crate::state::AppState;

#[derive(Serialize)]
pub struct RecalcResponse {
    pub status: String,
    pub total_reviews: usize,
    pub total_sessions: usize,
    pub total_xp_awarded: i64,
    pub users_updated: usize,
}

pub async fn trigger(State(state): State<Arc<AppState>>) -> Result<Json<RecalcResponse>, StatusCode> {
    info!("Recalculation triggered via API");

    let stats = processor::recalculate_all_xp(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Recalculation failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(RecalcResponse {
        status: "complete".to_string(),
        total_reviews: stats.total_reviews,
        total_sessions: stats.total_sessions,
        total_xp_awarded: stats.total_xp_awarded,
        users_updated: stats.users_updated,
    }))
}
