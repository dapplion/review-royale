//! GitHub API client for fetching PRs and reviews

pub mod client;

pub use client::{
    ClientError, GitHubClient, GithubCommit, GithubPr, GithubRepo, GithubReview,
    GithubReviewComment, GithubUser,
};
