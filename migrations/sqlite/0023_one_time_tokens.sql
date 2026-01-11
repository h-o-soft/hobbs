-- One-time tokens for secure URL-based authentication (WebSocket, file downloads)
-- These tokens are short-lived and can only be used once
CREATE TABLE one_time_tokens (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token           TEXT NOT NULL UNIQUE,
    purpose         TEXT NOT NULL,  -- 'websocket' or 'download'
    target_id       INTEGER,        -- Optional: file_id for downloads
    expires_at      TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    used_at         TEXT            -- NULL if not used yet
);

CREATE INDEX idx_one_time_tokens_token ON one_time_tokens(token);
CREATE INDEX idx_one_time_tokens_user_id ON one_time_tokens(user_id);
CREATE INDEX idx_one_time_tokens_expires_at ON one_time_tokens(expires_at);
