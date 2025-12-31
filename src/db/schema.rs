//! Database schema and migrations for HOBBS.
//!
//! This module contains all database migrations that will be applied
//! sequentially when the database is first opened or upgraded.

/// Database migrations.
///
/// Each migration is a SQL script that will be executed in order.
/// The schema_version table tracks which migrations have been applied.
pub const MIGRATIONS: &[&str] = &[
    // v1: Initial schema - users table
    r#"
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
"#,
    // v2: Add encoding column for character encoding preference
    r#"
ALTER TABLE users ADD COLUMN encoding TEXT NOT NULL DEFAULT 'shiftjis';
"#,
    // v3: Boards table for bulletin board management
    r#"
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
"#,
    // v4: Threads table for thread-based boards
    r#"
-- Threads table for thread-based boards
CREATE TABLE threads (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    board_id    INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    author_id   INTEGER NOT NULL REFERENCES users(id),
    post_count  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_threads_board_id ON threads(board_id);
CREATE INDEX idx_threads_author_id ON threads(author_id);
CREATE INDEX idx_threads_updated_at ON threads(updated_at);
"#,
    // v5: Posts table for both thread and flat boards
    r#"
-- Posts table for both thread and flat boards
CREATE TABLE posts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    board_id    INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    thread_id   INTEGER REFERENCES threads(id) ON DELETE CASCADE,  -- NULL for flat boards
    author_id   INTEGER NOT NULL REFERENCES users(id),
    title       TEXT,                                               -- Used for flat boards
    body        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_posts_board_id ON posts(board_id);
CREATE INDEX idx_posts_thread_id ON posts(thread_id);
CREATE INDEX idx_posts_author_id ON posts(author_id);
CREATE INDEX idx_posts_created_at ON posts(created_at);
"#,
    // v6: Read positions table for unread management
    r#"
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
"#,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_not_empty() {
        assert!(!MIGRATIONS.is_empty());
    }

    #[test]
    fn test_first_migration_contains_users_table() {
        let first = MIGRATIONS[0];
        assert!(first.contains("CREATE TABLE users"));
        assert!(first.contains("username"));
        assert!(first.contains("password"));
        assert!(first.contains("nickname"));
    }

    #[test]
    fn test_migrations_are_valid_sql() {
        // Each migration should be non-empty and contain SQL keywords
        for migration in MIGRATIONS {
            assert!(!migration.trim().is_empty());
            assert!(
                migration.contains("CREATE TABLE")
                    || migration.contains("ALTER TABLE")
                    || migration.contains("CREATE INDEX")
            );
        }
    }

    #[test]
    fn test_boards_migration_contains_boards_table() {
        let boards_migration = MIGRATIONS[2];
        assert!(boards_migration.contains("CREATE TABLE boards"));
        assert!(boards_migration.contains("name"));
        assert!(boards_migration.contains("board_type"));
        assert!(boards_migration.contains("min_read_role"));
        assert!(boards_migration.contains("min_write_role"));
    }

    #[test]
    fn test_threads_migration_contains_threads_table() {
        let threads_migration = MIGRATIONS[3];
        assert!(threads_migration.contains("CREATE TABLE threads"));
        assert!(threads_migration.contains("board_id"));
        assert!(threads_migration.contains("title"));
        assert!(threads_migration.contains("author_id"));
        assert!(threads_migration.contains("post_count"));
        assert!(threads_migration.contains("updated_at"));
    }

    #[test]
    fn test_posts_migration_contains_posts_table() {
        let posts_migration = MIGRATIONS[4];
        assert!(posts_migration.contains("CREATE TABLE posts"));
        assert!(posts_migration.contains("board_id"));
        assert!(posts_migration.contains("thread_id"));
        assert!(posts_migration.contains("author_id"));
        assert!(posts_migration.contains("title"));
        assert!(posts_migration.contains("body"));
    }

    #[test]
    fn test_read_positions_migration_contains_read_positions_table() {
        let read_positions_migration = MIGRATIONS[5];
        assert!(read_positions_migration.contains("CREATE TABLE read_positions"));
        assert!(read_positions_migration.contains("user_id"));
        assert!(read_positions_migration.contains("board_id"));
        assert!(read_positions_migration.contains("last_read_post_id"));
        assert!(read_positions_migration.contains("last_read_at"));
        assert!(read_positions_migration.contains("UNIQUE(user_id, board_id)"));
    }
}
