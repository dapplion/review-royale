//! GitHub event types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GitHub user (as appears in webhook payloads)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
}

/// GitHub repository (as appears in webhook payloads)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub owner: GitHubUser,
}

/// GitHub pull request (as appears in webhook payloads)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    pub id: i64,
    pub number: i32,
    pub title: String,
    pub state: String,
    pub user: GitHubUser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged: Option<bool>,
}

/// GitHub review (as appears in webhook payloads)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubReview {
    pub id: i64,
    pub user: GitHubUser,
    pub body: Option<String>,
    pub state: String,
    pub submitted_at: Option<DateTime<Utc>>,
}

/// GitHub comment (as appears in webhook payloads)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubComment {
    pub id: i64,
    pub user: GitHubUser,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Pull request event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub number: i32,
    pub pull_request: GitHubPullRequest,
    pub repository: GitHubRepo,
    pub sender: GitHubUser,
}

/// Pull request review event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestReviewEvent {
    pub action: String,
    pub review: GitHubReview,
    pub pull_request: GitHubPullRequest,
    pub repository: GitHubRepo,
    pub sender: GitHubUser,
}

/// Pull request review comment event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestReviewCommentEvent {
    pub action: String,
    pub comment: GitHubComment,
    pub pull_request: GitHubPullRequest,
    pub repository: GitHubRepo,
    pub sender: GitHubUser,
}

/// Issue comment event payload (also used for PR comments)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCommentEvent {
    pub action: String,
    pub issue: IssueOrPr,
    pub comment: GitHubComment,
    pub repository: GitHubRepo,
    pub sender: GitHubUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueOrPr {
    pub id: i64,
    pub number: i32,
    pub title: String,
    pub pull_request: Option<serde_json::Value>, // Present if it's a PR
}

impl IssueOrPr {
    pub fn is_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }
}

/// Check run event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRunEvent {
    pub action: String,
    pub check_run: CheckRun,
    pub repository: GitHubRepo,
    pub sender: GitHubUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRun {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
