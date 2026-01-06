//! Database module for HOBBS.
//!
//! This module provides SQLite database connectivity and migration management.

mod refresh_token;
mod repository;
mod schema;
mod user;

pub use refresh_token::{NewRefreshToken, RefreshToken, RefreshTokenRepository};
pub use repository::UserRepository;
pub use schema::MIGRATIONS;
pub use user::{NewUser, Role, User, UserUpdate};

use std::path::Path;

use rusqlite::{Connection, Transaction};
use tracing::{debug, info};

use crate::Result;

/// Database wrapper for managing SQLite connections and migrations.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open a database connection at the specified path.
    ///
    /// If the database file doesn't exist, it will be created.
    /// Migrations are automatically applied.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        info!("Opening database at {:?}", path);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(path)?;
        Self::configure_connection(&conn)?;

        let mut db = Self { conn };
        db.migrate()?;

        Ok(db)
    }

    /// Open an in-memory database for testing.
    pub fn open_in_memory() -> Result<Self> {
        debug!("Opening in-memory database");
        let conn = Connection::open_in_memory()?;
        Self::configure_connection(&conn)?;

        let mut db = Self { conn };
        db.migrate()?;

        Ok(db)
    }

    /// Configure the connection with recommended settings.
    fn configure_connection(conn: &Connection) -> Result<()> {
        // Enable foreign key constraints
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        // Use WAL mode for better concurrent read performance
        // journal_mode returns the mode as a result, so we use query_row
        let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        // Set busy timeout to 5 seconds (returns timeout value, so use query_row)
        let _: i64 = conn.query_row("PRAGMA busy_timeout = 5000", [], |row| row.get(0))?;
        Ok(())
    }

    /// Get a reference to the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Get a mutable reference to the underlying connection.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Begin a new transaction.
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }

    /// Get the current schema version.
    pub fn schema_version(&self) -> Result<i64> {
        // Check if schema_version table exists
        let table_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version')",
            [],
            |row| row.get(0),
        )?;

        if !table_exists {
            return Ok(0);
        }

        let version: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(version)
    }

    /// Apply pending migrations.
    pub fn migrate(&mut self) -> Result<()> {
        let current_version = self.schema_version()?;
        let migrations = MIGRATIONS;

        if current_version as usize >= migrations.len() {
            debug!("Database is up to date (version {})", current_version);
            return Ok(());
        }

        info!(
            "Migrating database from version {} to {}",
            current_version,
            migrations.len()
        );

        // Ensure schema_version table exists
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version     INTEGER PRIMARY KEY,
                applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;

        // Apply each pending migration in a transaction
        for (i, migration) in migrations.iter().enumerate().skip(current_version as usize) {
            let version = (i + 1) as i64;
            info!("Applying migration v{}", version);

            let tx = self.conn.transaction()?;

            // Execute the migration SQL
            tx.execute_batch(migration)?;

            // Record the migration
            tx.execute("INSERT INTO schema_version (version) VALUES (?)", [version])?;

            tx.commit()?;
            debug!("Migration v{} applied successfully", version);
        }

        info!(
            "Database migration complete (now at version {})",
            migrations.len()
        );
        Ok(())
    }

    /// Execute a SQL statement that doesn't return rows.
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        Ok(self.conn.execute(sql, params)?)
    }

    /// Check if a table exists.
    pub fn table_exists(&self, table_name: &str) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
            [table_name],
            |row| row.get(0),
        )?;
        Ok(exists)
    }
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.schema_version().unwrap() > 0);
    }

    #[test]
    fn test_migrations_applied() {
        let db = Database::open_in_memory().unwrap();

        // Check that migrations were applied
        let version = db.schema_version().unwrap();
        assert_eq!(version as usize, MIGRATIONS.len());
    }

    #[test]
    fn test_users_table_exists() {
        let db = Database::open_in_memory().unwrap();

        // Check that users table exists
        assert!(db.table_exists("users").unwrap());
    }

    #[test]
    fn test_schema_version_table_exists() {
        let db = Database::open_in_memory().unwrap();

        assert!(db.table_exists("schema_version").unwrap());
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let db = Database::open_in_memory().unwrap();

        let fk_enabled: i64 = db
            .conn()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk_enabled, 1);
    }

    #[test]
    fn test_insert_and_query_user() {
        let db = Database::open_in_memory().unwrap();

        // Insert a test user
        db.execute(
            "INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)",
            &[&"testuser", &"hashedpassword", &"Test User", &"member"],
        )
        .unwrap();

        // Query the user
        let (id, username, nickname): (i64, String, String) = db
            .conn()
            .query_row(
                "SELECT id, username, nickname FROM users WHERE username = ?",
                ["testuser"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(id, 1);
        assert_eq!(username, "testuser");
        assert_eq!(nickname, "Test User");
    }

    #[test]
    fn test_transaction() {
        let mut db = Database::open_in_memory().unwrap();

        // Start a transaction
        let tx = db.transaction().unwrap();

        // Insert a user
        tx.execute(
            "INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)",
            ["txuser", "hash", "TX User", "member"],
        )
        .unwrap();

        // Commit the transaction
        tx.commit().unwrap();

        // Verify the user was inserted
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM users WHERE username = ?",
                ["txuser"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_transaction_rollback() {
        let mut db = Database::open_in_memory().unwrap();

        // Start a transaction
        {
            let tx = db.transaction().unwrap();

            // Insert a user
            tx.execute(
                "INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)",
                ["rollbackuser", "hash", "Rollback User", "member"],
            )
            .unwrap();

            // Don't commit - transaction will be rolled back when dropped
        }

        // Verify the user was not inserted
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM users WHERE username = ?",
                ["rollbackuser"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_open_file_database() {
        use std::fs;

        let temp_dir = std::env::temp_dir().join("hobbs_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let db_path = temp_dir.join("test.db");

        // Open and close database
        {
            let db = Database::open(&db_path).unwrap();
            assert!(db.table_exists("users").unwrap());
        }

        // Reopen database
        {
            let db = Database::open(&db_path).unwrap();
            assert!(db.table_exists("users").unwrap());
            // Migrations should not be reapplied
            assert_eq!(db.schema_version().unwrap() as usize, MIGRATIONS.len());
        }

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_users_table_columns() {
        let db = Database::open_in_memory().unwrap();

        // Check that all expected columns exist by selecting them
        let result: rusqlite::Result<()> = db.conn().query_row(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    created_at, last_login, is_active
             FROM users LIMIT 0",
            [],
            |_| Ok(()),
        );

        // This should not error - if a column is missing, it will fail
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("no rows"));
    }

    #[test]
    fn test_users_table_indexes() {
        let db = Database::open_in_memory().unwrap();

        // Check indexes exist (username index was renamed to idx_users_username_nocase in v21)
        let idx_username: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_users_username_nocase'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(idx_username, 1);

        let idx_role: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_users_role'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(idx_role, 1);
    }
}
