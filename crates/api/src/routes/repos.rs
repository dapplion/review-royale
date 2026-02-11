//! Repository routes

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;

use crate::error::{ApiResult, DbResultExt, OptionExt};
use crate::state::AppState;
use common::models::Repository;

pub async fn list(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Repository>>> {
    let repos = db::repos::list(&state.pool).await.db_err()?;
    Ok(Json(repos))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
) -> ApiResult<Json<Repository>> {
    let repo = db::repos::get_by_name(&state.pool, &owner, &name)
        .await
        .db_err()?
        .not_found(format!("Repository {}/{} not found", owner, name))?;

    Ok(Json(repo))
}

/// Open PR response
#[derive(Serialize)]
pub struct OpenPrResponse {
    pub number: i32,
    pub title: String,
    pub author: String,
    pub author_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub age_hours: i64,
    pub first_review_at: Option<DateTime<Utc>>,
    pub hours_to_first_review: Option<i64>,
    pub review_count: i32,
    pub approvals: i32,
    pub changes_requested: i32,
    pub comments_count: i32,
    pub status: String, // "approved", "changes_requested", "reviewed", "needs_review"
    pub reviewers: Vec<String>,
    pub url: String,
}

/// Open PRs summary
#[derive(Serialize)]
pub struct OpenPrsSummary {
    pub total: i64,
    pub needs_review: i64,
    pub approved: i64,
    pub changes_requested: i64,
    pub prs: Vec<OpenPrResponse>,
}

/// Get open PRs for a repository
pub async fn open_prs(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
) -> ApiResult<Json<OpenPrsSummary>> {
    let repo = db::repos::get_by_name(&state.pool, &owner, &name)
        .await
        .db_err()?
        .not_found(format!("Repository {}/{} not found", owner, name))?;

    let prs = db::prs::list_open_with_stats(&state.pool, repo.id)
        .await
        .db_err()?;

    let now = Utc::now();
    let mut needs_review = 0i64;
    let mut approved = 0i64;
    let mut changes_requested = 0i64;

    let prs: Vec<OpenPrResponse> = prs
        .into_iter()
        .map(|pr| {
            let age_hours = (now - pr.created_at).num_hours();
            let hours_to_first_review = pr.first_review_at.map(|t| (t - pr.created_at).num_hours());
            
            let status = if pr.approvals > 0 && pr.changes_requested == 0 {
                approved += 1;
                "approved"
            } else if pr.changes_requested > 0 {
                changes_requested += 1;
                "changes_requested"
            } else if pr.review_count > 0 {
                "reviewed"
            } else {
                needs_review += 1;
                "needs_review"
            };

            OpenPrResponse {
                number: pr.number,
                title: pr.title,
                author: pr.author_login,
                author_avatar: pr.author_avatar,
                created_at: pr.created_at,
                age_hours,
                first_review_at: pr.first_review_at,
                hours_to_first_review,
                review_count: pr.review_count,
                approvals: pr.approvals,
                changes_requested: pr.changes_requested,
                comments_count: pr.comments_count,
                status: status.to_string(),
                reviewers: pr.reviewers,
                url: format!("https://github.com/{}/{}/pull/{}", owner, name, pr.number),
            }
        })
        .collect();

    Ok(Json(OpenPrsSummary {
        total: prs.len() as i64,
        needs_review,
        approved,
        changes_requested,
        prs,
    }))
}
