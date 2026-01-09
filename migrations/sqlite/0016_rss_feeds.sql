-- RSS feeds table for external feed subscriptions
CREATE TABLE rss_feeds (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    url             TEXT NOT NULL UNIQUE,
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
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_rss_feeds_is_active ON rss_feeds(is_active);
CREATE INDEX idx_rss_feeds_last_fetched ON rss_feeds(last_fetched_at);
