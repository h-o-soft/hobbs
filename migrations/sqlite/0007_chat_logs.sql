-- Chat logs table for storing chat message history
CREATE TABLE chat_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id         TEXT NOT NULL,
    user_id         INTEGER REFERENCES users(id) ON DELETE SET NULL,  -- NULL for system messages
    sender_name     TEXT NOT NULL,                                     -- Display name at send time
    message_type    TEXT NOT NULL,                                     -- 'chat', 'action', 'system', 'join', 'leave'
    content         TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_chat_logs_room_id ON chat_logs(room_id);
CREATE INDEX idx_chat_logs_created_at ON chat_logs(created_at);
CREATE INDEX idx_chat_logs_room_created ON chat_logs(room_id, created_at);
