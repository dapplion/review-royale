//! Leaderboard routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;
use common::models::LeaderboardEntry;

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    /// Time period: "week", "month", "all"
    #[serde(default = "default_period")]
    period: String,
    /// Limit
    #[serde(default = "default_limit")]
    limit: i32,
}

fn default_period() -> String {
    "month".to_string()
}

fn default_limit() -> i32 {
    25
}

fn period_to_since(period: &str) -> chrono::DateTime<Utc> {
    match period {
        "week" => Utc::now() - Duration::days(7),
        "month" => Utc::now() - Duration::days(30),
        "all" => Utc::now() - Duration::days(365 * 10),
        _ => Utc::now() - Duration::days(30),
    }
}

pub async fn global(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<Vec<LeaderboardEntry>>, StatusCode> {
    let since = period_to_since(&query.period);

    let leaderboard = db::leaderboard::get_leaderboard(&state.pool, None, since, query.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(leaderboard))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<Vec<LeaderboardEntry>>, StatusCode> {
    let repo = db::repos::get_by_name(&state.pool, &owner, &name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let since = period_to_since(&query.period);

    let leaderboard =
        db::leaderboard::get_leaderboard(&state.pool, Some(repo.id), since, query.limit)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(leaderboard))
}
