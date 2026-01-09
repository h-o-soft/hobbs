-- RSS read positions table for tracking user's last read position per feed
CREATE TABLE rss_read_positions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id             INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    feed_id             INTEGER NOT NULL REFERENCES rss_feeds(id) ON DELETE CASCADE,
    last_read_item_id   INTEGER REFERENCES rss_items(id) ON DELETE SET NULL,
    last_read_at        TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, feed_id)
);

CREATE INDEX idx_rss_read_positions_user ON rss_read_positions(user_id);
CREATE INDEX idx_rss_read_positions_feed ON rss_read_positions(feed_id);
