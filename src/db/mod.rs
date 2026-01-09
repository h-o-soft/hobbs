//! Database module for HOBBS.
//!
//! This module provides database connectivity and migration management.
//!
//! # Backend Support
//!
//! Supports multiple backends via feature flags:
//! - SQLite via sqlx with connection pooling (feature = "sqlite")
//! - PostgreSQL via sqlx with connection pooling (feature = "postgres")

mod refresh_token;
mod repository;
mod user;

pub use refresh_token::{NewRefreshToken, RefreshToken, RefreshTokenRepository};
pub use repository::UserRepository;
pub use user::{NewUser, Role, User, UserUpdate};

use tracing::{debug, info};

use crate::Result;

// Type aliases for database pool based on feature
#[cfg(feature = "sqlite")]
pub type DbPool = sqlx::sqlite::SqlitePool;

#[cfg(feature = "postgres")]
pub type DbPool = sqlx::postgres::PgPool;

/// Database wrapper for managing database connections with connection pooling.
pub struct Database {
    pool: DbPool,
}

// SQLite implementation
#[cfg(feature = "sqlite")]
mod sqlite_impl {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::ConnectOptions;
    use std::path::Path;
    use std::time::Duration;

    impl Database {
        /// Open a database connection pool at the specified path.
        ///
        /// If the database file doesn't exist, it will be created.
        /// Migrations are automatically applied.
        pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
            let path = path.as_ref();
            info!("Opening SQLite database at {:?}", path);

            // Create parent directories if they don't exist
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            let options = SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .foreign_keys(true)
                .busy_timeout(Duration::from_secs(5))
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .disable_statement_logging();

            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(options)
                .await
                .map_err(|e| crate::HobbsError::DatabaseConnection(e.to_string()))?;

            let db = Self { pool };
            db.migrate().await?;

            Ok(db)
        }

        /// Open an in-memory database for testing.
        pub async fn open_in_memory() -> Result<Self> {
            debug!("Opening in-memory SQLite database");

            let options = SqliteConnectOptions::new()
                .filename(":memory:")
                .foreign_keys(true)
                .disable_statement_logging();

            // For in-memory databases, we need exactly 1 connection to share state
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(options)
                .await
                .map_err(|e| crate::HobbsError::DatabaseConnection(e.to_string()))?;

            let db = Self { pool };
            db.migrate().await?;

            Ok(db)
        }

        /// Get the current schema version.
        pub async fn schema_version(&self) -> Result<i64> {
            // Check if _sqlx_migrations table exists
            let table_exists: i32 = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations')",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

            if table_exists == 0 {
                return Ok(0);
            }

            let version: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

            Ok(version)
        }

        /// Apply pending migrations using sqlx embedded migrations.
        ///
        /// For legacy databases (created with rusqlite before the sqlx migration),
        /// all migrations are marked as already applied since the schema is already in place.
        pub async fn migrate(&self) -> Result<()> {
            info!("Running SQLite database migrations...");

            // Check if this is a legacy database that needs migration records
            let users_table_exists: i32 = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='users')",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

            let migrations_table_exists: i32 = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations')",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

            // Check how many migrations are recorded
            let migrations_recorded: i64 = if migrations_table_exists == 1 {
                sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
                    .fetch_one(&self.pool)
                    .await
                    .unwrap_or(0)
            } else {
                0
            };

            info!(
                "Migration check: users exists={}, migrations table exists={}, migrations recorded={}",
                users_table_exists, migrations_table_exists, migrations_recorded
            );

            // If users table exists but no migrations are recorded, this is a legacy database
            if users_table_exists == 1 && migrations_recorded == 0 {
                // Legacy database detected - mark all migrations as applied
                info!("Legacy database detected, marking migrations as applied...");
                self.mark_legacy_migrations_applied().await?;
            } else {
                // Run migrations normally
                sqlx::migrate!("./migrations/sqlite")
                    .run(&self.pool)
                    .await
                    .map_err(|e| crate::HobbsError::Database(format!("Migration failed: {}", e)))?;
            }

            let version = self.schema_version().await?;
            info!("Database migration complete (version {})", version);

