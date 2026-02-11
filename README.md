# 👑 Review Royale

Gamified PR review analytics for GitHub repositories. Track reviews, earn XP, unlock achievements, climb the leaderboard.

**Live at**: https://review-royale.fly.dev

## Features

- **XP System**: Earn points for reviews, fast responses, thorough feedback
- **Session-Based Scoring**: Groups review activity into meaningful "review sessions"
- **AI Quality Analysis**: Comments categorized and scored for quality (logic bugs > cosmetic nits)
- **Achievements**: Unlock badges for review milestones and streaks
- **Leaderboards**: Weekly, monthly, and all-time rankings per repo or globally
- **Discord Bot**: Weekly digests, achievement notifications, and friendly roasts
- **Multi-Repo Support**: Track any public GitHub repo

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
| `GET /api/repos/:owner/:name/leaderboard` | Repo-specific leaderboard |
| `GET /api/users/:username` | User profile & stats |
| `POST /api/backfill/:owner/:repo` | Trigger backfill |
| `POST /api/recalculate` | Recalculate all XP from reviews |

## Scoring

A **review session** = one meaningful pass reviewing a specific version of code.

| Action | XP |
|--------|-----|
| Base (per review session) | 10 |
| Per substantive comment (>20 chars) | +5 |
| Fast review (<1 hour after commits) | +10 |
| Thorough (>5 comments) | +5 |
| Deep review (>10 comments) | +10 |

**Quality-weighted XP** (when AI categorization is enabled):
- High-quality comments (7-10): +8 XP each
- Logic bug catches: +3 XP bonus
- Structural improvements: +2 XP bonus

## Tech Stack

- **Backend**: Rust (Axum)
- **Database**: PostgreSQL
- **Bot**: Discord (Serenity)
- **Frontend**: Vanilla JS
- **Deployment**: Fly.io

## Development

```bash
# Set up git hooks (runs cargo fmt check on push)
git config core.hooksPath .githooks

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
