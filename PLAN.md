# Review Royale - Development Plan

> Gamified PR review analytics for GitHub teams. Make code review competitive, fun, and visible.

## Vision

Transform PR reviews from a chore into a sport. Points, achievements, leaderboards, seasons, trash talk. A full gamification layer on top of GitHub's review system.

---

## Phase 1: Foundation (Week 1)

### 1.1 Project Setup
- [x] Create repo structure
- [ ] Set up Rust workspace with crates
- [ ] Docker Compose for local dev (Postgres, Redis)
- [ ] Basic CI (cargo check, fmt, clippy, test)
- [ ] README with project overview

### 1.2 GitHub App
- [ ] Register GitHub App on github.com
- [ ] Implement webhook receiver (Axum)
- [ ] Handle webhook signature verification
- [ ] Parse relevant events:
  - `pull_request` (opened, closed, merged, reopened)
  - `pull_request_review` (submitted, edited, dismissed)
  - `pull_request_review_comment` (created, edited, deleted)
  - `issue_comment` (for PR comments)
  - `check_run` / `check_suite` (CI status)
  - `push` (commits to PR branch)

### 1.3 Data Model & Storage
- [ ] Design PostgreSQL schema
- [ ] Set up migrations (sqlx or refinery)
- [ ] Core tables:
  - `repositories` - tracked repos
  - `users` - GitHub users (denormalized from events)
  - `pull_requests` - PR metadata + timestamps
  - `reviews` - review events with timing
  - `review_comments` - individual comments
  - `ci_runs` - CI status changes
  - `metrics_daily` - pre-aggregated daily stats
  - `achievements` - achievement definitions
  - `user_achievements` - unlocked achievements
  - `seasons` - season definitions
  - `season_scores` - per-user per-season scores

### 1.4 Event Processing
- [ ] Event queue (Redis streams or in-process for MVP)
- [ ] Processor that:
  - Stores raw events
  - Updates PR state machine
  - Computes real-time metrics
  - Checks achievement triggers

---

## Phase 2: Metrics Engine (Week 1-2)

### Core Metrics

| Metric | Computation | Storage |
|--------|-------------|---------|
| **Time to First Review** | `first_review.created_at - pr.created_at` | Per PR, aggregated per user |
| **Review Response Time** | Time between review request and review | Per reviewer per PR |
| **Author Turnaround** | Time for author to respond after review | Per author per PR |
| **CI Fix Time** | Time from CI failure to passing after review | Per PR |
| **Review Depth** | Comments count, files reviewed, suggestions | Per review |
| **Review Volume** | Reviews submitted per time period | Daily/weekly/monthly per user |
| **Stale PR Detection** | PRs with no activity > threshold | Continuous |
| **Review Coverage** | % of PRs user reviewed vs total | Per user per period |

### Derived Scores

```
XP = (reviews * 10) 
   + (first_reviews * 5)           // bonus for being first
   + (fast_reviews * 3)            // < 4 hours
   + (deep_reviews * depth_score)  // based on comments/suggestions
   + (achievement_bonuses)
```

### Aggregation Jobs
- [ ] Hourly: Update leaderboard caches
- [ ] Daily: Compute daily rollups, check daily achievements
- [ ] Weekly: Weekly digest data, streak checks
- [ ] Monthly: Season standings, monthly achievements

---

## Phase 3: API (Week 2)

### REST Endpoints

```
GET  /api/health
GET  /api/repos
GET  /api/repos/:owner/:repo/stats
GET  /api/repos/:owner/:repo/prs
GET  /api/repos/:owner/:repo/prs/:number
GET  /api/repos/:owner/:repo/leaderboard
GET  /api/users/:username
GET  /api/users/:username/stats
GET  /api/users/:username/achievements
GET  /api/users/:username/reviews
GET  /api/seasons
GET  /api/seasons/:id/leaderboard
GET  /api/achievements
```

### WebSocket
- Real-time leaderboard updates
- Live PR activity feed
- Achievement unlock notifications

### Auth
- GitHub OAuth for users viewing their own detailed stats
- Public endpoints for leaderboards (no auth)

---

## Phase 4: Frontend (Week 2-3)

### Tech Stack
- SvelteKit (fast, fun, good DX)
- TailwindCSS
- Chart.js or Recharts for visualizations
- Deployed to Vercel/Cloudflare Pages

### Pages

1. **Home / Leaderboard**
   - Current season standings
   - Top reviewers (week/month/all-time)
   - Recent activity feed
   - Stale PR wall of shame

2. **User Profile**
   - Avatar, stats, level, XP
   - Achievement showcase
   - Review history
   - Personal records
   - Rivalries (most reviewed/reviewed-by)

3. **PR Detail**
   - Timeline visualization
   - Time metrics breakdown
   - Participants and their contribution

4. **Repo Dashboard**
   - Overall health metrics
   - Review velocity trends
   - Team comparison
   - Bottleneck identification

5. **Achievements**
   - All achievements (locked/unlocked)
   - Rarity stats
   - Recent unlocks across team

6. **Season Archive**
   - Past seasons
   - Historical leaderboards
   - Season MVPs

---

## Phase 5: Bot (Week 3)

