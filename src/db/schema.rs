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
}
