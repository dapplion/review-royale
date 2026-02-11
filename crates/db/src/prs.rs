//! Pull request queries

#![allow(clippy::too_many_arguments)]

use chrono::{DateTime, Utc};
use common::models::{PrState, PullRequest};
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn parse_pr_state(s: &str) -> PrState {
    match s {
        "merged" => PrState::Merged,
        "closed" => PrState::Closed,
        _ => PrState::Open,
    }
}

/// Create or update a pull request
pub async fn upsert(
    pool: &PgPool,
    repo_id: Uuid,
    github_id: i64,
    number: i32,
    title: &str,
    author_id: Uuid,
    state: PrState,
    created_at: DateTime<Utc>,
) -> Result<PullRequest, sqlx::Error> {
    let state_str = match state {
        PrState::Open => "open",
        PrState::Merged => "merged",
        PrState::Closed => "closed",
    };

    let id = Uuid::new_v4();
    let row = sqlx::query(
        r#"
        INSERT INTO pull_requests (id, repo_id, github_id, number, title, author_id, state, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (github_id) DO UPDATE
        SET title = EXCLUDED.title,
            state = EXCLUDED.state
        RETURNING id, repo_id, github_id, number, title, author_id, state, created_at, first_review_at, merged_at, closed_at
        "#,
    )
    .bind(id)
    .bind(repo_id)
    .bind(github_id)
    .bind(number)
    .bind(title)
    .bind(author_id)
    .bind(state_str)
    .bind(created_at)
    .fetch_one(pool)
    .await?;

    Ok(PullRequest {
        id: row.get("id"),
        repo_id: row.get("repo_id"),
        github_id: row.get("github_id"),
        number: row.get("number"),
        title: row.get("title"),
        author_id: row.get("author_id"),
        state: parse_pr_state(row.get("state")),
        created_at: row.get("created_at"),
        first_review_at: row.get("first_review_at"),
        merged_at: row.get("merged_at"),
        closed_at: row.get("closed_at"),
    })
}

/// Update merged_at and closed_at timestamps
pub async fn update_timestamps(
    pool: &PgPool,
    pr_id: Uuid,
    merged_at: Option<DateTime<Utc>>,
    closed_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE pull_requests
        SET merged_at = COALESCE($2, merged_at),
            closed_at = COALESCE($3, closed_at),
            state = CASE 
                WHEN $2 IS NOT NULL THEN 'merged'
                WHEN $3 IS NOT NULL THEN 'closed'
                ELSE state
            END
        WHERE id = $1
        "#,
    )
    .bind(pr_id)
    .bind(merged_at)
    .bind(closed_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Record first review time
pub async fn set_first_review(
    pool: &PgPool,
    pr_id: Uuid,
    first_review_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE pull_requests
        SET first_review_at = $2
        WHERE id = $1 AND first_review_at IS NULL
        "#,
    )
    .bind(pr_id)
    .bind(first_review_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get PR by repo and number
pub async fn get_by_number(
    pool: &PgPool,
    repo_id: Uuid,
    number: i32,
) -> Result<Option<PullRequest>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, repo_id, github_id, number, title, author_id, state, created_at, first_review_at, merged_at, closed_at
        FROM pull_requests
        WHERE repo_id = $1 AND number = $2
        "#,
    )
    .bind(repo_id)
    .bind(number)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| PullRequest {
        id: r.get("id"),
        repo_id: r.get("repo_id"),
        github_id: r.get("github_id"),
        number: r.get("number"),
        title: r.get("title"),
        author_id: r.get("author_id"),
        state: parse_pr_state(r.get("state")),
        created_at: r.get("created_at"),
        first_review_at: r.get("first_review_at"),
        merged_at: r.get("merged_at"),
        closed_at: r.get("closed_at"),
    }))
}

