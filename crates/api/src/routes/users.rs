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
use common::models::{User, UserStats, UserAchievement};

#[derive(Serialize)]
pub struct UserProfile {
    pub user: User,
    pub stats: UserStats,
    pub achievements: Vec<UserAchievement>,
    pub rank: Option<i32>,
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

    // Get rank
    let since = Utc::now() - Duration::days(30);
    let rank = db::leaderboard::get_user_rank(&state.pool, user.id, None, since)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get review count
    let reviews_given = db::reviews::count_by_user(&state.pool, user.id, since)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats = UserStats {
        reviews_given: reviews_given as i32,
        ..Default::default()
    };

    Ok(Json(UserProfile {
        user,
        stats,
        achievements,
        rank,
    }))
}
