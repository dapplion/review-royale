# Review Royale - Architecture & Plan

## Overview

Gamified PR review analytics for GitHub repositories. Tracks review activity via GitHub API polling, awards XP, achievements, and maintains leaderboards.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Review Royale                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   GitHub     │───▶│   Backfill   │───▶│   Database   │  │
│  │     API      │    │   Service    │    │  (Postgres)  │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│                                                   │          │
│                                                   ▼          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   Discord    │◀───│     API      │◀───│  Processor   │  │
│  │     Bot      │    │   Server     │    │  (XP/Achv)   │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│                             │                               │
│                             ▼                               │
│                      ┌──────────────┐                       │
│                      │   Frontend   │                       │
│                      │  (SvelteKit) │                       │
│                      └──────────────┘                       │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

1. **Backfill Service** polls GitHub API for PRs and reviews
2. **Processor** computes XP, checks achievement conditions
3. **Database** stores all state (users, PRs, reviews, achievements)
4. **API Server** exposes REST endpoints for leaderboard, stats
5. **Discord Bot** sends notifications, weekly digests, roasts
6. **Frontend** displays leaderboard and user profiles

## Crates

| Crate | Purpose |
|-------|---------|
| `common` | Shared types, config, errors |
| `db` | PostgreSQL queries and migrations |
| `github` | GitHub API client |
| `processor` | Backfill, XP calculation, achievements |
| `api` | REST API server (Axum) |
| `bot` | Discord bot (Serenity) |

## API Endpoints

### Public
- `GET /health` - Health check
- `GET /api/leaderboard` - Global leaderboard
- `GET /api/repos` - List tracked repositories
- `GET /api/repos/:owner/:name` - Repository details
- `GET /api/repos/:owner/:name/leaderboard` - Repo-specific leaderboard
- `GET /api/users/:username` - User profile
- `GET /api/users/:username/stats` - User statistics

### Admin
- `GET /api/backfill/:owner/:name` - Check backfill status
- `POST /api/backfill/:owner/:name` - Trigger backfill (params: `max_days`)

## Scoring System

### XP Awards
- **Review submitted**: 10 XP base
- **First review on PR**: +15 XP bonus
- **Fast review** (<1 hour): +10 XP bonus
- **Thorough review** (>3 comments): +5 XP per comment
- **PR merged** (author): 20 XP

### Levels
```
Level = floor(sqrt(XP / 100)) + 1
```

| Level | XP Required |
|-------|-------------|
| 1 | 0 |
| 2 | 100 |
| 3 | 400 |
| 4 | 900 |
| 5 | 1,600 |
| 10 | 8,100 |

### Achievements

| ID | Name | Description | XP | Rarity |
|----|------|-------------|-----|--------|
| `first_review` | First Blood | Submit your first review | 50 | Common |
| `review_10` | Getting Started | Submit 10 reviews | 100 | Common |
| `review_50` | Reviewer | Submit 50 reviews | 250 | Uncommon |
| `review_100` | Centurion | Submit 100 reviews | 500 | Rare |
| `speed_demon` | Speed Demon | Review a PR within 1 hour (10x) | 200 | Uncommon |
| `night_owl` | Night Owl | Submit 10 reviews after midnight | 150 | Uncommon |
| `review_streak_7` | On Fire | Review PRs 7 days in a row | 300 | Rare |

## Backfill Strategy

1. **Initial run**: Fetch PRs from last 365 days
2. **Incremental**: Track `last_synced_at` per repo, only fetch updated PRs
3. **Rate limiting**: Respect GitHub's 5000 req/hr, exponential backoff on 403
4. **Caching**: Store sync cursor to resume on failure

## Test Plan

### Unit Tests
- **github**: API client mocking, pagination, error handling
- **db**: CRUD operations, leaderboard queries, idempotency
- **processor**: XP formulas, achievement triggers, backfill dedup

### Integration Tests
- Backfill flow (mock GitHub → DB assertions)
- API endpoints (test server → JSON responses)
- Full flow (backfill → XP → leaderboard)

### Test Infrastructure
- Real Postgres in CI (already configured)
- `wiremock` for GitHub API mocking
- Transaction rollback for test isolation

## Deployment

- **Platform**: Fly.io
- **Database**: Fly Postgres
- **CI/CD**: GitHub Actions (auto-deploy on main)

## Milestones

### M1: Core Backend ✅
- [x] Database schema
- [x] GitHub API client
- [x] Backfill service
- [x] XP calculation
- [x] REST API

### M2: Polish (Current)
- [ ] Comprehensive test coverage
- [ ] Production deployment
- [ ] Backfill sigp/lighthouse

### M3: Discord Bot
- [ ] Leaderboard command
- [ ] Weekly digest
- [ ] Achievement notifications
- [ ] Roast command

### M4: Frontend
- [ ] SvelteKit app
- [ ] Leaderboard page
- [ ] User profiles
- [ ] Repository stats

### M5: Advanced Features
- [ ] Seasons (monthly/quarterly resets)
- [ ] Review quality scoring
- [ ] Team leaderboards
- [ ] Custom achievements
