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
│  │   GitHub     │───▶│     Sync     │───▶│   Database   │  │
│  │     API      │    │   Service    │    │  (Postgres)  │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│                       │ incremental │            │          │
│                       │ from cursor │            ▼          │
│                       │ every N hr  │    ┌──────────────┐  │
│                       └──────────────┘    │  Processor   │  │
│                                          │  (Sessions)  │  │
│  ┌──────────────┐    ┌──────────────┐    └──────────────┘  │
│  │ Mattermost   │◀───│     API      │◀──────────┘          │
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

### Sync Strategy (Simplified)

No separate "backfill" concept - just **one sync operation**:

```
sync(repo, from: last_synced_at || start_date, to: now)
```

- **First run**: `from = 365 days ago` (no cursor yet)
- **Subsequent runs**: `from = last_synced_at`
- **After success**: `last_synced_at = now`

Incremental, stateful, simple.

## Crates

| Crate | Purpose |
|-------|---------|
| `common` | Shared types, config, errors |
| `db` | PostgreSQL queries and migrations |
| `github` | GitHub API client |
| `processor` | Backfill, sync, XP calculation, achievements |
| `api` | REST API server (Axum) + static frontend |
| `bot` | Mattermost bot — skeleton only |

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

### What is a "Review"?

**A review = one session of work reviewing a specific version of code.**

Individual GitHub review events (comments, approvals) are grouped into **review sessions** based on:

#### A review session ENDS when:
1. **Author pushes new commits** → new code to review = new work
2. **24-hour gap** between reviewer's comments → came back fresh = new session

#### What does NOT create a new review:
- Multiple comments within same session (even if 30 min apart)
- Quick back-and-forth in same hour
- Re-requesting changes on same commits

#### Minimum threshold for a review to count:
- At least **1 substantive comment** (>20 chars) OR **state change** (approved/changes_requested)
- **Rubber stamp rejections**: Pure "approved" with 0 comments and <1 min review time = **no credit**

### XP Formula

**Per review session:**
- **Base**: 10 XP (one meaningful review session)
- **Comments**: +5 XP per substantive comment (>20 chars)
- **Fast review**: +10 XP if reviewed <1 hour after commits pushed
- **Thorough**: +5 XP if >5 comments in session
- **Deep review**: +10 XP if >10 comments in session

**No "first reviewer" bonus** — we don't reward racing. Multiple reviewers can all get the fast bonus.

**Example (without quality data):**
- Author pushes commits at 10:00 AM
- Alice reviews at 10:30 AM with 3 comments → 10 base + 15 comments + 10 fast = **35 XP**
- Bob reviews at 10:45 AM with 7 comments → 10 base + 35 comments + 10 fast + 5 thorough = **60 XP**

### Quality-Weighted XP (M5)

When comments are AI-categorized, XP is weighted by quality:

**Quality tiers (per comment):**
- Low quality (1-3): +2 XP (brief, superficial)
- Medium quality (4-6): +5 XP (standard)
- High quality (7-10): +8 XP (detailed, insightful)

**Category bonuses (on top of quality XP):**
- `logic`: +3 XP (catches bugs = most valuable)
- `structural`: +2 XP (design improvements)
- `cosmetic`/`nit`/`question`: +0 XP

**Example (with quality data):**
- 5 high-quality comments, 2 catching logic bugs, 1 structural:
- 10 base + 5×8 quality + 2×3 logic + 1×2 structural = 10 + 40 + 6 + 2 = **58 XP**

Uncategorized comments use the flat +5 XP rate.

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

**Milestone achievements:**
| ID | Name | Description | XP | Rarity |
|----|------|-------------|-----|--------|
| `first_review` | First Blood | Submit your first review | 50 | Common |
| `review_10` | Getting Started | Submit 10 reviews | 100 | Common |
| `review_50` | Reviewer | Submit 50 reviews | 250 | Uncommon |
| `review_100` | Centurion | Submit 100 reviews | 500 | Rare |
| `review_500` | Gatekeeper | Submit 500 reviews | 1000 | Epic |
| `review_1000` | Code Guardian | Submit 1000 reviews | 2000 | Legendary |

**Speed achievements:**
| ID | Name | Description | XP | Rarity |
|----|------|-------------|-----|--------|
| `speed_demon` | Speed Demon | Review within 1h of PR creation (10x) | 200 | Uncommon |
| `first_responder` | First Responder | Be first reviewer on a PR (25x) | 300 | Rare |

**Quality achievements:**
| ID | Name | Description | XP | Rarity |
|----|------|-------------|-----|--------|
| `nitpicker` | Nitpicker | Leave 50 comments marked as nits | 100 | Common |
| `bug_hunter` | Bug Hunter | Catch 10 bugs in reviews | 400 | Rare |
| `thorough` | Deep Dive | Leave 10+ comments in a single review (5x) | 250 | Uncommon |

**Streak & special achievements:**
| ID | Name | Description | XP | Rarity |
|----|------|-------------|-----|--------|
| `review_streak_7` | On Fire | Review PRs 7 days in a row | 300 | Rare |
| `review_streak_30` | Unstoppable | Review PRs 30 days in a row | 750 | Epic |
| `comeback_kid` | Comeback Kid | Return after 30+ day absence | 150 | Uncommon |
| `review_rampage` | Review Rampage | Review 5 PRs in a single day | 200 | Uncommon |
| `the_closer` | The Closer | Your approval led to 10 merges | 350 | Rare |
| `helpful` | Helpful | Get 10 "thanks" replies to comments | 200 | Uncommon |

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

