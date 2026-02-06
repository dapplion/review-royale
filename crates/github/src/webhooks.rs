//! Webhook payload parsing

use crate::events::*;
use serde_json::Value;
use tracing::{debug, warn};

/// Parsed webhook payload
#[derive(Debug)]
pub enum WebhookPayload {
    PullRequest(PullRequestEvent),
    PullRequestReview(PullRequestReviewEvent),
    PullRequestReviewComment(PullRequestReviewCommentEvent),
    IssueComment(IssueCommentEvent),
    CheckRun(CheckRunEvent),
    Ping { zen: String },
    Unknown { event_type: String },
}

impl WebhookPayload {
    /// Parse a webhook payload from the event type and body
    pub fn parse(event_type: &str, body: &[u8]) -> Result<Self, serde_json::Error> {
        debug!("Parsing webhook: {}", event_type);

        match event_type {
            "ping" => {
                let v: Value = serde_json::from_slice(body)?;
                let zen = v["zen"].as_str().unwrap_or("").to_string();
                Ok(WebhookPayload::Ping { zen })
            }
            "pull_request" => {
                let event: PullRequestEvent = serde_json::from_slice(body)?;
                Ok(WebhookPayload::PullRequest(event))
            }
            "pull_request_review" => {
                let event: PullRequestReviewEvent = serde_json::from_slice(body)?;
                Ok(WebhookPayload::PullRequestReview(event))
            }
            "pull_request_review_comment" => {
                let event: PullRequestReviewCommentEvent = serde_json::from_slice(body)?;
                Ok(WebhookPayload::PullRequestReviewComment(event))
            }
            "issue_comment" => {
                let event: IssueCommentEvent = serde_json::from_slice(body)?;
                Ok(WebhookPayload::IssueComment(event))
            }
            "check_run" => {
                let event: CheckRunEvent = serde_json::from_slice(body)?;
                Ok(WebhookPayload::CheckRun(event))
            }
            _ => {
                warn!("Unknown webhook event type: {}", event_type);
                Ok(WebhookPayload::Unknown {
                    event_type: event_type.to_string(),
                })
            }
        }
    }
}
