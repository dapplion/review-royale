//! Season queries and management

use chrono::{DateTime, Datelike, Utc};
use common::models::{LeaderboardEntry, Season, User, UserStats};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Get all seasons ordered by number descending
pub async fn get_all_seasons(pool: &PgPool) -> Result<Vec<Season>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, number, starts_at, ends_at
        FROM seasons
        ORDER BY number DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Season {
            id: row.get("id"),
            name: row.get("name"),
            number: row.get("number"),
            starts_at: row.get("starts_at"),
            ends_at: row.get("ends_at"),
        })
        .collect())
}

/// Get the current active season (if any)
pub async fn get_current_season(pool: &PgPool) -> Result<Option<Season>, sqlx::Error> {
    let now = Utc::now();
    let row = sqlx::query(
        r#"
        SELECT id, name, number, starts_at, ends_at
        FROM seasons
        WHERE starts_at <= $1 AND ends_at > $1
        ORDER BY number DESC
        LIMIT 1
        "#,
    )
    .bind(now)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Season {
        id: r.get("id"),
        name: r.get("name"),
        number: r.get("number"),
        starts_at: r.get("starts_at"),
        ends_at: r.get("ends_at"),
    }))
}

/// Get a season by number
pub async fn get_season_by_number(
    pool: &PgPool,
    number: i32,
) -> Result<Option<Season>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, name, number, starts_at, ends_at
        FROM seasons
        WHERE number = $1
        "#,
    )
    .bind(number)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Season {
        id: r.get("id"),
        name: r.get("name"),
        number: r.get("number"),
        starts_at: r.get("starts_at"),
        ends_at: r.get("ends_at"),
    }))
}

/// Create a new season
pub async fn create_season(
    pool: &PgPool,
    name: &str,
    number: i32,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
) -> Result<Season, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO seasons (id, name, number, starts_at, ends_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(number)
    .bind(starts_at)
    .bind(ends_at)
    .execute(pool)
    .await?;

    Ok(Season {
        id,
        name: name.to_string(),
        number,
        starts_at,
        ends_at,
    })
}

/// Create a monthly season (convenience function)
pub async fn create_monthly_season(
    pool: &PgPool,
    year: i32,
    month: u32,
) -> Result<Season, sqlx::Error> {
    use chrono::TimeZone;

    let starts_at = Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();

    // Calculate end of month
    let (end_year, end_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let ends_at = Utc.with_ymd_and_hms(end_year, end_month, 1, 0, 0, 0).unwrap();

    // Season number: YYYYMM format
    let number = year * 100 + month as i32;
    let name = format!("{} {}", month_name(month), year);

    create_season(pool, &name, number, starts_at, ends_at).await
}

/// Get season leaderboard
pub async fn get_season_leaderboard(
    pool: &PgPool,
    season_id: Uuid,
    repo_id: Option<Uuid>,
    limit: i32,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    // Get the season's date range
    let season = sqlx::query("SELECT starts_at, ends_at FROM seasons WHERE id = $1")
        .bind(season_id)
        .fetch_optional(pool)
        .await?;

    let Some(season_row) = season else {
        return Ok(vec![]);
    };

    let starts_at: DateTime<Utc> = season_row.get("starts_at");
    let ends_at: DateTime<Utc> = season_row.get("ends_at");

    // Use the existing leaderboard logic but with season date bounds
    let rows = sqlx::query(
        r#"
        WITH first_reviews AS (
            SELECT DISTINCT ON (r.pr_id) 
                r.pr_id,
                r.reviewer_id
            FROM reviews r
            JOIN pull_requests pr ON pr.id = r.pr_id
            JOIN users u ON u.id = r.reviewer_id
            WHERE r.submitted_at >= $1 AND r.submitted_at < $2
              AND ($3::uuid IS NULL OR pr.repo_id = $3)
              AND u.login NOT LIKE '%[bot]'
            ORDER BY r.pr_id, r.submitted_at ASC
        ),
        user_stats AS (
            SELECT 
                u.id,
                COUNT(r.id)::int as reviews_given,
                COUNT(DISTINCT r.pr_id)::int as prs_reviewed,
                COALESCE(SUM(r.comments_count), 0)::int as comments_written,
                COALESCE(SUM(r.xp_earned), 0)::bigint as period_xp,
                COALESCE((SELECT COUNT(*) FROM first_reviews fr WHERE fr.reviewer_id = u.id), 0)::int as first_reviews
            FROM users u
            LEFT JOIN reviews r ON r.reviewer_id = u.id 
                AND r.submitted_at >= $1 AND r.submitted_at < $2
            LEFT JOIN pull_requests pr ON pr.id = r.pr_id
            WHERE ($3::uuid IS NULL OR pr.repo_id = $3)
              AND u.login NOT LIKE '%[bot]'
            GROUP BY u.id
            HAVING COUNT(r.id) > 0
        )
        SELECT 
            u.id, u.github_id, u.login, u.avatar_url, 
            u.xp, u.level,
            u.created_at, u.updated_at,
            us.reviews_given,
            us.prs_reviewed,
            us.comments_written,
            us.first_reviews,
            us.period_xp
        FROM users u
        JOIN user_stats us ON us.id = u.id
        WHERE u.login NOT LIKE '%[bot]'
        ORDER BY us.period_xp DESC, us.reviews_given DESC
        LIMIT $4
        "#,
    )
    .bind(starts_at)
    .bind(ends_at)
    .bind(repo_id)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| {
            let total_xp: i64 = row.get("xp");
            let period_xp: i64 = row.get("period_xp");
            let user = User {
                id: row.get("id"),
                github_id: row.get("github_id"),
                login: row.get("login"),
                avatar_url: row.get("avatar_url"),
                xp: total_xp,
                level: row.get("level"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            LeaderboardEntry {
                rank: (idx + 1) as i32,
                score: period_xp,
                user,
                stats: UserStats {
                    reviews_given: row.get("reviews_given"),
                    prs_reviewed: row.get("prs_reviewed"),
                    comments_written: row.get("comments_written"),
                    first_reviews: row.get("first_reviews"),
                    ..Default::default()
                },
            }
        })
        .collect();

    Ok(entries)
}

/// Ensure current month's season exists, create if not
pub async fn ensure_current_season(pool: &PgPool) -> Result<Season, sqlx::Error> {
    let now = Utc::now();
    let year = now.year();
    let month = now.month();

    // Check if season exists
    let number = year * 100 + month as i32;
    if let Some(season) = get_season_by_number(pool, number).await? {
        return Ok(season);
    }

    // Create it
    create_monthly_season(pool, year, month).await
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}