/// List recent PRs for a repo
pub async fn list_recent(
    pool: &PgPool,
    repo_id: Uuid,
    limit: i32,
) -> Result<Vec<PullRequest>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, repo_id, github_id, number, title, author_id, state, created_at, first_review_at, merged_at, closed_at
        FROM pull_requests
        WHERE repo_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(repo_id)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PullRequest {
            id: r.get("id"),
            repo_id: r.get("repo_id"),
            github_id: r.get("github_id"),
            number: r.get("number"),
            title: r.get("title"),
            author_id: r.get("author_id"),
            state: parse_pr_state(r.get("state")),
            created_at: r.get("created_at"),
            first_review_at: r.get("first_review_at"),
            merged_at: r.get("merged_at"),
            closed_at: r.get("closed_at"),
        })
        .collect())
}

/// Count PRs authored by a user
pub async fn count_by_author(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM pull_requests
        WHERE author_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}

/// Count merged PRs authored by a user
pub async fn count_merged_by_author(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM pull_requests
        WHERE author_id = $1 AND state = 'merged'
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}

/// Open PR with review statistics
#[derive(Debug)]
pub struct OpenPrWithStats {
    pub id: Uuid,
    pub number: i32,
    pub title: String,
    pub author_login: String,
    pub author_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub first_review_at: Option<DateTime<Utc>>,
    pub review_count: i32,
    pub approvals: i32,
    pub changes_requested: i32,
    pub comments_count: i32,
    pub latest_review_state: Option<String>,
    pub reviewers: Vec<String>,
}

/// Get open PRs with review statistics
pub async fn list_open_with_stats(
    pool: &PgPool,
    repo_id: Uuid,
) -> Result<Vec<OpenPrWithStats>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT 
            pr.id,
            pr.number,
            pr.title,
            author.login as author_login,
            author.avatar_url as author_avatar,
            pr.created_at,
            pr.first_review_at,
            COALESCE(rs.review_count, 0)::int as review_count,
            COALESCE(rs.approvals, 0)::int as approvals,
            COALESCE(rs.changes_requested, 0)::int as changes_requested,
            COALESCE(rs.comments_count, 0)::int as comments_count,
            rs.latest_review_state,
            COALESCE(rs.reviewers, ARRAY[]::text[]) as reviewers
        FROM pull_requests pr
        JOIN users author ON author.id = pr.author_id
        LEFT JOIN LATERAL (
            SELECT 
                COUNT(DISTINCT r.id)::int as review_count,
                COUNT(DISTINCT r.id) FILTER (WHERE r.state = 'approved')::int as approvals,
                COUNT(DISTINCT r.id) FILTER (WHERE r.state = 'changes_requested')::int as changes_requested,
                SUM(r.comments_count)::int as comments_count,
                (SELECT r2.state FROM reviews r2 WHERE r2.pr_id = pr.id ORDER BY r2.submitted_at DESC LIMIT 1) as latest_review_state,
                ARRAY_AGG(DISTINCT reviewer.login) FILTER (WHERE reviewer.login IS NOT NULL) as reviewers
            FROM reviews r
            JOIN users reviewer ON reviewer.id = r.reviewer_id
            WHERE r.pr_id = pr.id
        ) rs ON true
        WHERE pr.repo_id = $1 AND pr.state = 'open'
        ORDER BY pr.created_at ASC
        "#,
    )
    .bind(repo_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| OpenPrWithStats {
            id: r.get("id"),
            number: r.get("number"),
            title: r.get("title"),
            author_login: r.get("author_login"),
            author_avatar: r.get("author_avatar"),
            created_at: r.get("created_at"),
            first_review_at: r.get("first_review_at"),
            review_count: r.get("review_count"),
            approvals: r.get("approvals"),
            changes_requested: r.get("changes_requested"),
            comments_count: r.get("comments_count"),
            latest_review_state: r.get("latest_review_state"),
            reviewers: r.get("reviewers"),
        })
        .collect())
}

/// Count open PRs for a repo
pub async fn count_open(pool: &PgPool, repo_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM pull_requests
        WHERE repo_id = $1 AND state = 'open'
        "#,
    )
    .bind(repo_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}
