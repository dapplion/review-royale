-- Initial schema for Review Royale

-- Repositories
CREATE TABLE IF NOT EXISTS repositories (
    id UUID PRIMARY KEY,
    github_id BIGINT NOT NULL UNIQUE,
    owner TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_repos_owner_name ON repositories(owner, name);

-- Users
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    github_id BIGINT NOT NULL UNIQUE,
    login TEXT NOT NULL,
    avatar_url TEXT,
    xp BIGINT NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_login ON users(login);
CREATE INDEX IF NOT EXISTS idx_users_xp ON users(xp DESC);

-- Pull Requests
CREATE TABLE IF NOT EXISTS pull_requests (
    id UUID PRIMARY KEY,
    repo_id UUID NOT NULL REFERENCES repositories(id),
    github_id BIGINT NOT NULL UNIQUE,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    author_id UUID NOT NULL REFERENCES users(id),
    state TEXT NOT NULL DEFAULT 'open',
    created_at TIMESTAMPTZ NOT NULL,
    first_review_at TIMESTAMPTZ,
    merged_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_prs_repo ON pull_requests(repo_id);
CREATE INDEX IF NOT EXISTS idx_prs_author ON pull_requests(author_id);
CREATE INDEX IF NOT EXISTS idx_prs_created ON pull_requests(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_prs_state ON pull_requests(state);

-- Reviews
CREATE TABLE IF NOT EXISTS reviews (
    id UUID PRIMARY KEY,
    pr_id UUID NOT NULL REFERENCES pull_requests(id),
    reviewer_id UUID NOT NULL REFERENCES users(id),
    github_id BIGINT NOT NULL UNIQUE,
    state TEXT NOT NULL,
    body TEXT,
    comments_count INTEGER NOT NULL DEFAULT 0,
    submitted_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reviews_pr ON reviews(pr_id);
CREATE INDEX IF NOT EXISTS idx_reviews_reviewer ON reviews(reviewer_id);
CREATE INDEX IF NOT EXISTS idx_reviews_submitted ON reviews(submitted_at DESC);

-- Achievements
CREATE TABLE IF NOT EXISTS achievements (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    emoji TEXT NOT NULL,
    xp_reward INTEGER NOT NULL DEFAULT 0,
    rarity TEXT NOT NULL DEFAULT 'common'
);

-- User Achievements
CREATE TABLE IF NOT EXISTS user_achievements (
    user_id UUID NOT NULL REFERENCES users(id),
    achievement_id TEXT NOT NULL REFERENCES achievements(id),
    unlocked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_user_achievements_user ON user_achievements(user_id);
CREATE INDEX IF NOT EXISTS idx_user_achievements_unlocked ON user_achievements(unlocked_at DESC);

-- Seasons
CREATE TABLE IF NOT EXISTS seasons (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    number INTEGER NOT NULL UNIQUE,
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL
);

-- Season Scores
CREATE TABLE IF NOT EXISTS season_scores (
    season_id UUID NOT NULL REFERENCES seasons(id),
    user_id UUID NOT NULL REFERENCES users(id),
    score BIGINT NOT NULL DEFAULT 0,
    reviews_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (season_id, user_id)
);

-- Insert default achievements
INSERT INTO achievements (id, name, description, emoji, xp_reward, rarity) VALUES
    ('first_review', 'First Blood', 'Submit your first review', '🩸', 50, 'common'),
    ('review_10', 'Getting Started', 'Submit 10 reviews', '📝', 100, 'common'),
    ('review_50', 'Reviewer', 'Submit 50 reviews', '👁️', 250, 'uncommon'),
    ('review_100', 'Centurion', 'Submit 100 reviews', '💯', 500, 'rare'),
    ('speed_demon', 'Speed Demon', 'Review a PR within 1 hour (10 times)', '⚡', 200, 'uncommon'),
    ('night_owl', 'Night Owl', 'Submit 10 reviews after midnight', '🦉', 150, 'uncommon'),
    ('review_streak_7', 'On Fire', 'Review PRs 7 days in a row', '🔥', 300, 'rare')
ON CONFLICT (id) DO NOTHING;
