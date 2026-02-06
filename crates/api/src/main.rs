//! Review Royale API Server

use axum::{routing::get, Router};
use processor::{SyncConfig, SyncService};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

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

    // Build router
    let app = Router::new()
        .route("/", get(routes::health::root))
        .route("/health", get(routes::health::health))
        .route("/api/repos", get(routes::repos::list))
        .route("/api/repos/:owner/:name", get(routes::repos::get))
        .route(
            "/api/repos/:owner/:name/leaderboard",
            get(routes::leaderboard::get),
        )
        .route("/api/users/:username", get(routes::users::get))
        .route("/api/users/:username/stats", get(routes::users::stats))
        .route("/api/leaderboard", get(routes::leaderboard::global))
        .route(
            "/api/backfill/:owner/:name",
            get(routes::backfill::status).post(routes::backfill::trigger),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    info!("🚀 Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
