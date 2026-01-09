-- Recreate rss_feeds table with per-user unique constraint
-- Each user can have their own subscription to the same feed URL
CREATE TABLE rss_feeds_new (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    url             TEXT NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    site_url        TEXT,
    last_fetched_at TEXT,
    last_item_at    TEXT,
    fetch_interval  INTEGER NOT NULL DEFAULT 3600,
    is_active       INTEGER NOT NULL DEFAULT 1,
    error_count     INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    created_by      INTEGER NOT NULL REFERENCES users(id),
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(created_by, url)
);

-- Copy existing data
INSERT INTO rss_feeds_new SELECT * FROM rss_feeds;

-- Drop old table
DROP TABLE rss_feeds;

-- Rename new table
ALTER TABLE rss_feeds_new RENAME TO rss_feeds;

-- Recreate indexes
CREATE INDEX idx_rss_feeds_is_active ON rss_feeds(is_active);
CREATE INDEX idx_rss_feeds_last_fetched ON rss_feeds(last_fetched_at);
CREATE INDEX idx_rss_feeds_created_by ON rss_feeds(created_by);
