-- Recreate rss_feeds table with per-user unique constraint
-- Each user can have their own subscription to the same feed URL

-- Drop dependent tables first (they will be recreated by their migrations in a fresh install,
-- but for an existing database we need to handle the constraint change)
-- Note: In PostgreSQL we can use ALTER TABLE to add constraints, but since we need to
-- change the UNIQUE constraint, we recreate the table similar to SQLite approach

-- Create new table with updated constraint
CREATE TABLE rss_feeds_new (
    id              BIGSERIAL PRIMARY KEY,
    url             TEXT NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    site_url        TEXT,
    last_fetched_at TEXT,
    last_item_at    TEXT,
    fetch_interval  BIGINT NOT NULL DEFAULT 3600,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    error_count     INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    created_by      BIGINT NOT NULL REFERENCES users(id),
    created_at      TEXT NOT NULL DEFAULT TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS'),
    updated_at      TEXT NOT NULL DEFAULT TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS'),
    UNIQUE(created_by, url)
);

-- Copy existing data
INSERT INTO rss_feeds_new (id, url, title, description, site_url, last_fetched_at, last_item_at,
    fetch_interval, is_active, error_count, last_error, created_by, created_at, updated_at)
SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
    fetch_interval, is_active, error_count, last_error, created_by, created_at, updated_at
FROM rss_feeds;

-- Update sequence to match max id
SELECT setval('rss_feeds_new_id_seq', COALESCE((SELECT MAX(id) FROM rss_feeds_new), 0) + 1, false);

-- Drop old table (cascades to drop foreign key references)
DROP TABLE rss_feeds CASCADE;

-- Rename new table
ALTER TABLE rss_feeds_new RENAME TO rss_feeds;

-- Rename the sequence
ALTER SEQUENCE rss_feeds_new_id_seq RENAME TO rss_feeds_id_seq;

-- Recreate indexes
CREATE INDEX idx_rss_feeds_is_active ON rss_feeds(is_active);
CREATE INDEX idx_rss_feeds_last_fetched ON rss_feeds(last_fetched_at);
CREATE INDEX idx_rss_feeds_created_by ON rss_feeds(created_by);

-- Recreate foreign key constraints on dependent tables
-- Note: These tables should exist if running migrations in order
ALTER TABLE rss_items ADD CONSTRAINT rss_items_feed_id_fkey
    FOREIGN KEY (feed_id) REFERENCES rss_feeds(id) ON DELETE CASCADE;

ALTER TABLE rss_read_positions ADD CONSTRAINT rss_read_positions_feed_id_fkey
    FOREIGN KEY (feed_id) REFERENCES rss_feeds(id) ON DELETE CASCADE;