### Discord Bot

**Commands:**
- `/leaderboard` - Current standings
- `/stats @user` - User stats
- `/pr <number>` - PR status
- `/stale` - List stale PRs
- `/achievements` - Recent unlocks
- `/roast @user` - Generate playful roast based on stats

**Automated Posts:**
- Weekly digest (Monday morning)
- Achievement unlocks (as they happen)
- Stale PR alerts (configurable threshold)
- Season end ceremony

**Personality:**
- Snarky, playful, competitive
- Uses memes and emoji liberally
- Roasts slow reviewers (gently)
- Celebrates achievements enthusiastically

### GitHub Bot (stretch)
- Comment on PRs with review stats
- Label PRs based on staleness
- Auto-request reviewers based on load balancing

---

## Phase 6: Advanced Features (Week 4+)

### AI-Powered Review Quality
- Use LLM to analyze review comments
- Score: superficial vs substantive
- Detect: nitpicks vs architectural feedback
- Track: suggestions that led to changes

### Meme Generation
- DALL-E / Stable Diffusion integration
- Generate memes for:
  - Stale PRs
  - Achievement unlocks
  - Weekly MVPs
  - Rivalry matchups

### Notifications
- Email digests (optional)
- Slack integration
- Custom webhooks

### Multi-Repo Support
- Org-wide leaderboards
- Cross-repo achievements
- Team groupings

---

## Technical Decisions

### Why Rust?
- Lighthouse team knows it
- Fast, reliable, good for long-running services
- Strong typing catches bugs early
- Axum is excellent for web services

### Why PostgreSQL?
- Mature, reliable
- Great for relational data + JSON
- Good aggregation support
- Easy to self-host or use managed

### Why Redis?
- Fast caching for leaderboards
- Pub/sub for real-time features
- Simple queue for event processing
- Session storage

### Why SvelteKit?
- Less boilerplate than React
- Great performance
- Fun to write
- Good for dashboards

---

## Repo Structure

```
review-royale/
├── Cargo.toml                 # Workspace manifest
├── Cargo.lock
├── README.md
├── PLAN.md                    # This file
├── LICENSE
├── .github/
│   └── workflows/
│       └── ci.yml
├── docker-compose.yml         # Local dev stack
├── crates/
│   ├── common/                # Shared types, utils
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models.rs      # Domain models
│   │       ├── config.rs      # Configuration
│   │       └── error.rs       # Error types
│   ├── db/                    # Database layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── migrations/
│   │       ├── repos.rs
│   │       ├── users.rs
│   │       ├── prs.rs
│   │       ├── reviews.rs
│   │       └── achievements.rs
│   ├── github/                # GitHub API & webhooks
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── webhooks.rs    # Webhook parsing
│   │       ├── events.rs      # Event types
│   │       ├── client.rs      # GitHub API client
│   │       └── verify.rs      # Signature verification
│   ├── processor/             # Event processing & metrics
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── metrics.rs     # Metric computation
│   │       ├── achievements.rs # Achievement checks
│   │       ├── scores.rs      # XP/scoring
│   │       └── aggregator.rs  # Rollup jobs
│   ├── api/                   # HTTP API server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/
│   │       │   ├── mod.rs
│   │       │   ├── health.rs
│   │       │   ├── repos.rs
│   │       │   ├── users.rs
│   │       │   ├── leaderboard.rs
│   │       │   └── webhooks.rs
│   │       ├── auth.rs
│   │       └── ws.rs          # WebSocket handler
│   └── bot/                   # Discord bot
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           ├── scheduled.rs   # Scheduled posts
│           └── personality.rs # Message generation
├── web/                       # SvelteKit frontend
│   ├── package.json
│   ├── svelte.config.js
│   ├── src/
│   │   ├── routes/
│   │   ├── lib/
│   │   └── app.html
│   └── static/
└── migrations/                # SQL migrations
    ├── 001_initial.sql
    └── ...
```

---

## Milestones

### M1: Webhook Ingestion (3 days)
- GitHub App receiving events
- Events stored in PostgreSQL
- Basic health endpoint
- Deployed somewhere (fly.io)

### M2: Basic Metrics (3 days)
- Time to first review computed
- Review count per user
- Simple leaderboard endpoint

### M3: Frontend MVP (4 days)
- Leaderboard page
- User profile with basic stats
- Deployed to Vercel

### M4: Bot MVP (3 days)
- Discord bot running
- `/leaderboard` command
- Weekly digest posting

### M5: Achievements (3 days)
- Achievement system implemented
- 10 initial achievements
- Unlock notifications

### M6: Polish & Launch (ongoing)
- More metrics
- More achievements
- Better visualizations
- Team feedback integration

---

## Open Questions

1. **Hosting**: Fly.io? Railway? Self-hosted on a VPS?
2. **Domain**: review-royale.dev? Something else?
3. **Discord server**: Use Lighthouse's existing or dedicated?
4. **Initial repos**: Start with just sigp/lighthouse?
5. **Historical data**: Backfill from GitHub API or start fresh?

---

## Let's Ship It 🚀

Starting with M1: Webhook ingestion. The foundation everything else builds on.