### M4: Polish ✅
- [x] Track comment counts per review
- [x] Track first reviews (who reviewed first)
- [x] Sort leaderboard by XP (not review count)
- [x] Pre-push hook for cargo fmt
- [x] Footer with XP calculation formula/specs
- [x] Data range info: "Last synced X, data from Y"
- [x] Individual contributor view (click username → profile page)
  - [x] Score breakdown (why XP is X)
  - [x] XP over time chart (per day/week)
  - [x] Recent reviews list
- [x] Add "PRs reviewed" distinct count (unique PRs vs total reviews)
- [x] **Multi-repo support**: Generalize to any GitHub repo
  - [x] URL structure: `/:org/:repo` → repo leaderboard
  - [x] User view: `/:org/:repo/user/:username` → user profile scoped to that repo
  - [x] Global leaderboard at `/` (all repos combined)
  - [x] Repo selector/switcher
  - [x] **SECURITY**: Access control by org:
    - `sigp/*` repos → show leaderboard (allowed)
    - Private repos → 404 / show nothing (prevent leaking internal repos)
    - Other orgs (public) → "Request access" page with link to Lion's Twitter
    - Goal: open to all orgs eventually, closed for now for safety
- [x] Test coverage
- [x] Error handling improvements

### M5: Review Quality Analysis
- [x] Store inline review comments (new `review_comments` table)
- [x] AI categorization (cosmetic/logic/structural/nit/question)
- [x] Quality score per comment (1-10 scale)
- [x] Quality-weighted XP bonuses

### M6: Discord Bot
- [x] Leaderboard command (with period filter: week/month/all)
- [x] Weekly digest (`!rr digest` command)
- [x] Achievement notifications (background loop + DISCORD_NOTIFICATION_CHANNEL env var)
- [x] Roast command

### M7: Advanced Features
- [x] Achievement unlock logic (in processor/achievements.rs, runs on recalculate)
- [x] Seasons (monthly/quarterly resets) - DB module + API endpoints
- [x] Team leaderboards (DB + API: GET /api/teams, POST /api/teams, GET /api/teams/leaderboard, team CRUD)
- [x] User profile pages (via M4 Individual contributor view)
- [x] Filter bots from leaderboard

### M8: UI Test Coverage ✅
- [x] Install Playwright for browser automation
- [x] Screenshot-based UI tests for core views:
  - [x] Global leaderboard (desktop + mobile)
  - [x] User profile page
  - [x] Period filter switching
  - [x] Repo-scoped leaderboard
- [x] AI vision verification of screenshots (`npm run verify` uses Claude Vision)
- [x] CI integration for visual regression

### M9: Polish & Generalization
- [x] Remove Night Owl achievement (can't know contributor timezones)
- [x] Add creative/fun achievements
- [ ] Landing page redesign:
  - [ ] Root URL shows intro/marketing landing page
  - [ ] List of tracked repositories with stats
  - [ ] Link to GitHub source
  - [ ] "Add your repo" CTA / request access flow

### M10: Open PRs Dashboard
- [ ] Show total open PRs count on leaderboard page (clickable)
- [ ] PR list view with rich stats:
  - [ ] PR title, number, author, created date
  - [ ] Difficulty estimate (lines changed, files touched)
  - [ ] Internal vs external contributor flag
  - [ ] Review status (approved/changes_requested/pending/none)
  - [ ] Time waiting for review
  - [ ] Active reviewers list
- [ ] Sort/filter options (by age, difficulty, status)

## Development Workflow: AI-Assisted UI Iteration

Visual feedback loop for frontend development:

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Render    │───▶│ Screenshot  │───▶│   Analyze   │
│ (headless)  │    │  (browser)  │    │  (vision)   │
└─────────────┘    └─────────────┘    └─────────────┘
       ▲                                     │
       │                                     ▼
       │           ┌─────────────┐    ┌─────────────┐
       └───────────│   Deploy    │◀───│    Edit     │
                   │  (fly.io)   │    │ (HTML/CSS)  │
                   └─────────────┘    └─────────────┘
```

### The Loop

1. **Render** — Playwright loads the live page (or local dev server)
2. **Screenshot** — Capture viewport at target resolution(s)
3. **Analyze** — Vision model critiques layout, spacing, colors, UX
4. **Edit** — Modify frontend code based on feedback
5. **Deploy** — Push changes, repeat

### Use Cases

- **Responsive checks**: Screenshot at 375px, 768px, 1440px widths
- **Accessibility audit**: Vision model spots contrast issues, missing focus states
- **Design matching**: "Make it look more like [reference]"
- **Visual regression**: Compare before/after screenshots
- **Polish passes**: Iterate on spacing, alignment, visual hierarchy

### Commands

```bash
# Screenshot current production
browser screenshot --url https://review-royale.fly.dev --width 1440

# Mobile viewport
browser screenshot --url https://review-royale.fly.dev --width 375

# Full page capture
browser screenshot --url https://review-royale.fly.dev --fullPage
```

### Limitations

- ~30-60s per iteration (render + analyze + edit)
- Vision models can miss subtle CSS issues
- Best for polish, not structural changes

## TODOs Before Launch

### XP Recalculation
- [x] Add "reset and recalculate all XP" function
  - Zeros all user XP
  - Recomputes from all reviews in database grouped into sessions
  - Use when XP formula changes or before production launch
  - Available at `POST /api/recalculate`
- [x] Run full DB reset + recalculate with session-based formula ✅ (2026-02-07)
  - Results: jimmygchen dropped from 19.9K XP (#1) → 700 XP (#3)
  - michaelsproul now #1 with 925 XP
  - Formula correctly rewards depth over comment spam
