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
    // v7: Chat logs table for chat message history
    r#"
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
"#,
    // v8: Mails table for internal mail system
    r#"
-- Mails table for internal mail system
CREATE TABLE mails (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    sender_id               INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id            INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject                 TEXT NOT NULL,
    body                    TEXT NOT NULL,
    is_read                 INTEGER NOT NULL DEFAULT 0,
    is_deleted_by_sender    INTEGER NOT NULL DEFAULT 0,
    is_deleted_by_recipient INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_mails_sender_id ON mails(sender_id);
CREATE INDEX idx_mails_recipient_id ON mails(recipient_id);
CREATE INDEX idx_mails_created_at ON mails(created_at);
"#,
    // v9: Folders table for file management
    r#"
-- Folders table for file management (hierarchical structure)
CREATE TABLE folders (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT,
    parent_id   INTEGER REFERENCES folders(id) ON DELETE CASCADE,
    permission  TEXT NOT NULL DEFAULT 'member',  -- 閲覧権限
    upload_perm TEXT NOT NULL DEFAULT 'subop',   -- アップロード権限
    order_num   INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_folders_parent ON folders(parent_id, order_num);
"#,
    // v10: Files table for file metadata
    r#"
-- Files table for file metadata
CREATE TABLE files (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id    INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    filename     TEXT NOT NULL,           -- 表示用ファイル名
    stored_name  TEXT NOT NULL,           -- 保存時のファイル名（UUID）
    size         INTEGER NOT NULL,        -- バイト数
    description  TEXT,
    uploader_id  INTEGER NOT NULL REFERENCES users(id),
    downloads    INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_files_folder ON files(folder_id);
CREATE INDEX idx_files_uploader ON files(uploader_id);
"#,
    // v11: Add language column for user language preference
    r#"
ALTER TABLE users ADD COLUMN language TEXT NOT NULL DEFAULT 'en';
"#,
    // v12: Scripts table for Lua script management
    r#"
-- Scripts table for Lua script metadata (file system-based)
CREATE TABLE scripts (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path               TEXT NOT NULL UNIQUE,       -- Relative path from scripts directory
    name                    TEXT NOT NULL,              -- Display name (from metadata)
    slug                    TEXT NOT NULL UNIQUE,       -- URL-safe identifier
    description             TEXT,                       -- Description (from metadata)
    author                  TEXT,                       -- Author name (from metadata)
    file_hash               TEXT,                       -- File hash for change detection
    synced_at               TEXT,                       -- Last sync timestamp
    min_role                INTEGER NOT NULL DEFAULT 0, -- Minimum role to execute (0=Guest)
    enabled                 INTEGER NOT NULL DEFAULT 1, -- Whether the script is enabled
    max_instructions        INTEGER NOT NULL DEFAULT 1000000,
    max_memory_mb           INTEGER NOT NULL DEFAULT 10,
    max_execution_seconds   INTEGER NOT NULL DEFAULT 30
);

CREATE INDEX idx_scripts_enabled ON scripts(enabled);
CREATE INDEX idx_scripts_min_role ON scripts(min_role);
CREATE INDEX idx_scripts_file_path ON scripts(file_path);
CREATE INDEX idx_scripts_slug ON scripts(slug);
"#,
    // v13: Script data table for persistent storage
    r#"
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
"#,
    // v14: Script execution logs
    r#"
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

    #[test]
    fn test_chat_logs_migration_contains_chat_logs_table() {
        let chat_logs_migration = MIGRATIONS[6];
        assert!(chat_logs_migration.contains("CREATE TABLE chat_logs"));
        assert!(chat_logs_migration.contains("room_id"));
        assert!(chat_logs_migration.contains("user_id"));
        assert!(chat_logs_migration.contains("sender_name"));
        assert!(chat_logs_migration.contains("message_type"));
        assert!(chat_logs_migration.contains("content"));
        assert!(chat_logs_migration.contains("created_at"));
    }

    #[test]
    fn test_mails_migration_contains_mails_table() {
        let mails_migration = MIGRATIONS[7];
        assert!(mails_migration.contains("CREATE TABLE mails"));
        assert!(mails_migration.contains("sender_id"));
        assert!(mails_migration.contains("recipient_id"));
        assert!(mails_migration.contains("subject"));
        assert!(mails_migration.contains("body"));
        assert!(mails_migration.contains("is_read"));
        assert!(mails_migration.contains("is_deleted_by_sender"));
        assert!(mails_migration.contains("is_deleted_by_recipient"));
        assert!(mails_migration.contains("created_at"));
    }

    #[test]
    fn test_folders_migration_contains_folders_table() {
        let folders_migration = MIGRATIONS[8];
        assert!(folders_migration.contains("CREATE TABLE folders"));
        assert!(folders_migration.contains("name"));
        assert!(folders_migration.contains("description"));
        assert!(folders_migration.contains("parent_id"));
        assert!(folders_migration.contains("permission"));
        assert!(folders_migration.contains("upload_perm"));
        assert!(folders_migration.contains("order_num"));
        assert!(folders_migration.contains("created_at"));
        assert!(folders_migration.contains("idx_folders_parent"));
    }

    #[test]
    fn test_files_migration_contains_files_table() {
        let files_migration = MIGRATIONS[9];
        assert!(files_migration.contains("CREATE TABLE files"));
        assert!(files_migration.contains("folder_id"));
        assert!(files_migration.contains("filename"));
        assert!(files_migration.contains("stored_name"));
        assert!(files_migration.contains("size"));
        assert!(files_migration.contains("description"));
        assert!(files_migration.contains("uploader_id"));
        assert!(files_migration.contains("downloads"));
        assert!(files_migration.contains("created_at"));
        assert!(files_migration.contains("idx_files_folder"));
        assert!(files_migration.contains("idx_files_uploader"));
    }

    #[test]
    fn test_scripts_migration_contains_scripts_table() {
        let scripts_migration = MIGRATIONS[11];
        assert!(scripts_migration.contains("CREATE TABLE scripts"));
        assert!(scripts_migration.contains("file_path"));
        assert!(scripts_migration.contains("name"));
        assert!(scripts_migration.contains("slug"));
        assert!(scripts_migration.contains("description"));
        assert!(scripts_migration.contains("author"));
        assert!(scripts_migration.contains("file_hash"));
        assert!(scripts_migration.contains("synced_at"));
        assert!(scripts_migration.contains("min_role"));
        assert!(scripts_migration.contains("enabled"));
        assert!(scripts_migration.contains("max_instructions"));
        assert!(scripts_migration.contains("max_memory_mb"));
        assert!(scripts_migration.contains("max_execution_seconds"));
        assert!(scripts_migration.contains("idx_scripts_enabled"));
        assert!(scripts_migration.contains("idx_scripts_min_role"));
        assert!(scripts_migration.contains("idx_scripts_file_path"));
        assert!(scripts_migration.contains("idx_scripts_slug"));
    }

    #[test]
    fn test_script_data_migration_contains_script_data_table() {
        let script_data_migration = MIGRATIONS[12];
        assert!(script_data_migration.contains("CREATE TABLE script_data"));
        assert!(script_data_migration.contains("script_id"));
        assert!(script_data_migration.contains("user_id"));
        assert!(script_data_migration.contains("key"));
        assert!(script_data_migration.contains("value"));
        assert!(script_data_migration.contains("updated_at"));
        assert!(script_data_migration.contains("UNIQUE(script_id, user_id, key)"));
        assert!(script_data_migration.contains("idx_script_data_script"));
        assert!(script_data_migration.contains("idx_script_data_user"));
        assert!(script_data_migration.contains("idx_script_data_script_user"));
    }

    #[test]
    fn test_script_logs_migration_contains_script_logs_table() {
        let script_logs_migration = MIGRATIONS[13];
        assert!(script_logs_migration.contains("CREATE TABLE script_logs"));
        assert!(script_logs_migration.contains("script_id"));
        assert!(script_logs_migration.contains("user_id"));
        assert!(script_logs_migration.contains("executed_at"));
        assert!(script_logs_migration.contains("execution_ms"));
        assert!(script_logs_migration.contains("success"));
        assert!(script_logs_migration.contains("error_message"));
        assert!(script_logs_migration.contains("idx_script_logs_script"));
        assert!(script_logs_migration.contains("idx_script_logs_user"));
        assert!(script_logs_migration.contains("idx_script_logs_executed_at"));
    }
}
