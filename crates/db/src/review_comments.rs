//! Review comment queries

#![allow(clippy::too_many_arguments)]

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// A stored review comment
#[derive(Debug, Clone)]
pub struct ReviewComment {
    pub id: Uuid,
    pub review_id: Option<Uuid>,
    pub pr_id: Uuid,
    pub user_id: Uuid,
    pub github_id: i64,
    pub body: String,
    pub path: Option<String>,
    pub diff_hunk: Option<String>,
    pub line: Option<i32>,
    pub in_reply_to_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub category: Option<String>,
    pub quality_score: Option<i32>,
}

/// Insert a new review comment
pub async fn insert(
    pool: &PgPool,
    review_id: Option<Uuid>,
    pr_id: Uuid,
    user_id: Uuid,
    github_id: i64,
    body: &str,
    path: Option<&str>,
    diff_hunk: Option<&str>,
    line: Option<i32>,
    in_reply_to_id: Option<i64>,
    created_at: DateTime<Utc>,
) -> Result<ReviewComment, sqlx::Error> {
    let id = Uuid::new_v4();
    let row = sqlx::query(
        r#"
        INSERT INTO review_comments 
            (id, review_id, pr_id, user_id, github_id, body, path, diff_hunk, line, in_reply_to_id, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (github_id) DO UPDATE
        SET body = EXCLUDED.body,
            path = EXCLUDED.path,
            diff_hunk = EXCLUDED.diff_hunk,
            line = EXCLUDED.line
        RETURNING id, review_id, pr_id, user_id, github_id, body, path, diff_hunk, line, 
                  in_reply_to_id, created_at, category, quality_score
        "#,
    )
    .bind(id)
    .bind(review_id)
    .bind(pr_id)
    .bind(user_id)
    .bind(github_id)
    .bind(body)
    .bind(path)
    .bind(diff_hunk)
    .bind(line)
    .bind(in_reply_to_id)
    .bind(created_at)
    .fetch_one(pool)
    .await?;

    Ok(ReviewComment {
        id: row.get("id"),
        review_id: row.get("review_id"),
        pr_id: row.get("pr_id"),
        user_id: row.get("user_id"),
        github_id: row.get("github_id"),
        body: row.get("body"),
        path: row.get("path"),
        diff_hunk: row.get("diff_hunk"),
        line: row.get("line"),
        in_reply_to_id: row.get("in_reply_to_id"),
        created_at: row.get("created_at"),
        category: row.get("category"),
        quality_score: row.get("quality_score"),
    })
}

/// Get comments for a review
pub async fn list_for_review(
    pool: &PgPool,
    review_id: Uuid,
) -> Result<Vec<ReviewComment>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, review_id, pr_id, user_id, github_id, body, path, diff_hunk, line,
               in_reply_to_id, created_at, category, quality_score
        FROM review_comments
        WHERE review_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(review_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_comment).collect())
}

/// Get comments for a PR
pub async fn list_for_pr(pool: &PgPool, pr_id: Uuid) -> Result<Vec<ReviewComment>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, review_id, pr_id, user_id, github_id, body, path, diff_hunk, line,
               in_reply_to_id, created_at, category, quality_score
        FROM review_comments
        WHERE pr_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(pr_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_comment).collect())
}

/// Get comments by a user
pub async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<ReviewComment>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, review_id, pr_id, user_id, github_id, body, path, diff_hunk, line,
               in_reply_to_id, created_at, category, quality_score
        FROM review_comments
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_comment).collect())
}

/// Count comments without category (for AI processing queue)
pub async fn count_uncategorized(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM review_comments
        WHERE category IS NULL
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, _>("count"))
}

/// Update category and quality score for a comment
pub async fn set_category(
    pool: &PgPool,
    id: Uuid,
    category: &str,
    quality_score: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE review_comments
        SET category = $2, quality_score = $3
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(category)
    .bind(quality_score)
    .execute(pool)
    .await?;

    Ok(())
}

fn row_to_comment(row: sqlx::postgres::PgRow) -> ReviewComment {
    ReviewComment {
        id: row.get("id"),
        review_id: row.get("review_id"),
        pr_id: row.get("pr_id"),
        user_id: row.get("user_id"),
        github_id: row.get("github_id"),
        body: row.get("body"),
        path: row.get("path"),
        diff_hunk: row.get("diff_hunk"),
        line: row.get("line"),
        in_reply_to_id: row.get("in_reply_to_id"),
        created_at: row.get("created_at"),
        category: row.get("category"),
        quality_score: row.get("quality_score"),
    }
}

/// Quality data for XP calculation
#[derive(Debug, Clone, Default)]
pub struct CommentQualityData {
    /// Count by quality tier: (low, medium, high)
    pub by_tier: (i32, i32, i32),
    /// Count by category: (logic, structural, other)
    pub by_category: (i32, i32, i32),
    /// Total categorized comments
    pub categorized_count: i32,
}

/// Get aggregated quality data for comments by PR and user
pub async fn get_quality_data_for_pr_user(
    pool: &PgPool,
    pr_id: Uuid,
    user_id: Uuid,
) -> Result<CommentQualityData, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) FILTER (WHERE quality_score IS NOT NULL AND quality_score <= 3) as low_quality,
            COUNT(*) FILTER (WHERE quality_score IS NOT NULL AND quality_score >= 4 AND quality_score <= 6) as medium_quality,
            COUNT(*) FILTER (WHERE quality_score IS NOT NULL AND quality_score >= 7) as high_quality,
            COUNT(*) FILTER (WHERE category = 'logic') as logic_count,
            COUNT(*) FILTER (WHERE category = 'structural') as structural_count,
            COUNT(*) FILTER (WHERE category IS NOT NULL AND category NOT IN ('logic', 'structural')) as other_count,
            COUNT(*) FILTER (WHERE category IS NOT NULL) as categorized_count
        FROM review_comments
        WHERE pr_id = $1 AND user_id = $2
        "#,
    )
    .bind(pr_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(CommentQualityData {
        by_tier: (
            row.get::<i64, _>("low_quality") as i32,
            row.get::<i64, _>("medium_quality") as i32,
            row.get::<i64, _>("high_quality") as i32,
        ),
        by_category: (
            row.get::<i64, _>("logic_count") as i32,
            row.get::<i64, _>("structural_count") as i32,
            row.get::<i64, _>("other_count") as i32,
        ),
        categorized_count: row.get::<i64, _>("categorized_count") as i32,
    })
}
