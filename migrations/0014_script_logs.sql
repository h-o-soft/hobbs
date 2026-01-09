-- Script execution logs for tracking usage and debugging
CREATE TABLE script_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    script_id       INTEGER NOT NULL REFERENCES scripts(id) ON DELETE CASCADE,
    user_id         INTEGER REFERENCES users(id) ON DELETE SET NULL,
    executed_at     TEXT NOT NULL DEFAULT (datetime('now')),
    execution_ms    INTEGER NOT NULL,       -- Execution time in milliseconds
    success         INTEGER NOT NULL,       -- 1 = success, 0 = error
    error_message   TEXT                    -- Error message if success = 0
);

CREATE INDEX idx_script_logs_script ON script_logs(script_id);
CREATE INDEX idx_script_logs_user ON script_logs(user_id);
CREATE INDEX idx_script_logs_executed_at ON script_logs(executed_at);
