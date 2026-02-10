-- Add review_sessions column to track actual session count (not raw events)
ALTER TABLE users ADD COLUMN IF NOT EXISTS review_sessions INTEGER NOT NULL DEFAULT 0;

-- Index for leaderboard sorting
CREATE INDEX IF NOT EXISTS idx_users_sessions ON users(review_sessions DESC);
