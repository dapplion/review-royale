//! User routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use serde::Serialize;
use std::sync::Arc;

use crate::state::AppState;
use common::models::{User, UserAchievement, UserStats};

#[derive(Serialize)]
pub struct UserProfile {
    pub user: User,
    pub stats: UserStats,
    pub achievements: Vec<UserAchievement>,
    pub rank: Option<i32>,
}

#[derive(Serialize)]
pub struct WeeklyActivity {
    pub week: String,
    pub reviews: i32,
    pub xp: i64,
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Json<User>, StatusCode> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(user))
}

pub async fn stats(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Json<UserProfile>, StatusCode> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get achievements
    let achievements = db::achievements::list_for_user(&state.pool, user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get rank (all time for profile)
    let since = Utc::now() - Duration::days(365 * 10);
    let rank = db::leaderboard::get_user_rank(&state.pool, user.id, None, since)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get full user stats
    let stats = db::users::get_stats(&state.pool, user.id, since)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(UserProfile {
        user,
        stats,
        achievements,
        rank,
    }))
}

pub async fn activity(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Json<Vec<WeeklyActivity>>, StatusCode> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let activity = db::users::get_weekly_activity(&state.pool, user.id, 12)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<WeeklyActivity> = activity
        .into_iter()
        .map(|(week, reviews, xp)| WeeklyActivity { week, reviews, xp })
        .collect();

    Ok(Json(result))
}
