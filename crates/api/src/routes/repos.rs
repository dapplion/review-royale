//! Repository routes

use axum::{
    extract::{Path, State},
    Json,
};
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
