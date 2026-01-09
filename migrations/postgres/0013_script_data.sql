-- Script data table for persistent key-value storage
CREATE TABLE script_data (
    id          BIGSERIAL PRIMARY KEY,
    script_id   BIGINT NOT NULL REFERENCES scripts(id) ON DELETE CASCADE,
    user_id     BIGINT REFERENCES users(id) ON DELETE CASCADE,  -- NULL = global data, non-NULL = per-user data
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,      -- JSON-encoded value
    updated_at  TEXT NOT NULL DEFAULT TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS'),
    UNIQUE(script_id, user_id, key)
);

CREATE INDEX idx_script_data_script ON script_data(script_id);
CREATE INDEX idx_script_data_user ON script_data(user_id);
CREATE INDEX idx_script_data_script_user ON script_data(script_id, user_id);
