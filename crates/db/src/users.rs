//! User queries

use common::models::User;
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
