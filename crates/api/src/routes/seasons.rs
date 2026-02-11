//! Seasons API routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::models::Season;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{ApiError, ApiResult, DbResultExt};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LeaderboardParams {
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_limit() -> i32 {
    20
}

#[derive(Serialize)]
pub struct SeasonsResponse {
    pub seasons: Vec<Season>,
    pub current: Option<Season>,
}

/// List all seasons + current
pub async fn list(State(state): State<Arc<AppState>>) -> ApiResult<Json<SeasonsResponse>> {
    let seasons = db::seasons::get_all_seasons(&state.pool).await.db_err()?;
    let current = db::seasons::get_current_season(&state.pool)
        .await
        .db_err()?;

    Ok(Json(SeasonsResponse { seasons, current }))
}

/// Get current season
pub async fn current(State(state): State<Arc<AppState>>) -> ApiResult<Json<Option<Season>>> {
    let season = db::seasons::get_current_season(&state.pool)
        .await
        .db_err()?;
    Ok(Json(season))
}

/// Get season leaderboard by number
pub async fn leaderboard(
    State(state): State<Arc<AppState>>,
    Path(number): Path<i32>,
    Query(params): Query<LeaderboardParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let season = db::seasons::get_season_by_number(&state.pool, number)
        .await
        .db_err()?
        .ok_or_else(|| ApiError::NotFound(format!("Season {} not found", number)))?;

    let entries = db::seasons::get_season_leaderboard(&state.pool, season.id, None, params.limit)
        .await
        .db_err()?;

    Ok(Json(serde_json::json!({
        "season": season,
        "leaderboard": entries
    })))
}

/// Create/ensure current month's season exists
pub async fn ensure_current(
    State(state): State<Arc<AppState>>,
) -> ApiResult<(StatusCode, Json<Season>)> {
    let season = db::seasons::ensure_current_season(&state.pool)
        .await
        .db_err()?;
    Ok((StatusCode::OK, Json(season)))
}
