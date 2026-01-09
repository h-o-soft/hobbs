-- RSS read positions table for tracking user's last read position per feed
CREATE TABLE rss_read_positions (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    feed_id             BIGINT NOT NULL REFERENCES rss_feeds(id) ON DELETE CASCADE,
    last_read_item_id   BIGINT REFERENCES rss_items(id) ON DELETE SET NULL,
    last_read_at        TEXT NOT NULL DEFAULT TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS'),
    UNIQUE(user_id, feed_id)
);

CREATE INDEX idx_rss_read_positions_user ON rss_read_positions(user_id);
CREATE INDEX idx_rss_read_positions_feed ON rss_read_positions(feed_id);