            Ok(())
        }

        /// Mark all migrations as applied for legacy databases.
        ///
        /// This is used when migrating from rusqlite to sqlx. The legacy database
        /// already has all tables, so we just need to create the _sqlx_migrations
        /// table and mark all migrations as completed.
        async fn mark_legacy_migrations_applied(&self) -> Result<()> {
            // Create _sqlx_migrations table
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                    version BIGINT PRIMARY KEY,
                    description TEXT NOT NULL,
                    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    success BOOLEAN NOT NULL,
                    checksum BLOB NOT NULL,
                    execution_time BIGINT NOT NULL
                )
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

            // Get all migrations from the embedded migrator
            let migrator = sqlx::migrate!("./migrations/sqlite");

            for migration in migrator.iter() {
                // Insert each migration as already applied
                sqlx::query(
                    r#"
                    INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
                    VALUES (?, ?, CURRENT_TIMESTAMP, 1, ?, 0)
                    "#,
                )
                .bind(migration.version)
                .bind(&*migration.description)
                .bind(&*migration.checksum)
                .execute(&self.pool)
                .await
                .map_err(|e| crate::HobbsError::Database(e.to_string()))?;
            }

            info!(
                "Marked {} migrations as applied for legacy database",
                migrator.iter().count()
            );

            Ok(())
        }

        /// Check if a table exists.
        pub async fn table_exists(&self, table_name: &str) -> Result<bool> {
            let exists: i32 = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
            )
            .bind(table_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

            Ok(exists == 1)
        }
    }
}

// PostgreSQL implementation
#[cfg(feature = "postgres")]
mod postgres_impl {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    impl Database {
        /// Open a database connection pool with the specified URL.
        ///
        /// Migrations are automatically applied.
        pub async fn open(url: impl AsRef<str>) -> Result<Self> {
            let url = url.as_ref();
            info!("Opening PostgreSQL database");

            let pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(url)
                .await
                .map_err(|e| crate::HobbsError::DatabaseConnection(e.to_string()))?;

            let db = Self { pool };
            db.migrate().await?;

            Ok(db)
        }

        /// Open an in-memory database for testing.
        ///
        /// Note: PostgreSQL doesn't support in-memory databases, so this creates
        /// a temporary database. The DATABASE_URL environment variable must be set
        /// to a PostgreSQL connection string for a test database.
        pub async fn open_in_memory() -> Result<Self> {
            debug!("Opening PostgreSQL test database");

            let url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://hobbs:hobbs@localhost/hobbs_test".to_string());

            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .map_err(|e| crate::HobbsError::DatabaseConnection(e.to_string()))?;

            // Clean up existing tables for a fresh test
            let db = Self { pool };
            db.cleanup_test_database().await?;
            db.migrate().await?;

            Ok(db)
        }

        /// Clean up test database by dropping all tables.
        async fn cleanup_test_database(&self) -> Result<()> {
            // Drop _sqlx_migrations first to allow fresh migration
            let _ = sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
                .execute(&self.pool)
                .await;

            // Get all user tables and drop them
            let tables: Vec<(String,)> = sqlx::query_as(
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename != '_sqlx_migrations'"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            for (table,) in tables {
                let _ = sqlx::query(&format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table))
                    .execute(&self.pool)
                    .await;
            }

            Ok(())
        }

        /// Get the current schema version.
        pub async fn schema_version(&self) -> Result<i64> {
            // Check if _sqlx_migrations table exists
            let table_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '_sqlx_migrations')",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

            if !table_exists {
                return Ok(0);
            }

            let version: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

            Ok(version)
        }

        /// Apply pending migrations using sqlx embedded migrations.
        pub async fn migrate(&self) -> Result<()> {
            info!("Running PostgreSQL database migrations...");

            sqlx::migrate!("./migrations/postgres")
                .run(&self.pool)
                .await
                .map_err(|e| crate::HobbsError::Database(format!("Migration failed: {}", e)))?;

            let version = self.schema_version().await?;
            info!("Database migration complete (version {})", version);

            Ok(())
        }

        /// Check if a table exists.
        pub async fn table_exists(&self, table_name: &str) -> Result<bool> {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1)",
            )
            .bind(table_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

            Ok(exists)
        }
    }
}

