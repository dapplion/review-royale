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
        Some(&"leaderboard") | Some(&"lb") => {
            // Optional: !rr lb week | !rr lb month | !rr lb all
            let period = parts.get(1).copied().unwrap_or("month");
            leaderboard(ctx, msg, pool, period).await
        }
        Some(&"stats") => {
            let username = parts.get(1).copied();
            stats(ctx, msg, pool, username).await
        }
        Some(&"roast") => {
            let username = parts.get(1).copied();
            roast(ctx, msg, pool, username).await
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
    period: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Leaderboard command from {} (period: {})",
        msg.author.name, period
    );

    let (since, period_label) = match period {
        "week" | "w" => (Utc::now() - Duration::days(7), "This Week"),
        "all" | "a" => (Utc::now() - Duration::days(365 * 10), "All Time"),
        _ => (Utc::now() - Duration::days(30), "This Month"), // Default: month
    };

    let entries = db::leaderboard::get_leaderboard(pool, None, since, 10).await?;

    if entries.is_empty() {
        msg.reply(&ctx.http, "No reviews yet! Get reviewing! 🔍")
            .await?;
        return Ok(());
    }

    let mut response = format!("👑 **Review Royale Leaderboard** ({})\n\n", period_label);

    for entry in entries {
        let medal = match entry.rank {
            1 => "🥇",
            2 => "🥈",
            3 => "🥉",
            _ => "  ",
        };

        response.push_str(&format!(
            "{} **#{}** {} — {} XP ({} reviews)\n",
            medal, entry.rank, entry.user.login, entry.score, entry.stats.reviews_given
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

async fn roast(
    ctx: &Context,
    msg: &Message,
    pool: &PgPool,
    username: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let username = username.unwrap_or(msg.author.name.as_str());

    info!("Roast command for {} from {}", username, msg.author.name);

    let user = match db::users::get_by_login(pool, username).await? {
        Some(u) => u,
        None => {
            msg.reply(
                &ctx.http,
                format!("User `{}` not found. Can't roast a ghost 👻", username),
            )
            .await?;
            return Ok(());
        }
    };

    let since_30d = Utc::now() - Duration::days(30);
    let since_7d = Utc::now() - Duration::days(7);

    let reviews_30d = db::reviews::count_by_user(pool, user.id, since_30d).await?;
    let reviews_7d = db::reviews::count_by_user(pool, user.id, since_7d).await?;
    let rank = db::leaderboard::get_user_rank(pool, user.id, None, since_30d).await?;

    // Generate roast based on stats
    let roast = generate_roast(
        &user.login,
        user.xp,
        user.level,
        reviews_30d,
        reviews_7d,
        rank,
    );

    msg.reply(
        &ctx.http,
        format!("🔥 **Roasting {}** 🔥\n\n{}", user.login, roast),
    )
    .await?;
    Ok(())
}

fn generate_roast(
    username: &str,
    xp: i64,
    level: i32,
    reviews_30d: i64,
    reviews_7d: i64,
    rank: Option<i32>,
) -> String {
    let mut roasts = Vec::new();

    // XP-based roasts
    if xp == 0 {
        roasts.push(format!(
            "{} has exactly 0 XP. Not even a participation trophy.",
            username
        ));
    } else if xp < 100 {
        roasts.push(format!(
            "With {} XP, {} is speedrunning mediocrity.",
            xp, username
        ));
    } else if xp > 5000 {
        roasts.push(format!(
            "{} XP? Touch grass, {}. The codebase isn't going anywhere.",
            xp, username
        ));
    }

    // Level-based roasts
    if level == 1 {
        roasts.push("Still level 1? Even bots level up faster.".to_string());
    } else if level >= 10 {
        roasts.push(format!(
            "Level {}? Someone's trying to make reviewing their whole personality.",
            level
        ));
    }

    // Activity-based roasts
    if reviews_7d == 0 && reviews_30d > 0 {
        roasts.push(
            "Ghosted the repo for a whole week. The PRs miss you. Just kidding, they don't."
                .to_string(),
        );
    } else if reviews_7d == 0 && reviews_30d == 0 {
        roasts.push(
            "Zero reviews in 30 days. At this point, just fork the repo and pretend it's yours."
                .to_string(),
        );
    } else if reviews_7d > 20 {
        roasts.push(
            "Over 20 reviews this week? Either dedicated or procrastinating something worse."
                .to_string(),
        );
    }

    // Rank-based roasts
    match rank {
        Some(1) => {
            roasts.push("Rank #1 huh? Lonely at the top... and everywhere else.".to_string())
        }
        Some(r) if r > 10 => roasts.push(format!(
            "Rank #{}? The leaderboard is just a list of shame at this point.",
            r
        )),
        Some(r) if r > 5 => {
            roasts.push(format!("Rank #{}: solidly in the \"participant\" tier.", r))
        }
        _ => {}
    }

    // Ratio roast
    if reviews_30d > 0 && xp < (reviews_30d * 15) {
        roasts.push(
            "Low XP per review ratio. Quality > quantity, but you chose neither.".to_string(),
        );
    }

    // Pick 2-3 random roasts (or all if less available)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    username.hash(&mut hasher);
    Utc::now().timestamp().hash(&mut hasher);
    let seed = hasher.finish() as usize;

    if roasts.is_empty() {
        return format!(
            "{} is so average, I can't even find anything to roast.",
            username
        );
    }

    let count = roasts.len().min(3);
    let mut selected = Vec::new();
    for i in 0..count {
        let idx = (seed + i * 7) % roasts.len();
        if !selected.contains(&roasts[idx]) {
            selected.push(roasts[idx].clone());
        }
    }

    if selected.is_empty() {
        selected.push(roasts[0].clone());
    }

    selected.join("\n\n")
}

async fn help(
    ctx: &Context,
    msg: &Message,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = "👑 **Review Royale Commands**\n\n\
        `!rr lb [period]` — Leaderboard (week/month/all, default: month)\n\
        `!rr stats [username]` — Show user stats\n\
        `!rr roast [username]` — Roast a reviewer 🔥\n\
        `!rr help` — Show this help\n\n\
        **Scoring:** Reviews earn XP based on depth and speed. More comments = more XP! 🔥";

    msg.reply(&ctx.http, response).await?;
    Ok(())
}
