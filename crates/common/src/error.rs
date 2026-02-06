//! Error types

use thiserror::Error;

/// Main error type for Review Royale
#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(String),

    #[error("GitHub API error: {0}")]
    GitHub(String),

    #[error("Invalid webhook signature")]
    InvalidSignature,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;
