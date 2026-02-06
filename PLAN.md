# Review Royale - Architecture & Plan

## Overview

Gamified PR review analytics for GitHub repositories. Tracks review activity via GitHub API polling, awards XP, achievements, and maintains leaderboards.

**Live at**: https://review-royale.fly.dev

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
│                             │                    │          │
│                      ┌──────┴───────┐            │          │
│                      │ Sync Service │            ▼          │
│                      │ (every N hr) │    ┌──────────────┐  │
│                      └──────────────┘    │  Processor   │  │
│                                          │  (XP/Achv)   │  │
│  ┌──────────────┐    ┌──────────────┐    └──────────────┘  │
│  │   Discord    │◀───│     API      │◀──────────┘          │
│  │     Bot      │    │   Server     │                       │
│  └──────────────┘    └──────────────┘                       │
│                             │                               │
│                             ▼                               │
│                      ┌──────────────┐                       │
│                      │   Frontend   │                       │
│                      │ (Vanilla JS) │                       │
│                      └──────────────┘                       │
└─────────────────────────────────────────────────────────────┘
```

## Crates

| Crate | Purpose |
|-------|---------|
| `common` | Shared types, config, errors |
| `db` | PostgreSQL queries and migrations |
| `github` | GitHub API client |
| `processor` | Backfill, sync, XP calculation, achievements |
| `api` | REST API server (Axum) + static frontend |
| `bot` | Discord bot (Serenity) — skeleton only |

## API Endpoints

### Public
- `GET /health` - Health check
- `GET /api/leaderboard?period=week|month|all&limit=N` - Global leaderboard
- `GET /api/repos` - List tracked repositories
- `GET /api/repos/:owner/:name` - Repository details
- `GET /api/repos/:owner/:name/leaderboard` - Repo-specific leaderboard
- `GET /api/users/:username` - User profile
- `GET /api/users/:username/stats` - User statistics

### Admin
- `GET /api/backfill/:owner/:name` - Check backfill status & last sync
- `POST /api/backfill/:owner/:name?max_days=N&force=bool` - Trigger backfill

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

### Achievements (Defined)

| ID | Name | Description | XP | Rarity |
|----|------|-------------|-----|--------|
| `first_review` | First Blood | Submit your first review | 50 | Common |
| `review_10` | Getting Started | Submit 10 reviews | 100 | Common |
| `review_50` | Reviewer | Submit 50 reviews | 250 | Uncommon |
| `review_100` | Centurion | Submit 100 reviews | 500 | Rare |
| `speed_demon` | Speed Demon | Review a PR within 1 hour (10x) | 200 | Uncommon |
| `night_owl` | Night Owl | Submit 10 reviews after midnight | 150 | Uncommon |
| `review_streak_7` | On Fire | Review PRs 7 days in a row | 300 | Rare |

## Deployment

- **Platform**: Fly.io (review-royale.fly.dev)
- **Database**: Fly Postgres
- **CI/CD**: GitHub Actions (auto-deploy on push to main)
- **Secrets**: DATABASE_URL, GITHUB_TOKEN, FLY_API_TOKEN

## Milestones

### M1: Core Backend ✅
- [x] Database schema
- [x] GitHub API client
- [x] Backfill service
- [x] Background sync service
- [x] XP calculation
- [x] REST API

### M2: Frontend ✅
- [x] Leaderboard page with dark theme
- [x] Period selectors (week/month/all)
- [x] Stats summary (reviewers, reviews, comments, first reviews)
- [x] Level badges with colors
- [x] Last synced timestamp

### M3: Deployment ✅
- [x] Docker + Fly.io
- [x] CI/CD pipeline
- [x] Production database
- [x] Backfill sigp/lighthouse (365 days)

### M4: Polish (Current)
- [x] Track comment counts per review
- [x] Track first reviews (who reviewed first)
- [x] Sort leaderboard by XP (not review count)
- [x] Pre-push hook for cargo fmt
- [ ] Add "PRs reviewed" distinct count (unique PRs vs total reviews)
- [ ] Test coverage
- [ ] Error handling improvements

### M5: Review Quality Analysis
- [ ] Store inline review comments (new `review_comments` table)
- [ ] AI categorization (cosmetic/logic/structural/nit/question)
- [ ] Quality score per comment
- [ ] Quality-weighted XP bonuses

### M6: Discord Bot
- [ ] Leaderboard command
- [ ] Weekly digest
- [ ] Achievement notifications
- [ ] Roast command

### M7: Advanced Features
- [ ] Achievement unlock logic
- [ ] Seasons (monthly/quarterly resets)
- [ ] Team leaderboards
- [ ] User profile pages
- [ ] Filter bots from leaderboard
