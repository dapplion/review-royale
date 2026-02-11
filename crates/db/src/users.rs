//! User queries

use chrono::{DateTime, Utc};
use common::models::{User, UserStats};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Get or create a user from GitHub data
pub async fn upsert(
    pool: &PgPool,
    github_id: i64,
    login: &str,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    let id = Uuid::new_v4();
    let row = sqlx::query(
        r#"
        INSERT INTO users (id, github_id, login, avatar_url, xp, level, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 0, 1, NOW(), NOW())
        ON CONFLICT (github_id) DO UPDATE
        SET login = EXCLUDED.login, 
            avatar_url = EXCLUDED.avatar_url,
            updated_at = NOW()
        RETURNING id, github_id, login, avatar_url, xp, level, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(github_id)
    .bind(login)
    .bind(avatar_url)
    .fetch_one(pool)
    .await?;

    Ok(User {
        id: row.get("id"),
        github_id: row.get("github_id"),
        login: row.get("login"),
        avatar_url: row.get("avatar_url"),
        xp: row.get("xp"),
        level: row.get("level"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// Get user by GitHub login
pub async fn get_by_login(pool: &PgPool, login: &str) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, github_id, login, avatar_url, xp, level, created_at, updated_at FROM users WHERE login = $1",
    )
    .bind(login)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| User {
        id: r.get("id"),
        github_id: r.get("github_id"),
        login: r.get("login"),
        avatar_url: r.get("avatar_url"),
        xp: r.get("xp"),
        level: r.get("level"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }))
}

/// Get user by ID
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, github_id, login, avatar_url, xp, level, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| User {
        id: r.get("id"),
        github_id: r.get("github_id"),
        login: r.get("login"),
        avatar_url: r.get("avatar_url"),
        xp: r.get("xp"),
        level: r.get("level"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }))
}

/// Get or create a user, returning whether they were newly created
pub async fn upsert_returning_created(
    pool: &PgPool,
    github_id: i64,
    login: &str,
    avatar_url: Option<&str>,
) -> Result<(User, bool), sqlx::Error> {
    // Check if user exists first
    let existing = sqlx::query("SELECT id FROM users WHERE github_id = $1")
        .bind(github_id)
        .fetch_optional(pool)
        .await?;

    let created = existing.is_none();
    let user = upsert(pool, github_id, login, avatar_url).await?;
    Ok((user, created))
}

