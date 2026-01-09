-- RSS items table for storing fetched articles
CREATE TABLE rss_items (
    id              BIGSERIAL PRIMARY KEY,
    feed_id         BIGINT NOT NULL REFERENCES rss_feeds(id) ON DELETE CASCADE,
    guid            TEXT NOT NULL,
    title           TEXT NOT NULL,
    link            TEXT,
    description     TEXT,
    author          TEXT,
    published_at    TIMESTAMP,
    fetched_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(feed_id, guid)
);

CREATE INDEX idx_rss_items_feed_id ON rss_items(feed_id);
CREATE INDEX idx_rss_items_published_at ON rss_items(published_at);
CREATE INDEX idx_rss_items_fetched_at ON rss_items(fetched_at);
