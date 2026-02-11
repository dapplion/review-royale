//! Team management and team leaderboard queries

use chrono::{DateTime, Utc};
use common::models::{Team, TeamLeaderboardEntry};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Create a new team
pub async fn create_team(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
    color: Option<&str>,
) -> Result<Team, sqlx::Error> {
    let id = Uuid::new_v4();
    let color = color.unwrap_or("#6366f1");

    let row = sqlx::query(
        r#"
        INSERT INTO teams (id, name, description, color)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, description, color, created_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(color)
    .fetch_one(pool)
    .await?;

    Ok(Team {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        color: row.get("color"),
        created_at: row.get("created_at"),
    })
}

/// Get a team by ID
pub async fn get_team(pool: &PgPool, team_id: Uuid) -> Result<Option<Team>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, name, description, color, created_at
        FROM teams
        WHERE id = $1
        "#,
    )
    .bind(team_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Team {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
        color: r.get("color"),
        created_at: r.get("created_at"),
    }))
}

/// Get a team by name
pub async fn get_team_by_name(pool: &PgPool, name: &str) -> Result<Option<Team>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, name, description, color, created_at
        FROM teams
        WHERE name = $1
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Team {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
        color: r.get("color"),
        created_at: r.get("created_at"),
    }))
}

/// List all teams
pub async fn list_teams(pool: &PgPool) -> Result<Vec<Team>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, description, color, created_at
        FROM teams
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Team {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            color: r.get("color"),
            created_at: r.get("created_at"),
        })
        .collect())
}

/// Add a user to a team
pub async fn add_member(pool: &PgPool, team_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO team_members (team_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (team_id, user_id) DO NOTHING
        "#,
    )
    .bind(team_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove a user from a team
pub async fn remove_member(pool: &PgPool, team_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM team_members
        WHERE team_id = $1 AND user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get team leaderboard for a time period
pub async fn get_team_leaderboard(
    pool: &PgPool,
    repo_id: Option<Uuid>,
    since: DateTime<Utc>,
    limit: i32,
) -> Result<Vec<TeamLeaderboardEntry>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        WITH team_stats AS (
            SELECT 
                t.id as team_id,
                t.name,
                t.description,
                t.color,
                t.created_at,
                COUNT(DISTINCT tm.user_id)::int as member_count,
                COALESCE(SUM(r.xp_earned), 0)::bigint as total_xp,
                COUNT(r.id)::int as reviews_count
            FROM teams t
            LEFT JOIN team_members tm ON tm.team_id = t.id
            LEFT JOIN users u ON u.id = tm.user_id
            LEFT JOIN reviews r ON r.reviewer_id = u.id AND r.submitted_at >= $1
            LEFT JOIN pull_requests pr ON pr.id = r.pr_id
            WHERE u.login NOT LIKE '%[bot]' OR u.id IS NULL
              AND ($2::uuid IS NULL OR pr.repo_id = $2 OR r.id IS NULL)
            GROUP BY t.id, t.name, t.description, t.color, t.created_at
        )
        SELECT 
            team_id,
            name,
            description,
            color,
            created_at,
            member_count,
            total_xp,
            reviews_count
        FROM team_stats
        WHERE member_count > 0
        ORDER BY total_xp DESC
        LIMIT $3
        "#,
    )
    .bind(since)
    .bind(repo_id)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| TeamLeaderboardEntry {
            rank: (idx + 1) as i32,
            team: Team {
                id: row.get("team_id"),
                name: row.get("name"),
                description: row.get("description"),
                color: row.get("color"),
                created_at: row.get("created_at"),
            },
            score: row.get("total_xp"),
            member_count: row.get("member_count"),
            reviews_count: row.get("reviews_count"),
        })
        .collect();

    Ok(entries)
}

/// Get members of a team with their stats
pub async fn get_team_members(
    pool: &PgPool,
    team_id: Uuid,
) -> Result<Vec<(Uuid, String, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT u.id, u.login, u.xp
        FROM team_members tm
        JOIN users u ON u.id = tm.user_id
        WHERE tm.team_id = $1
        ORDER BY u.xp DESC
        "#,
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| (r.get("id"), r.get("login"), r.get("xp")))
        .collect())
}

/// Delete a team
pub async fn delete_team(pool: &PgPool, team_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(team_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
