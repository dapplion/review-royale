# Review Royale 👑

> Gamified PR review analytics. Make code review competitive, fun, and visible.

## What is this?

A platform that tracks PR review behavior and gamifies it with:
- 📊 **Metrics** - Time to first review, review depth, response times
- 🏆 **Achievements** - Unlock badges for review milestones
- 📈 **Leaderboards** - Weekly, monthly, seasonal rankings
- 🤖 **Bot** - Discord notifications, weekly digests, playful roasts

## Architecture

```
GitHub Webhooks → API Server → PostgreSQL
                     ↓
              Processor (metrics, achievements)
                     ↓
              Redis (cache, leaderboards)
                     ↓
            Frontend + Discord Bot
```

## Quick Start

```bash
# Start local dependencies
docker-compose up -d

# Run migrations
cargo run -p db --bin migrate

# Start API server
cargo run -p api

# Start bot (optional)
cargo run -p bot
```

## Configuration

Copy `.env.example` to `.env` and fill in:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/review_royale
REDIS_URL=redis://localhost:6379
GITHUB_APP_ID=your_app_id
GITHUB_PRIVATE_KEY_PATH=./github-app.pem
GITHUB_WEBHOOK_SECRET=your_webhook_secret
DISCORD_TOKEN=your_discord_bot_token
```

## Development

```bash
# Check everything compiles
cargo check --workspace

# Run tests
cargo test --workspace

# Format code
cargo fmt --all

# Lint
cargo clippy --workspace
```

## Crates

| Crate | Description |
|-------|-------------|
| `common` | Shared types, config, errors |
| `db` | Database models and queries |
| `github` | GitHub API client and webhook handling |
| `processor` | Metrics computation, achievements |
| `api` | HTTP API server |
| `bot` | Discord bot |

## License

MIT
