-- Add commits table to track PR commits for review session boundaries

CREATE TABLE IF NOT EXISTS commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pr_id UUID NOT NULL REFERENCES pull_requests(id) ON DELETE CASCADE,
    sha TEXT NOT NULL,
    author_id UUID REFERENCES users(id),
    committed_at TIMESTAMPTZ NOT NULL,
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(pr_id, sha)
);

CREATE INDEX IF NOT EXISTS idx_commits_pr ON commits(pr_id, committed_at DESC);
CREATE INDEX IF NOT EXISTS idx_commits_author ON commits(author_id);
