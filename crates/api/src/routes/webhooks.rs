//! Webhook routes

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::state::AppState;
use github::{verify_signature, WebhookPayload};

#[derive(Serialize)]
pub struct WebhookResponse {
    ok: bool,
    message: Option<String>,
}

pub async fn github(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, StatusCode> {
    // Get event type
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing X-GitHub-Event header");
            StatusCode::BAD_REQUEST
        })?;

    // Verify signature if configured
    if let Some(secret) = &state.config.github_webhook_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                warn!("Missing X-Hub-Signature-256 header");
                StatusCode::UNAUTHORIZED
            })?;

        if !verify_signature(signature, secret, &body) {
            warn!("Invalid webhook signature");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Parse the payload
    let payload = WebhookPayload::parse(event_type, &body).map_err(|e| {
        error!("Failed to parse webhook: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // Handle the event
    if let Err(e) = state.event_handler.handle(payload).await {
        error!("Failed to handle webhook: {}", e);
        return Ok(Json(WebhookResponse {
            ok: false,
            message: Some(e.to_string()),
        }));
    }

    info!("Successfully processed {} event", event_type);
    
    Ok(Json(WebhookResponse {
        ok: true,
        message: None,
    }))
}
