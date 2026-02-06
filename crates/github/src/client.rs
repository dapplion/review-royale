//! GitHub REST API client for fetching PRs and reviews

use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Rate limited, retry after {retry_after} seconds")]
    RateLimited { retry_after: u64 },
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("GitHub API error: {status} - {message}")]
    Api { status: u16, message: String },
}

/// GitHub API client
pub struct GitHubClient {
    client: reqwest::Client,
    token: Option<String>,
}

/// PR as returned by GitHub API
#[derive(Debug, Deserialize)]
pub struct GithubPr {
    pub id: i64,
    pub number: i32,
    pub title: String,
    pub state: String,
    pub user: GithubUser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

/// Review as returned by GitHub API
#[derive(Debug, Deserialize)]
pub struct GithubReview {
    pub id: i64,
    pub user: Option<GithubUser>,
    pub state: String,
    pub body: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
}

/// User as returned by GitHub API
#[derive(Debug, Deserialize)]
pub struct GithubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
}

/// Repository as returned by GitHub API
#[derive(Debug, Deserialize)]
pub struct GithubRepo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub owner: GithubUser,
}

/// Review comment as returned by GitHub API
#[derive(Debug, Deserialize)]
pub struct GithubReviewComment {
    pub id: i64,
    pub user: Option<GithubUser>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub pull_request_review_id: Option<i64>,
}

/// Commit as returned by GitHub API
#[derive(Debug, Deserialize)]
pub struct GithubCommit {
    pub sha: String,
    pub commit: GithubCommitDetail,
}

#[derive(Debug, Deserialize)]
pub struct GithubCommitDetail {
    pub author: GithubCommitAuthor,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubCommitAuthor {
    pub date: DateTime<Utc>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        let client = reqwest::Client::new();
        Self { client, token }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("review-royale/0.1"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        if let Some(ref token) = self.token {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        headers
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, ClientError> {
        debug!("GET {}", url);
        let resp = self.client.get(url).headers(self.headers()).send().await?;

        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(ClientError::NotFound(url.to_string()));
        }
        if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
            return Err(ClientError::RateLimited { retry_after });
        }
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ClientError::Api {
                status: status.as_u16(),
                message,
            });
        }

        Ok(resp.json().await?)
    }

    /// Fetch repository info
    pub async fn get_repo(&self, owner: &str, name: &str) -> Result<GithubRepo, ClientError> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, name);
        self.get(&url).await
    }

    /// Fetch PRs, paginated. Returns PRs updated since `since` if provided.
    /// GitHub returns newest first by default (sorted by created desc).
    pub async fn list_prs(
        &self,
        owner: &str,
        repo: &str,
        state: &str, // "all", "open", "closed"
        page: u32,
        per_page: u32,
    ) -> Result<Vec<GithubPr>, ClientError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls?state={}&page={}&per_page={}&sort=updated&direction=desc",
            owner, repo, state, page, per_page
        );
        self.get(&url).await
    }

    /// Fetch all reviews for a PR
    pub async fn list_reviews(
        &self,
        owner: &str,
        repo: &str,
        pr_number: i32,
    ) -> Result<Vec<GithubReview>, ClientError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/reviews",
            owner, repo, pr_number
        );
        self.get(&url).await
    }

    /// Fetch review comments for a PR (to count comments per review)
    pub async fn list_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: i32,
    ) -> Result<Vec<GithubReviewComment>, ClientError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/comments?per_page=100",
            owner, repo, pr_number
        );
        self.get(&url).await
    }

    /// Fetch all PRs updated since a given date, handling pagination
    pub async fn fetch_prs_since(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        max_age_days: u32,
    ) -> Result<Vec<GithubPr>, ClientError> {
        let cutoff =
            since.unwrap_or_else(|| Utc::now() - chrono::Duration::days(max_age_days as i64));

        let mut all_prs = Vec::new();
        let mut page = 1u32;
        let per_page = 100u32;

        loop {
            info!("Fetching PRs page {} for {}/{}", page, owner, repo);
            let prs = self.list_prs(owner, repo, "all", page, per_page).await?;

            if prs.is_empty() {
                break;
            }

            // Check if we've gone past the cutoff
            let oldest_in_page = prs.iter().map(|p| p.updated_at).min();
            let mut should_stop = false;

            for pr in prs {
                if pr.updated_at >= cutoff {
                    all_prs.push(pr);
                } else {
                    // PRs are sorted by updated desc, so once we hit old ones, stop
                    should_stop = true;
                    break;
                }
            }

            if should_stop {
                debug!("Reached PRs older than cutoff, stopping pagination");
                break;
            }

            if oldest_in_page.map(|d| d < cutoff).unwrap_or(false) {
                break;
            }

            page += 1;

            // Safety: don't fetch more than 50 pages (5000 PRs)
            if page > 50 {
                warn!("Hit pagination limit of 50 pages");
                break;
            }
        }

        info!("Fetched {} PRs total for {}/{}", all_prs.len(), owner, repo);
        Ok(all_prs)
    }

    /// Fetch commits for a PR
    pub async fn fetch_commits(
        &self,
        owner: &str,
        repo: &str,
        pr_number: i32,
    ) -> Result<Vec<GithubCommit>, ClientError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/commits",
            owner, repo, pr_number
        );
        self.get_paginated(&url, 100).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GitHubClient::new(None);
        assert!(client.token.is_none());

        let client = GitHubClient::new(Some("test".to_string()));
        assert_eq!(client.token, Some("test".to_string()));
    }
}
