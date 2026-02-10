//! API error handling
//!
//! Consistent JSON error responses across all endpoints.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

/// Structured JSON error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
}

/// API error type that converts to JSON responses
#[derive(Debug)]
pub enum ApiError {
    /// Resource not found
    NotFound(String),
    /// Database error
    Database(String),
    /// GitHub API rate limited
    RateLimited(u64),
    /// GitHub API error
    GitHub(String),
    /// Internal server error
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, response) = match self {
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: msg,
                    code: Some("not_found".to_string()),
                    retry_after_secs: None,
                },
            ),
            ApiError::Database(msg) => {
                error!("Database error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: "Database error".to_string(),
                        code: Some("database_error".to_string()),
                        retry_after_secs: None,
                    },
                )
            }
            ApiError::RateLimited(retry_after) => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorResponse {
                    error: "Rate limited by GitHub API".to_string(),
                    code: Some("rate_limited".to_string()),
                    retry_after_secs: Some(retry_after),
                },
            ),
            ApiError::GitHub(msg) => {
                error!("GitHub API error: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    ErrorResponse {
                        error: format!("GitHub API error: {}", msg),
                        code: Some("github_error".to_string()),
                        retry_after_secs: None,
                    },
                )
            }
            ApiError::Internal(msg) => {
                error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: "Internal server error".to_string(),
                        code: Some("internal_error".to_string()),
                        retry_after_secs: None,
                    },
                )
            }
        };

        (status, Json(response)).into_response()
    }
}

/// Result type for API handlers
pub type ApiResult<T> = Result<T, ApiError>;

/// Extension trait to convert sqlx errors to ApiError
pub trait DbResultExt<T> {
    fn db_err(self) -> Result<T, ApiError>;
}

impl<T, E: std::fmt::Display> DbResultExt<T> for Result<T, E> {
    fn db_err(self) -> Result<T, ApiError> {
        self.map_err(|e| ApiError::Database(e.to_string()))
    }
}

/// Extension trait to convert Option to NotFound
pub trait OptionExt<T> {
    fn not_found(self, resource: impl Into<String>) -> Result<T, ApiError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn not_found(self, resource: impl Into<String>) -> Result<T, ApiError> {
        self.ok_or_else(|| ApiError::NotFound(resource.into()))
    }
}
