//! Repository routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::state::AppState;
use common::models::Repository;

pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Repository>>, StatusCode> {
    let repos = db::repos::list(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(repos))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path((owner, name)): Path<(String, String)>,
) -> Result<Json<Repository>, StatusCode> {
    let repo = db::repos::get_by_name(&state.pool, &owner, &name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(repo))
}
