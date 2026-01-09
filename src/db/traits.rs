//! Database abstraction traits for HOBBS.
//!
//! This module defines traits for abstracting database operations,
//! allowing different database backends (SQLite, PostgreSQL, MySQL)
//! to be used interchangeably.
//!
//! # Architecture
//!
//! The abstraction layer consists of:
//! - `DatabaseBackendTrait`: Factory trait for creating database connections
//! - Repository traits (defined in repository_traits module): CRUD operations
//!
//! # Current Implementation
//!
//! Currently, only SQLite (via rusqlite) is implemented. Future phases will add:
//! - Phase B: sqlx with SQLite
//! - Phase C: PostgreSQL and MySQL support via sqlx

use crate::Result;

/// Marker trait for database backends.
///
/// This trait is implemented by database wrapper types to indicate
/// they can be used as a database backend. The actual operations
/// are performed through repository types that take a reference to
/// the implementing type.
///
/// # Note on Thread Safety
///
/// This trait does not require `Send + Sync` because rusqlite's Connection
/// is not thread-safe. For multi-threaded access, wrap the database in
/// appropriate synchronization primitives (e.g., `Arc<Mutex<Database>>`).
///
/// When migrating to sqlx (Phase B), the trait can be updated to require
/// `Send + Sync` since sqlx connections are thread-safe.
///
/// # Example
///
/// ```ignore
/// use hobbs::db::{Database, DatabaseBackendTrait};
///
/// let db = Database::open_in_memory()?;
/// assert!(db.is_sqlite()); // Check backend type
/// ```
pub trait DatabaseBackendTrait {
    /// Returns the name of the database backend.
    fn backend_name(&self) -> &'static str;

    /// Returns true if this is a SQLite backend.
    fn is_sqlite(&self) -> bool {
        self.backend_name() == "sqlite"
    }

    /// Returns true if this is a PostgreSQL backend.
    fn is_postgres(&self) -> bool {
        self.backend_name() == "postgres"
    }

    /// Returns true if this is a MySQL backend.
    fn is_mysql(&self) -> bool {
        self.backend_name() == "mysql"
    }

    /// Get the current schema version.
    fn schema_version(&self) -> Result<i64>;

    /// Check if a table exists in the database.
    fn table_exists(&self, table_name: &str) -> Result<bool>;
}

/// Trait for types that can provide a database connection.
///
/// This trait allows different connection management strategies:
/// - Single connection (current rusqlite implementation)
/// - Connection pool (future sqlx implementation)
pub trait ConnectionProvider {
    /// The connection type provided by this provider.
    type Connection;

    /// Get a connection from the provider.
    ///
    /// For single-connection backends, this returns a reference to the connection.
    /// For pooled backends, this acquires a connection from the pool.
    fn get_connection(&self) -> &Self::Connection;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_database_backend_trait() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.backend_name(), "sqlite");
        assert!(db.is_sqlite());
        assert!(!db.is_postgres());
        assert!(!db.is_mysql());
    }

    #[test]
    fn test_schema_version() {
        let db = Database::open_in_memory().unwrap();
        let version = db.schema_version().unwrap();
        assert!(version > 0);
    }

    #[test]
    fn test_table_exists() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.table_exists("users").unwrap());
        assert!(!db.table_exists("nonexistent").unwrap());
    }
}
