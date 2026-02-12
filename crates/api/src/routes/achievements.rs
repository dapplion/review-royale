//! Achievement routes

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::error::{ApiResult, DbResultExt, OptionExt};
use crate::state::AppState;
use db::achievements::{AchievementCategory, AchievementProgress, AchievementWithStats};

/// Grouped achievements for the catalog
#[derive(Serialize)]
pub struct AchievementCatalog {
    pub total: usize,
    pub categories: Vec<CategoryGroup>,
}

#[derive(Serialize)]
pub struct CategoryGroup {
    pub name: String,
    pub achievements: Vec<AchievementWithStats>,
}

/// List all achievements (catalog)
pub async fn list(State(state): State<Arc<AppState>>) -> ApiResult<Json<AchievementCatalog>> {
    let achievements = db::achievements::list_all_with_stats(&state.pool)
        .await
        .db_err()?;

    let total = achievements.len();

    // Group by category
    let mut milestone = Vec::new();
    let mut speed = Vec::new();
    let mut quality = Vec::new();
    let mut streak = Vec::new();
    let mut special = Vec::new();

    for a in achievements {
        match a.category {
            AchievementCategory::Milestone => milestone.push(a),
            AchievementCategory::Speed => speed.push(a),
            AchievementCategory::Quality => quality.push(a),
            AchievementCategory::Streak => streak.push(a),
            AchievementCategory::Special => special.push(a),
        }
    }

    let categories = vec![
        CategoryGroup {
            name: "Milestone".to_string(),
            achievements: milestone,
        },
        CategoryGroup {
            name: "Speed".to_string(),
            achievements: speed,
        },
        CategoryGroup {
            name: "Quality".to_string(),
            achievements: quality,
        },
        CategoryGroup {
            name: "Streak".to_string(),
            achievements: streak,
        },
        CategoryGroup {
            name: "Special".to_string(),
            achievements: special,
        },
    ]
    .into_iter()
    .filter(|c| !c.achievements.is_empty())
    .collect();

    Ok(Json(AchievementCatalog { total, categories }))
}

/// Get a user's progress toward all achievements
pub async fn user_progress(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> ApiResult<Json<Vec<AchievementProgress>>> {
    let user = db::users::get_by_login(&state.pool, &username)
        .await
        .db_err()?
        .not_found(format!("User {} not found", username))?;

    let progress = db::achievements::get_user_progress(&state.pool, user.id)
        .await
        .db_err()?;

    Ok(Json(progress))
}
