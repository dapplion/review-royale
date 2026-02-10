//! AI categorization routes

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CategorizeQuery {
    /// Maximum comments to process (default: 50)
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct CategorizeResponse {
    pub status: String,
    pub processed: usize,
    pub skipped: usize,
    pub errors: usize,
}

#[derive(Serialize)]
pub struct CategoryStatsResponse {
    pub total: usize,
    pub categorized: usize,
    pub uncategorized: usize,
    pub by_category: CategoryBreakdownResponse,
    pub avg_quality: f64,
}

#[derive(Serialize)]
pub struct CategoryBreakdownResponse {
    pub cosmetic: usize,
    pub logic: usize,
    pub structural: usize,
    pub nit: usize,
    pub question: usize,
}

/// Get categorization statistics
pub async fn stats(State(state): State<Arc<AppState>>) -> ApiResult<Json<CategoryStatsResponse>> {
    let stats = processor::get_category_stats(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get stats: {}", e)))?;

    Ok(Json(CategoryStatsResponse {
        total: stats.total,
        categorized: stats.categorized,
        uncategorized: stats.total - stats.categorized,
        by_category: CategoryBreakdownResponse {
            cosmetic: stats.by_category.cosmetic,
            logic: stats.by_category.logic,
            structural: stats.by_category.structural,
            nit: stats.by_category.nit,
            question: stats.by_category.question,
        },
        avg_quality: (stats.avg_quality * 10.0).round() / 10.0,
    }))
}

/// Trigger AI categorization for uncategorized comments
pub async fn trigger(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CategorizeQuery>,
) -> ApiResult<Json<CategorizeResponse>> {
    let api_key = state
        .config
        .openai_api_key
        .as_ref()
        .ok_or_else(|| ApiError::Internal("OPENAI_API_KEY not configured".to_string()))?;

    let batch_size = query.limit.unwrap_or(50).min(100);

    info!(
        "AI categorization triggered via API (batch_size={})",
        batch_size
    );

    let stats = processor::categorize_batch(&state.pool, api_key, batch_size)
        .await
        .map_err(|e| ApiError::Internal(format!("Categorization failed: {}", e)))?;

    Ok(Json(CategorizeResponse {
        status: "complete".to_string(),
        processed: stats.processed,
        skipped: stats.skipped,
        errors: stats.errors,
    }))
}
