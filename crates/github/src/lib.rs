//! GitHub API client for fetching PRs and reviews

pub mod client;

pub use client::{
    ClientError, GitHubClient, GithubPr, GithubRepo, GithubReview, GithubReviewComment, GithubUser,
};
