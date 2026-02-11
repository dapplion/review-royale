//! User routes

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{ApiResult, DbResultExt, OptionExt};
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

#[derive(Deserialize)]
pub struct StatsQuery {
    #[serde(default = "default_period")]
    pub period: String,
}

fn default_period() -> String {
    "all".to_string()
}

fn default_limit() -> i64 {
    10
}

fn period_to_since(period: &str) -> chrono::DateTime<Utc> {
    match period {
        "week" => Utc::now() - Duration::days(7),
        "month" => Utc::now() - Duration::days(30),
        "all" => Utc::now() - Duration::days(365 * 10),
        _ => Utc::now() - Duration::days(365 * 10),
    }
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> ApiResult<Json<User>> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", username))?;

    Ok(Json(user))
}

pub async fn stats(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
    Query(query): Query<StatsQuery>,
) -> ApiResult<Json<UserProfile>> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", username))?;

    // Get achievements
    let achievements = db::achievements::list_for_user(&state.pool, user.id)
        .await
        .db_err()?;

    // Get period-specific stats
    let since = period_to_since(&query.period);
    let rank = db::leaderboard::get_user_rank(&state.pool, user.id, None, since)
        .await
        .db_err()?;

    // Get full user stats for the period
    let stats = db::users::get_stats(&state.pool, user.id, since)
        .await
        .db_err()?;

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
) -> ApiResult<Json<Vec<WeeklyActivity>>> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", username))?;

    let activity = db::users::get_weekly_activity(&state.pool, user.id, 12)
        .await
        .db_err()?;

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
) -> ApiResult<Json<Vec<ReviewItem>>> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", username))?;

    let limit = query.limit.clamp(1, 50); // Cap at 50, min 1
    let reviews = db::users::get_recent_reviews(&state.pool, user.id, limit)
        .await
        .db_err()?;

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
) -> ApiResult<Json<UserProfile>> {
    // Get repo
    let repo = db::repos::get_by_name(&state.pool, &path.owner, &path.name)
        .await
        .db_err()?
        .not_found(format!("Repository {}/{} not found", path.owner, path.name))?;

    // Get user
    let user = db::users::get_by_login(&state.pool, &path.username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", path.username))?;

    // Get achievements (global, not repo-scoped)
    let achievements = db::achievements::list_for_user(&state.pool, user.id)
        .await
        .db_err()?;

    // Get rank within this repo
    let since = Utc::now() - Duration::days(365 * 10);
    let rank = db::leaderboard::get_user_rank(&state.pool, user.id, Some(repo.id), since)
        .await
        .db_err()?;

    // Get stats scoped to this repo
    let stats = db::users::get_stats_for_repo(&state.pool, user.id, Some(repo.id), since)
        .await
        .db_err()?;

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
) -> ApiResult<Json<Vec<WeeklyActivity>>> {
    // Get repo
    let repo = db::repos::get_by_name(&state.pool, &path.owner, &path.name)
        .await
        .db_err()?
        .not_found(format!("Repository {}/{} not found", path.owner, path.name))?;

    // Get user
    let user = db::users::get_by_login(&state.pool, &path.username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", path.username))?;

    let activity = db::users::get_weekly_activity_for_repo(&state.pool, user.id, Some(repo.id), 12)
        .await
        .db_err()?;

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
) -> ApiResult<Json<Vec<ReviewItem>>> {
    // Get repo
    let repo = db::repos::get_by_name(&state.pool, &path.owner, &path.name)
        .await
        .db_err()?
        .not_found(format!("Repository {}/{} not found", path.owner, path.name))?;

    // Get user
    let user = db::users::get_by_login(&state.pool, &path.username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", path.username))?;

    let limit = query.limit.clamp(1, 50);
    let reviews =
        db::users::get_recent_reviews_for_repo(&state.pool, user.id, Some(repo.id), limit)
            .await
            .db_err()?;

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
