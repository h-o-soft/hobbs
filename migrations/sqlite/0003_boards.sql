-- Boards table for bulletin board management
CREATE TABLE boards (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL UNIQUE,
    description     TEXT,
    board_type      TEXT NOT NULL DEFAULT 'thread',  -- 'thread' or 'flat'
    min_read_role   TEXT NOT NULL DEFAULT 'guest',   -- minimum role to read
    min_write_role  TEXT NOT NULL DEFAULT 'member',  -- minimum role to write
    sort_order      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_boards_sort_order ON boards(sort_order);
CREATE INDEX idx_boards_is_active ON boards(is_active);
