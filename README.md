# 👑 Review Royale

Gamified PR review analytics for GitHub repositories. Track reviews, earn XP, unlock achievements, climb the leaderboard.

## Features

- **XP System**: Earn points for reviews, fast responses, thorough feedback
- **Achievements**: Unlock badges for review milestones and streaks
- **Leaderboards**: Compete with your team for review glory
- **Discord Bot**: Weekly digests, notifications, and friendly roasts
- **Zero Setup**: Works on any public repo via GitHub API polling

## Quick Start

```bash
# Clone
git clone https://github.com/dapplion/review-royale
cd review-royale

# Set up environment
cp .env.example .env
# Edit .env with your DATABASE_URL and GITHUB_TOKEN

# Run with Docker
docker-compose up -d

# Or run locally
cargo run -p api
```

## Backfill a Repository

```bash
# Fetch 1 year of PR review history
curl -X POST "http://localhost:3000/api/backfill/sigp/lighthouse?max_days=365"

# Check leaderboard
curl "http://localhost:3000/api/leaderboard"
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check |
| `GET /api/leaderboard` | Global leaderboard |
| `GET /api/repos` | List tracked repos |
| `GET /api/users/:username` | User profile & stats |
| `POST /api/backfill/:owner/:repo` | Trigger backfill |

## Scoring

| Action | XP |
|--------|-----|
| Submit review | 10 |
| First review on PR | +15 bonus |
| Review within 1 hour | +10 bonus |
| Per review comment | +5 |
| PR merged (author) | 20 |

## Tech Stack

- **Backend**: Rust (Axum)
- **Database**: PostgreSQL
- **Bot**: Discord (Serenity)
- **Frontend**: SvelteKit (planned)

## Development

```bash
# Run tests
cargo test

# Run with hot reload
cargo watch -x 'run -p api'

# Format & lint
cargo fmt
cargo clippy
```

## License

MIT
