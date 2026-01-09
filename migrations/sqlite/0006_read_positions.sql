-- Read positions table for tracking user's last read position per board
CREATE TABLE read_positions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id             INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    board_id            INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    last_read_post_id   INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    last_read_at        TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, board_id)
);

CREATE INDEX idx_read_positions_user_id ON read_positions(user_id);
CREATE INDEX idx_read_positions_board_id ON read_positions(board_id);
