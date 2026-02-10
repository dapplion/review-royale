-- Add xp_earned column to reviews to enable period-filtered XP
ALTER TABLE reviews ADD COLUMN IF NOT EXISTS xp_earned INTEGER NOT NULL DEFAULT 0;

-- Index for summing XP by period
CREATE INDEX IF NOT EXISTS idx_reviews_xp_period ON reviews(reviewer_id, submitted_at, xp_earned);
