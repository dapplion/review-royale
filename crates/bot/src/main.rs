//! Review Royale Discord Bot

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

mod commands;

struct Bot {
    pool: PgPool,
}

struct DbPool;
impl TypeMapKey for DbPool {
    type Value = PgPool;
}

struct NotificationChannel;
impl TypeMapKey for NotificationChannel {
    type Value = Option<ChannelId>;
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore bot messages
        if msg.author.bot {
            return;
        }

        // Handle commands
        if msg.content.starts_with("!rr ") {
            let command = &msg.content[4..];
            if let Err(e) = commands::handle(&ctx, &msg, command, &self.pool).await {
                error!("Command error: {}", e);
                let _ = msg.reply(&ctx.http, format!("Error: {}", e)).await;
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("🤖 {} is connected!", ready.user.name);

        // Start achievement notification loop
        let ctx = Arc::new(ctx);
        let pool = self.pool.clone();
        tokio::spawn(async move {
            achievement_notification_loop(ctx, pool).await;
        });
    }
}

/// Background loop that checks for and posts achievement notifications
async fn achievement_notification_loop(ctx: Arc<Context>, pool: PgPool) {
    // Get notification channel from env
    let channel_id = match std::env::var("DISCORD_NOTIFICATION_CHANNEL") {
        Ok(id) => match id.parse::<u64>() {
            Ok(id) => ChannelId::new(id),
            Err(_) => {
                warn!("Invalid DISCORD_NOTIFICATION_CHANNEL, achievement notifications disabled");
                return;
            }
        },
        Err(_) => {
            info!("DISCORD_NOTIFICATION_CHANNEL not set, achievement notifications disabled");
            return;
        }
    };

    info!(
        "🔔 Achievement notifications enabled for channel {}",
        channel_id
    );

    // Check every 60 seconds
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        match db::achievements::get_pending_notifications(&pool, 10).await {
            Ok(notifications) => {
                for notif in notifications {
                    let message = format!(
                        "🏆 **Achievement Unlocked!**\n\n{} {} earned **{}**!\n_{}_",
                        notif.achievement_emoji,
                        notif.user_login,
                        notif.achievement_name,
                        notif.achievement_description
                    );

                    match channel_id.say(&ctx.http, &message).await {
                        Ok(_) => {
                            info!(
                                "Posted achievement notification: {} -> {}",
                                notif.user_login, notif.achievement_name
                            );

                            // Mark as notified
                            if let Err(e) = db::achievements::mark_notified(
                                &pool,
                                notif.user_id,
                                &notif.achievement_id,
                            )
                            .await
                            {
                                error!("Failed to mark achievement as notified: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to post achievement notification: {}", e);
                        }
                    }

                    // Small delay between notifications to avoid rate limits
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
            Err(e) => {
                error!("Failed to fetch pending notifications: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("review_royale=debug".parse()?)
                .add_directive("bot=debug".parse()?),
        )
        .init();

    info!("🤖 Starting Review Royale Bot");

    // Load configuration
    let config = common::Config::from_env();

    let token = config.discord_token.expect("DISCORD_TOKEN must be set");

    // Connect to database
    let pool = db::create_pool(&config.database_url).await?;

    // Create bot
    let bot = Bot { pool: pool.clone() };

    // Create client
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents).event_handler(bot).await?;

    // Store pool in client data
    {
        let mut data = client.data.write().await;
        data.insert::<DbPool>(pool);
    }

    // Start bot
    info!("🚀 Starting bot...");
    if let Err(e) = client.start().await {
        error!("Client error: {}", e);
    }

    Ok(())
}
