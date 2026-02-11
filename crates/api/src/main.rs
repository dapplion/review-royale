//! Review Royale API Server

use axum::{routing::get, Router};
use processor::{SyncConfig, SyncService};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info;

mod error;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("review_royale=debug".parse()?)
                .add_directive("api=debug".parse()?),
        )
        .init();

    info!("🎮 Starting Review Royale API");

    // Load configuration
    let config = common::Config::from_env();

    // Connect to database
    let pool = db::create_pool(&config.database_url).await?;

    // Run migrations
    db::run_migrations(&pool).await?;

    // Start background sync service (if enabled)
    if config.sync_interval_hours > 0 {
        let sync_config = SyncConfig {
            interval: Duration::from_secs(config.sync_interval_hours as u64 * 60 * 60),
            max_age_days: 365,
            github_token: config.github_token.clone(),
        };
        let sync_service = SyncService::new(pool.clone(), sync_config);
        tokio::spawn(async move {
            sync_service.run().await;
        });
        info!(
            "📡 Background sync enabled (every {} hours)",
            config.sync_interval_hours
        );
    } else {
        info!("📡 Background sync disabled (SYNC_INTERVAL_HOURS=0)");
    }

    // Create app state
    let state = Arc::new(AppState::new(config.clone(), pool));

    // Build API router with state
    let api_router = Router::new()
        .route("/health", get(routes::health::health))
        .route("/api/repos", get(routes::repos::list))
        .route("/api/repos/:owner/:name", get(routes::repos::get))
        .route(
            "/api/repos/:owner/:name/leaderboard",
            get(routes::leaderboard::get),
        )
        .route(
            "/api/repos/:owner/:name/users/:username/stats",
            get(routes::users::repo_stats),
        )
        .route(
            "/api/repos/:owner/:name/users/:username/activity",
            get(routes::users::repo_activity),
        )
        .route(
            "/api/repos/:owner/:name/users/:username/reviews",
            get(routes::users::repo_reviews),
        )
        .route("/api/users/:username", get(routes::users::get))
        .route("/api/users/:username/stats", get(routes::users::stats))
        .route(
            "/api/users/:username/activity",
            get(routes::users::activity),
        )
        .route("/api/users/:username/reviews", get(routes::users::reviews))
        .route("/api/leaderboard", get(routes::leaderboard::global))
        .route(
            "/api/backfill/:owner/:name",
            get(routes::backfill::status).post(routes::backfill::trigger),
        )
        .route(
            "/api/recalculate",
            axum::routing::post(routes::recalc::trigger),
        )
        .route(
            "/api/categorize",
            get(routes::categorize::stats).post(routes::categorize::trigger),
        )
        .route("/api/seasons", get(routes::seasons::list))
        .route("/api/seasons/current", get(routes::seasons::current))
        .route(
            "/api/seasons/:number/leaderboard",
            get(routes::seasons::leaderboard),
        )
        .route(
            "/api/seasons/ensure",
            axum::routing::post(routes::seasons::ensure_current),
        )
        // Team routes
        .route(
            "/api/teams",
            get(routes::teams::list).post(routes::teams::create),
        )
        .route("/api/teams/leaderboard", get(routes::teams::leaderboard))
        .route(
            "/api/teams/:name",
            get(routes::teams::get).delete(routes::teams::delete),
        )
        .route(
            "/api/teams/:name/members",
            axum::routing::post(routes::teams::add_member),
        )
        .route(
            "/api/teams/:name/members/:username",
            axum::routing::delete(routes::teams::remove_member),
        )
        .with_state(state);

    // Build full router with static file serving and SPA fallback
    // Serve static files, but fall back to index.html for SPA routing
    let static_service = ServeDir::new("static")
        .append_index_html_on_directories(true)
        .fallback(tower_http::services::ServeFile::new("static/index.html"));

    let app = api_router
        .fallback_service(static_service)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    info!("🚀 Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
