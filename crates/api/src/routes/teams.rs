//! Team routes

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use common::models::{Team, TeamLeaderboardEntry};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiResult, DbResultExt, OptionExt};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub period: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct TeamWithMembers {
    #[serde(flatten)]
    pub team: Team,
    pub members: Vec<TeamMember>,
}

#[derive(Debug, Serialize)]
pub struct TeamMember {
    pub id: Uuid,
    pub login: String,
    pub xp: i64,
}

/// List all teams
pub async fn list(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Team>>> {
    let teams = db::teams::list_teams(&state.pool).await.db_err()?;
    Ok(Json(teams))
}

/// Get team by name
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<TeamWithMembers>> {
    let team = db::teams::get_team_by_name(&state.pool, &name)
        .await
        .db_err()?
        .not_found(format!("Team '{}' not found", name))?;

    let members = db::teams::get_team_members(&state.pool, team.id)
        .await
        .db_err()?;
    let members = members
        .into_iter()
        .map(|(id, login, xp)| TeamMember { id, login, xp })
        .collect();

    Ok(Json(TeamWithMembers { team, members }))
}

/// Create a new team
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTeamRequest>,
) -> ApiResult<(StatusCode, Json<Team>)> {
    let team = db::teams::create_team(
        &state.pool,
        &req.name,
        req.description.as_deref(),
        req.color.as_deref(),
    )
    .await
    .db_err()?;

    Ok((StatusCode::CREATED, Json(team)))
}

/// Delete a team
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let team = db::teams::get_team_by_name(&state.pool, &name)
        .await
        .db_err()?
        .not_found(format!("Team '{}' not found", name))?;

    db::teams::delete_team(&state.pool, team.id)
        .await
        .db_err()?;

    Ok(StatusCode::NO_CONTENT)
}

/// Add a member to a team
pub async fn add_member(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> ApiResult<StatusCode> {
    let team = db::teams::get_team_by_name(&state.pool, &name)
        .await
        .db_err()?
        .not_found(format!("Team '{}' not found", name))?;

    let user = db::users::get_by_login(&state.pool, &req.username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", req.username))?;

    db::teams::add_member(&state.pool, team.id, user.id)
        .await
        .db_err()?;

    Ok(StatusCode::CREATED)
}

/// Remove a member from a team
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path((name, username)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let team = db::teams::get_team_by_name(&state.pool, &name)
        .await
        .db_err()?
        .not_found(format!("Team '{}' not found", name))?;

    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .db_err()?
        .not_found(format!("User '{}' not found", username))?;

    db::teams::remove_member(&state.pool, team.id, user.id)
        .await
        .db_err()?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get team leaderboard
pub async fn leaderboard(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LeaderboardQuery>,
) -> ApiResult<Json<Vec<TeamLeaderboardEntry>>> {
    let since = match params.period.as_deref() {
        Some("week") => Utc::now() - Duration::days(7),
        Some("month") => Utc::now() - Duration::days(30),
        _ => Utc::now() - Duration::days(365 * 10), // "all" = 10 years
    };

    let limit = params.limit.unwrap_or(50);

    let leaderboard = db::teams::get_team_leaderboard(&state.pool, None, since, limit)
        .await
        .db_err()?;

    Ok(Json(leaderboard))
}
