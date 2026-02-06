//! User queries

use common::models::User;
use sqlx::PgPool;
use uuid::Uuid;

/// Get or create a user from GitHub data
pub async fn upsert(
    pool: &PgPool,
    github_id: i64,
    login: &str,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, github_id, login, avatar_url, xp, level, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 0, 1, NOW(), NOW())
        ON CONFLICT (github_id) DO UPDATE
        SET login = EXCLUDED.login, 
            avatar_url = EXCLUDED.avatar_url,
            updated_at = NOW()
        RETURNING id, github_id, login, avatar_url, xp, level, created_at, updated_at
        "#,
        Uuid::new_v4(),
        github_id,
        login,
        avatar_url,
    )
    .fetch_one(pool)
    .await
}

/// Get user by GitHub login
pub async fn get_by_login(pool: &PgPool, login: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, github_id, login, avatar_url, xp, level, created_at, updated_at FROM users WHERE login = $1",
        login,
    )
    .fetch_optional(pool)
    .await
}

/// Get user by ID
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, github_id, login, avatar_url, xp, level, created_at, updated_at FROM users WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await
}

/// Add XP to a user and potentially level up
pub async fn add_xp(pool: &PgPool, user_id: Uuid, xp: i64) -> Result<User, sqlx::Error> {
    // Simple leveling: level = floor(sqrt(xp / 100)) + 1
    sqlx::query_as!(
        User,
        r#"
        UPDATE users
        SET xp = xp + $2,
            level = FLOOR(SQRT((xp + $2) / 100.0))::int + 1,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, github_id, login, avatar_url, xp, level, created_at, updated_at
        "#,
        user_id,
        xp,
    )
    .fetch_one(pool)
    .await
}
