-- Add sync tracking to repositories

ALTER TABLE repositories ADD COLUMN IF NOT EXISTS last_synced_at TIMESTAMPTZ;
ALTER TABLE repositories ADD COLUMN IF NOT EXISTS sync_cursor TEXT;

CREATE INDEX IF NOT EXISTS idx_repos_last_synced ON repositories(last_synced_at);
