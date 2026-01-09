-- Users table for authentication and member management
CREATE TABLE users (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    username    TEXT NOT NULL UNIQUE,
    password    TEXT NOT NULL,           -- Argon2 hash
    nickname    TEXT NOT NULL,
    email       TEXT,
    role        TEXT NOT NULL DEFAULT 'member',  -- 'sysop', 'subop', 'member'
    profile     TEXT,                    -- Self-introduction
    terminal    TEXT NOT NULL DEFAULT 'standard',  -- 'standard', 'c64', 'c64_ansi'
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    last_login  TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_role ON users(role);
