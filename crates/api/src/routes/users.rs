//! User routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use common::models::{User, UserAchievement, UserStats};

/// Path parameters for repo-scoped user endpoints
#[derive(Deserialize)]
pub struct RepoUserPath {
    pub owner: String,
    pub name: String,
    pub username: String,
}

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

#[derive(Serialize)]
pub struct ReviewItem {
    pub state: String,
    pub comments_count: i32,
    pub submitted_at: String,
    pub pr_number: i32,
    pub pr_title: String,
    pub pr_state: String,
    pub repo_owner: String,
    pub repo_name: String,
}

#[derive(Deserialize)]
pub struct ReviewsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    10
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

pub async fn reviews(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
    Query(query): Query<ReviewsQuery>,
) -> Result<Json<Vec<ReviewItem>>, StatusCode> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let limit = query.limit.clamp(1, 50); // Cap at 50, min 1
    let reviews = db::users::get_recent_reviews(&state.pool, user.id, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<ReviewItem> = reviews
        .into_iter()
        .map(|r| ReviewItem {
            state: r.state,
            comments_count: r.comments_count,
            submitted_at: r.submitted_at.to_rfc3339(),
            pr_number: r.pr_number,
            pr_title: r.pr_title,
            pr_state: r.pr_state,
            repo_owner: r.repo_owner,
            repo_name: r.repo_name,
        })
        .collect();

    Ok(Json(result))
}

/// User profile scoped to a specific repository
pub async fn repo_stats(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoUserPath>,
) -> Result<Json<UserProfile>, StatusCode> {
    // Get repo
    let repo = db::repos::get_by_name(&state.pool, &path.owner, &path.name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get user
    let user = db::users::get_by_login(&state.pool, &path.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get achievements (global, not repo-scoped)
    let achievements = db::achievements::list_for_user(&state.pool, user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get rank within this repo
    let since = Utc::now() - Duration::days(365 * 10);
    let rank = db::leaderboard::get_user_rank(&state.pool, user.id, Some(repo.id), since)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get stats scoped to this repo
    let stats = db::users::get_stats_for_repo(&state.pool, user.id, Some(repo.id), since)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(UserProfile {
        user,
        stats,
        achievements,
        rank,
    }))
}

/// User activity scoped to a specific repository
pub async fn repo_activity(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoUserPath>,
) -> Result<Json<Vec<WeeklyActivity>>, StatusCode> {
    // Get repo
    let repo = db::repos::get_by_name(&state.pool, &path.owner, &path.name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get user
    let user = db::users::get_by_login(&state.pool, &path.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let activity = db::users::get_weekly_activity_for_repo(&state.pool, user.id, Some(repo.id), 12)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<WeeklyActivity> = activity
        .into_iter()
        .map(|(week, reviews, xp)| WeeklyActivity { week, reviews, xp })
        .collect();

    Ok(Json(result))
}

/// User reviews scoped to a specific repository
pub async fn repo_reviews(
    State(state): State<Arc<AppState>>,
    Path(path): Path<RepoUserPath>,
    Query(query): Query<ReviewsQuery>,
) -> Result<Json<Vec<ReviewItem>>, StatusCode> {
    // Get repo
    let repo = db::repos::get_by_name(&state.pool, &path.owner, &path.name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get user
    let user = db::users::get_by_login(&state.pool, &path.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let limit = query.limit.clamp(1, 50);
    let reviews =
        db::users::get_recent_reviews_for_repo(&state.pool, user.id, Some(repo.id), limit)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        reviews
            .into_iter()
            .map(|r| ReviewItem {
                state: r.state,
                comments_count: r.comments_count,
                submitted_at: r.submitted_at.to_rfc3339(),
                pr_number: r.pr_number,
                pr_title: r.pr_title,
                pr_state: r.pr_state,
                repo_owner: r.repo_owner,
                repo_name: r.repo_name,
            })
            .collect(),
    ))
}
