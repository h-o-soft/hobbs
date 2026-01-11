-- One-time tokens for secure URL-based authentication (WebSocket, file downloads)
-- These tokens are short-lived and can only be used once
CREATE TABLE one_time_tokens (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token           TEXT NOT NULL UNIQUE,
    purpose         TEXT NOT NULL,  -- 'websocket' or 'download'
    target_id       BIGINT,         -- Optional: file_id for downloads
    expires_at      TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS'),
    used_at         TEXT            -- NULL if not used yet
);

CREATE INDEX idx_one_time_tokens_token ON one_time_tokens(token);
CREATE INDEX idx_one_time_tokens_user_id ON one_time_tokens(user_id);
CREATE INDEX idx_one_time_tokens_expires_at ON one_time_tokens(expires_at);
