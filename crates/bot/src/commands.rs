//! Bot commands

use chrono::{Duration, Utc};
use serenity::model::channel::Message;
use serenity::prelude::*;
use sqlx::PgPool;
use tracing::info;

pub async fn handle(
    ctx: &Context,
    msg: &Message,
    command: &str,
    pool: &PgPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    match parts.first() {
        Some(&"leaderboard") | Some(&"lb") => leaderboard(ctx, msg, pool).await,
        Some(&"stats") => {
            let username = parts.get(1).copied();
            stats(ctx, msg, pool, username).await
        }
        Some(&"help") => help(ctx, msg).await,
        _ => {
            msg.reply(&ctx.http, "Unknown command. Try `!rr help`")
                .await?;
            Ok(())
        }
    }
}

async fn leaderboard(
    ctx: &Context,
    msg: &Message,
    pool: &PgPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Leaderboard command from {}", msg.author.name);

    let since = Utc::now() - Duration::days(30);
    let entries = db::leaderboard::get_leaderboard(pool, None, since, 10).await?;

    if entries.is_empty() {
        msg.reply(&ctx.http, "No reviews yet! Get reviewing! 🔍")
            .await?;
        return Ok(());
    }

    let mut response = String::from("👑 **Review Royale Leaderboard** (Last 30 days)\n\n");

    for entry in entries {
        let medal = match entry.rank {
            1 => "🥇",
            2 => "🥈",
            3 => "🥉",
            _ => "  ",
        };

        response.push_str(&format!(
            "{} **#{}** {} — {} reviews (Level {})\n",
            medal, entry.rank, entry.user.login, entry.stats.reviews_given, entry.user.level
        ));
    }

    msg.reply(&ctx.http, response).await?;
    Ok(())
}

async fn stats(
    ctx: &Context,
    msg: &Message,
    pool: &PgPool,
    username: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let username = username.unwrap_or(msg.author.name.as_str());

    info!("Stats command for {} from {}", username, msg.author.name);

    let user = match db::users::get_by_login(pool, username).await? {
        Some(u) => u,
        None => {
            msg.reply(&ctx.http, format!("User `{}` not found", username))
                .await?;
            return Ok(());
        }
    };

    let since = Utc::now() - Duration::days(30);
    let reviews = db::reviews::count_by_user(pool, user.id, since).await?;
    let rank = db::leaderboard::get_user_rank(pool, user.id, None, since).await?;
    let achievements = db::achievements::list_for_user(pool, user.id).await?;

    let response = format!(
        "📊 **Stats for {}**\n\n\
        🎮 **Level:** {}\n\
        ⭐ **XP:** {}\n\
        📝 **Reviews (30d):** {}\n\
        🏆 **Rank:** {}\n\
        🎖️ **Achievements:** {}\n",
        user.login,
        user.level,
        user.xp,
        reviews,
        rank.map(|r| format!("#{}", r))
            .unwrap_or_else(|| "Unranked".to_string()),
        achievements.len()
    );

    msg.reply(&ctx.http, response).await?;
    Ok(())
}

async fn help(
    ctx: &Context,
    msg: &Message,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = "👑 **Review Royale Commands**\n\n\
        `!rr leaderboard` — Show top reviewers\n\
        `!rr stats [username]` — Show user stats\n\
        `!rr help` — Show this help\n\n\
        More commands coming soon! 🚀";

    msg.reply(&ctx.http, response).await?;
    Ok(())
}
