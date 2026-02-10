//! Leaderboard routes

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::error::{ApiResult, DbResultExt, OptionExt};
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
) -> ApiResult<Json<Vec<LeaderboardEntry>>> {
    let since = period_to_since(&query.period);

    let leaderboard = db::leaderboard::get_leaderboard(&state.pool, None, since, query.limit)
        .await
        .db_err()?;

    Ok(Json(leaderboard))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
    Query(query): Query<LeaderboardQuery>,
) -> ApiResult<Json<Vec<LeaderboardEntry>>> {
    let repo = db::repos::get_by_name(&state.pool, &owner, &name)
        .await
        .db_err()?
        .not_found(format!("Repository {}/{} not found", owner, name))?;

    let since = period_to_since(&query.period);

    let leaderboard =
        db::leaderboard::get_leaderboard(&state.pool, Some(repo.id), since, query.limit)
            .await
            .db_err()?;

    Ok(Json(leaderboard))
}
