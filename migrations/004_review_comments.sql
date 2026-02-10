-- Store inline review comments for AI categorization (M5)

CREATE TABLE IF NOT EXISTS review_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id UUID REFERENCES reviews(id) ON DELETE CASCADE,
    pr_id UUID NOT NULL REFERENCES pull_requests(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    github_id BIGINT NOT NULL UNIQUE,
    body TEXT NOT NULL,
    path TEXT,                    -- file path commented on
    diff_hunk TEXT,               -- code context
    line INTEGER,                 -- line number in diff
    in_reply_to_id BIGINT,        -- parent comment for threads
    created_at TIMESTAMPTZ NOT NULL,
    -- M5 future: AI categorization
    category TEXT,                -- cosmetic/logic/structural/nit/question
    quality_score INTEGER,        -- 0-100 quality rating
    CONSTRAINT fk_reply_to FOREIGN KEY (in_reply_to_id) 
        REFERENCES review_comments(github_id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_review_comments_review ON review_comments(review_id);
CREATE INDEX IF NOT EXISTS idx_review_comments_pr ON review_comments(pr_id);
CREATE INDEX IF NOT EXISTS idx_review_comments_user ON review_comments(user_id);
CREATE INDEX IF NOT EXISTS idx_review_comments_created ON review_comments(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_review_comments_category ON review_comments(category) WHERE category IS NOT NULL;