/// Add XP to a user and potentially level up
pub async fn add_xp(pool: &PgPool, user_id: Uuid, xp: i64) -> Result<User, sqlx::Error> {
    // Simple leveling: level = floor(sqrt(xp / 100)) + 1
    let row = sqlx::query(
        r#"
        UPDATE users
        SET xp = xp + $2,
            level = FLOOR(SQRT((xp + $2) / 100.0))::int + 1,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, github_id, login, avatar_url, xp, level, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(xp)
    .fetch_one(pool)
    .await?;

    Ok(User {
        id: row.get("id"),
        github_id: row.get("github_id"),
        login: row.get("login"),
        avatar_url: row.get("avatar_url"),
        xp: row.get("xp"),
        level: row.get("level"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// Get user stats including reviews, comments, first reviews, distinct PRs reviewed
/// If repo_id is Some, stats are scoped to that repo only
pub async fn get_stats(
    pool: &PgPool,
    user_id: Uuid,
    since: DateTime<Utc>,
) -> Result<UserStats, sqlx::Error> {
    get_stats_for_repo(pool, user_id, None, since).await
}

/// Get user stats scoped to a specific repo (or all repos if repo_id is None)
pub async fn get_stats_for_repo(
    pool: &PgPool,
    user_id: Uuid,
    repo_id: Option<Uuid>,
    since: DateTime<Utc>,
) -> Result<UserStats, sqlx::Error> {
    let row = sqlx::query(
        r#"
        WITH first_reviews AS (
            SELECT DISTINCT ON (r.pr_id) r.reviewer_id
            FROM reviews r
            JOIN pull_requests pr ON pr.id = r.pr_id
            WHERE r.submitted_at >= $2
              AND ($3::uuid IS NULL OR pr.repo_id = $3)
            ORDER BY r.pr_id, r.submitted_at ASC
        )
        SELECT
            COUNT(r.id)::int as reviews_given,
            COUNT(DISTINCT r.pr_id)::int as prs_reviewed,
            COALESCE(SUM(r.comments_count), 0)::int as comments_written,
            COALESCE((SELECT COUNT(*) FROM first_reviews fr WHERE fr.reviewer_id = $1), 0)::int as first_reviews,
            COUNT(DISTINCT pr.id) FILTER (WHERE pr.author_id = $1)::int as prs_authored,
            COUNT(DISTINCT pr.id) FILTER (WHERE pr.author_id = $1 AND pr.merged_at IS NOT NULL)::int as prs_merged,
            COALESCE(SUM(r.xp_earned), 0)::bigint as period_xp,
            COUNT(r.id) FILTER (WHERE r.xp_earned > 0)::int as sessions
        FROM users u
        LEFT JOIN reviews r ON r.reviewer_id = u.id AND r.submitted_at >= $2
        LEFT JOIN pull_requests pr ON pr.id = r.pr_id
        WHERE u.id = $1
          AND ($3::uuid IS NULL OR pr.repo_id = $3)
        "#,
    )
    .bind(user_id)
    .bind(since)
    .bind(repo_id)
    .fetch_one(pool)
    .await?;

    Ok(UserStats {
        reviews_given: row.get("reviews_given"),
        prs_reviewed: row.get("prs_reviewed"),
        first_reviews: row.get("first_reviews"),
        comments_written: row.get("comments_written"),
        prs_authored: row.get("prs_authored"),
        prs_merged: row.get("prs_merged"),
        period_xp: row.get("period_xp"),
        sessions: row.get("sessions"),
        ..Default::default()
    })
}

/// Get weekly activity for a user (reviews per week)
pub async fn get_weekly_activity(
    pool: &PgPool,
    user_id: Uuid,
    weeks: i32,
) -> Result<Vec<(String, i32, i64)>, sqlx::Error> {
    get_weekly_activity_for_repo(pool, user_id, None, weeks).await
}

/// Get weekly activity for a user scoped to a specific repo
pub async fn get_weekly_activity_for_repo(
    pool: &PgPool,
    user_id: Uuid,
    repo_id: Option<Uuid>,
    weeks: i32,
) -> Result<Vec<(String, i32, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        WITH weeks AS (
            SELECT generate_series(
                date_trunc('week', NOW() - ($2 || ' weeks')::interval),
                date_trunc('week', NOW()),
                '1 week'::interval
            ) as week_start
        )
        SELECT
            to_char(w.week_start, 'Mon DD') as week,
            COUNT(r.id)::int as reviews,
            COALESCE(SUM(10 + r.comments_count * 5), 0)::bigint as xp
        FROM weeks w
        LEFT JOIN reviews r ON r.reviewer_id = $1 
            AND r.submitted_at >= w.week_start 
            AND r.submitted_at < w.week_start + '1 week'::interval
        LEFT JOIN pull_requests pr ON pr.id = r.pr_id
        WHERE ($3::uuid IS NULL OR pr.repo_id = $3)
        GROUP BY w.week_start
        ORDER BY w.week_start ASC
        "#,
    )
    .bind(user_id)
    .bind(weeks)
    .bind(repo_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.get::<String, _>("week"),
                r.get::<i32, _>("reviews"),
                r.get::<i64, _>("xp"),
            )
        })
        .collect())
}

/// Review with PR details for display
#[derive(Debug)]
pub struct ReviewWithPr {
    pub review_id: Uuid,
    pub state: String,
    pub comments_count: i32,
    pub submitted_at: DateTime<Utc>,
    pub pr_number: i32,
    pub pr_title: String,
    pub pr_state: String,
    pub repo_owner: String,
    pub repo_name: String,
}

/// Get recent reviews by a user
pub async fn get_recent_reviews(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<ReviewWithPr>, sqlx::Error> {
    get_recent_reviews_for_repo(pool, user_id, None, limit).await
}

/// Get recent reviews by a user scoped to a specific repo
pub async fn get_recent_reviews_for_repo(
    pool: &PgPool,
    user_id: Uuid,
    repo_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<ReviewWithPr>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            r.id as review_id,
            r.state::text as state,
            r.comments_count,
            r.submitted_at,
            pr.number as pr_number,
            pr.title as pr_title,
            pr.state::text as pr_state,
            repo.owner as repo_owner,
            repo.name as repo_name
        FROM reviews r
        JOIN pull_requests pr ON pr.id = r.pr_id
        JOIN repositories repo ON repo.id = pr.repo_id
        WHERE r.reviewer_id = $1
          AND ($3::uuid IS NULL OR repo.id = $3)
        ORDER BY r.submitted_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(repo_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ReviewWithPr {
            review_id: r.get("review_id"),
            state: r.get("state"),
            comments_count: r.get("comments_count"),
            submitted_at: r.get("submitted_at"),
            pr_number: r.get("pr_number"),
            pr_title: r.get("pr_title"),
            pr_state: r.get("pr_state"),
            repo_owner: r.get("repo_owner"),
            repo_name: r.get("repo_name"),
        })
        .collect())
}