// Common implementation for both backends
impl Database {
    /// Get a reference to the underlying connection pool.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Close the database connection pool.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database").finish()
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_in_memory() {
        let db = Database::open_in_memory().await.unwrap();
        assert!(db.schema_version().await.unwrap() > 0);
    }

    #[tokio::test]
    async fn test_migrations_applied() {
        let db = Database::open_in_memory().await.unwrap();

        // Check that migrations were applied
        let version = db.schema_version().await.unwrap();
        assert_eq!(version as usize, 22); // 22 migrations
    }

    #[tokio::test]
    async fn test_users_table_exists() {
        let db = Database::open_in_memory().await.unwrap();

        // Check that users table exists
        assert!(db.table_exists("users").await.unwrap());
    }

    #[tokio::test]
    async fn test_foreign_keys_enabled() {
        let db = Database::open_in_memory().await.unwrap();

        let fk_enabled: i32 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(fk_enabled, 1);
    }

    #[tokio::test]
    async fn test_insert_and_query_user() {
        let db = Database::open_in_memory().await.unwrap();

        // Insert a test user
        sqlx::query("INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)")
            .bind("testuser")
            .bind("hashedpassword")
            .bind("Test User")
            .bind("member")
            .execute(db.pool())
            .await
            .unwrap();

        // Query the user
        let row: (i64, String, String) =
            sqlx::query_as("SELECT id, username, nickname FROM users WHERE username = ?")
                .bind("testuser")
                .fetch_one(db.pool())
                .await
                .unwrap();

        assert_eq!(row.0, 1);
        assert_eq!(row.1, "testuser");
        assert_eq!(row.2, "Test User");
    }

    #[tokio::test]
    async fn test_transaction() {
        let db = Database::open_in_memory().await.unwrap();

        // Start a transaction
        let mut tx = db.pool().begin().await.unwrap();

        // Insert a user
        sqlx::query("INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)")
            .bind("txuser")
            .bind("hash")
            .bind("TX User")
            .bind("member")
            .execute(&mut *tx)
            .await
            .unwrap();

        // Commit the transaction
        tx.commit().await.unwrap();

        // Verify the user was inserted
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ?")
            .bind("txuser")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let db = Database::open_in_memory().await.unwrap();

        // Start a transaction
        {
            let mut tx = db.pool().begin().await.unwrap();

            // Insert a user
            sqlx::query(
                "INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)",
            )
            .bind("rollbackuser")
            .bind("hash")
            .bind("Rollback User")
            .bind("member")
            .execute(&mut *tx)
            .await
            .unwrap();

            // Don't commit - transaction will be rolled back when dropped
        }

        // Verify the user was not inserted
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ?")
            .bind("rollbackuser")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_open_file_database() {
        use std::fs;

        let temp_dir = std::env::temp_dir().join("hobbs_test_sqlx");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let db_path = temp_dir.join("test.db");

        // Open and close database
        {
            let db = Database::open(&db_path).await.unwrap();
            assert!(db.table_exists("users").await.unwrap());
            db.close().await;
        }

        // Reopen database
        {
            let db = Database::open(&db_path).await.unwrap();
            assert!(db.table_exists("users").await.unwrap());
            // Migrations should not be reapplied
            assert_eq!(db.schema_version().await.unwrap(), 22);
            db.close().await;
        }

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_users_table_columns() {
        let db = Database::open_in_memory().await.unwrap();

        // Check that all expected columns exist by selecting them
        let result = sqlx::query(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    created_at, last_login, is_active
             FROM users LIMIT 0",
        )
        .execute(db.pool())
        .await;

        // This should not error - if a column is missing, it will fail
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_users_table_indexes() {
        let db = Database::open_in_memory().await.unwrap();

        // Check indexes exist (username index was renamed to idx_users_username_nocase in v21)
        let idx_username: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_users_username_nocase'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(idx_username, 1);

        let idx_role: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_users_role'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(idx_role, 1);
    }
}
