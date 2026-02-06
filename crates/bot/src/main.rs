//! Review Royale Discord Bot

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use sqlx::PgPool;
use tracing::{error, info};

mod commands;

struct Bot {
    pool: PgPool,
}

struct DbPool;
impl TypeMapKey for DbPool {
    type Value = PgPool;
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

    async fn ready(&self, _: Context, ready: Ready) {
        info!("🤖 {} is connected!", ready.user.name);
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
