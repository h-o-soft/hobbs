-- Script data table for persistent key-value storage
CREATE TABLE script_data (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    script_id   INTEGER NOT NULL REFERENCES scripts(id) ON DELETE CASCADE,
    user_id     INTEGER REFERENCES users(id) ON DELETE CASCADE,  -- NULL = global data, non-NULL = per-user data
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,      -- JSON-encoded value
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(script_id, user_id, key)
);

CREATE INDEX idx_script_data_script ON script_data(script_id);
CREATE INDEX idx_script_data_user ON script_data(user_id);
CREATE INDEX idx_script_data_script_user ON script_data(script_id, user_id);
