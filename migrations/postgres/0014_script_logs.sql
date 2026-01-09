-- Script execution logs for tracking usage and debugging
CREATE TABLE script_logs (
    id              BIGSERIAL PRIMARY KEY,
    script_id       BIGINT NOT NULL REFERENCES scripts(id) ON DELETE CASCADE,
    user_id         BIGINT REFERENCES users(id) ON DELETE SET NULL,
    executed_at     TIMESTAMP NOT NULL DEFAULT NOW(),
    execution_ms    INTEGER NOT NULL,       -- Execution time in milliseconds
    success         BOOLEAN NOT NULL,       -- TRUE = success, FALSE = error
    error_message   TEXT                    -- Error message if success = FALSE
);

CREATE INDEX idx_script_logs_script ON script_logs(script_id);
CREATE INDEX idx_script_logs_user ON script_logs(user_id);
CREATE INDEX idx_script_logs_executed_at ON script_logs(executed_at);
