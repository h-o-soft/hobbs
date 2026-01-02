//! Script data repository for persistent key-value storage.
//!
//! Provides storage for script-specific data, both global and per-user.

use crate::db::Database;
use crate::error::Result;
use rusqlite::{params, OptionalExtension};

/// A single script data entry.
#[derive(Debug, Clone)]
pub struct ScriptData {
    /// Unique identifier.
    pub id: i64,
    /// Script ID this data belongs to.
    pub script_id: i64,
    /// User ID (None for global data).
    pub user_id: Option<i64>,
    /// Data key.
    pub key: String,
    /// JSON-encoded value.
    pub value: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Repository for script data operations.
pub struct ScriptDataRepository<'a> {
    db: &'a Database,
}

impl<'a> ScriptDataRepository<'a> {
    /// Create a new script data repository.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get global data for a script.
    pub fn get_global(&self, script_id: i64, key: &str) -> Result<Option<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT value FROM script_data WHERE script_id = ? AND user_id IS NULL AND key = ?",
        )?;

        let result = stmt
            .query_row(params![script_id, key], |row| row.get::<_, String>(0))
            .optional()?;

        Ok(result)
    }

    /// Set global data for a script.
    pub fn set_global(&self, script_id: i64, key: &str, value: &str) -> Result<()> {
        let conn = self.db.conn();

        // SQLite doesn't treat NULL as equal in UNIQUE constraints,
        // so we need to check and update/insert separately
        let exists: i32 = conn
            .prepare("SELECT COUNT(*) FROM script_data WHERE script_id = ? AND user_id IS NULL AND key = ?")?
            .query_row(params![script_id, key], |row| row.get(0))?;

        if exists > 0 {
            conn.execute(
                "UPDATE script_data SET value = ?, updated_at = datetime('now') WHERE script_id = ? AND user_id IS NULL AND key = ?",
                params![value, script_id, key],
            )?;
        } else {
            conn.execute(
                "INSERT INTO script_data (script_id, user_id, key, value, updated_at) VALUES (?, NULL, ?, ?, datetime('now'))",
                params![script_id, key, value],
            )?;
        }

        Ok(())
    }

    /// Delete global data for a script.
    pub fn delete_global(&self, script_id: i64, key: &str) -> Result<bool> {
        let conn = self.db.conn();
        let affected = conn.execute(
            "DELETE FROM script_data WHERE script_id = ? AND user_id IS NULL AND key = ?",
            params![script_id, key],
        )?;

        Ok(affected > 0)
    }

    /// Get user-specific data for a script.
    pub fn get_user(&self, script_id: i64, user_id: i64, key: &str) -> Result<Option<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT value FROM script_data WHERE script_id = ? AND user_id = ? AND key = ?",
        )?;

        let result = stmt
            .query_row(params![script_id, user_id, key], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;

        Ok(result)
    }

    /// Set user-specific data for a script.
    pub fn set_user(&self, script_id: i64, user_id: i64, key: &str, value: &str) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            r#"
            INSERT INTO script_data (script_id, user_id, key, value, updated_at)
            VALUES (?, ?, ?, ?, datetime('now'))
            ON CONFLICT(script_id, user_id, key) DO UPDATE SET
                value = excluded.value,
                updated_at = datetime('now')
            "#,
            params![script_id, user_id, key, value],
        )?;

        Ok(())
    }

    /// Delete user-specific data for a script.
    pub fn delete_user(&self, script_id: i64, user_id: i64, key: &str) -> Result<bool> {
        let conn = self.db.conn();
        let affected = conn.execute(
            "DELETE FROM script_data WHERE script_id = ? AND user_id = ? AND key = ?",
            params![script_id, user_id, key],
        )?;

        Ok(affected > 0)
    }

    /// List all global keys for a script.
    pub fn list_global_keys(&self, script_id: i64) -> Result<Vec<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT key FROM script_data WHERE script_id = ? AND user_id IS NULL ORDER BY key",
        )?;

        let keys = stmt
            .query_map(params![script_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(keys)
    }

    /// List all user-specific keys for a script.
    pub fn list_user_keys(&self, script_id: i64, user_id: i64) -> Result<Vec<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT key FROM script_data WHERE script_id = ? AND user_id = ? ORDER BY key",
        )?;

        let keys = stmt
            .query_map(params![script_id, user_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(keys)
    }

    /// Delete all data for a script.
    pub fn delete_all_for_script(&self, script_id: i64) -> Result<usize> {
        let conn = self.db.conn();
        let affected = conn.execute(
            "DELETE FROM script_data WHERE script_id = ?",
            params![script_id],
        )?;

        Ok(affected)
    }

    /// Delete all data for a user across all scripts.
    pub fn delete_all_for_user(&self, user_id: i64) -> Result<usize> {
        let conn = self.db.conn();
        let affected = conn.execute(
            "DELETE FROM script_data WHERE user_id = ?",
            params![user_id],
        )?;

        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> Database {
        Database::open_in_memory().expect("Failed to create test database")
    }

    fn create_test_script(db: &Database) -> i64 {
        let conn = db.conn();
        conn.execute(
            r#"
            INSERT INTO scripts (file_path, name, slug, min_role, enabled)
            VALUES ('test.lua', 'Test Script', 'test', 0, 1)
            "#,
            [],
        )
        .expect("Failed to create test script");

        conn.last_insert_rowid()
    }

    fn create_test_user(db: &Database) -> i64 {
        let conn = db.conn();
        conn.execute(
            r#"
            INSERT INTO users (username, password, nickname, role)
            VALUES ('testuser', 'hash', 'Test User', 'member')
            "#,
            [],
        )
        .expect("Failed to create test user");

        conn.last_insert_rowid()
    }

    #[test]
    fn test_global_data_crud() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let repo = ScriptDataRepository::new(&db);

        // Initially no data
        assert!(repo.get_global(script_id, "score").unwrap().is_none());

        // Set data
        repo.set_global(script_id, "score", "100").unwrap();
        assert_eq!(
            repo.get_global(script_id, "score").unwrap(),
            Some("100".to_string())
        );

        // Update data
        repo.set_global(script_id, "score", "200").unwrap();
        assert_eq!(
            repo.get_global(script_id, "score").unwrap(),
            Some("200".to_string())
        );

        // Delete data
        assert!(repo.delete_global(script_id, "score").unwrap());
        assert!(repo.get_global(script_id, "score").unwrap().is_none());

        // Delete non-existent
        assert!(!repo.delete_global(script_id, "nonexistent").unwrap());
    }

    #[test]
    fn test_user_data_crud() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let user_id = create_test_user(&db);
        let repo = ScriptDataRepository::new(&db);

        // Initially no data
        assert!(repo.get_user(script_id, user_id, "wins").unwrap().is_none());

        // Set data
        repo.set_user(script_id, user_id, "wins", "5").unwrap();
        assert_eq!(
            repo.get_user(script_id, user_id, "wins").unwrap(),
            Some("5".to_string())
        );

        // Update data
        repo.set_user(script_id, user_id, "wins", "10").unwrap();
        assert_eq!(
            repo.get_user(script_id, user_id, "wins").unwrap(),
            Some("10".to_string())
        );

        // Delete data
        assert!(repo.delete_user(script_id, user_id, "wins").unwrap());
        assert!(repo.get_user(script_id, user_id, "wins").unwrap().is_none());
    }

    #[test]
    fn test_global_and_user_data_separate() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let user_id = create_test_user(&db);
        let repo = ScriptDataRepository::new(&db);

        // Set both global and user data with same key
        repo.set_global(script_id, "counter", "global_value")
            .unwrap();
        repo.set_user(script_id, user_id, "counter", "user_value")
            .unwrap();

        // They should be separate
        assert_eq!(
            repo.get_global(script_id, "counter").unwrap(),
            Some("global_value".to_string())
        );
        assert_eq!(
            repo.get_user(script_id, user_id, "counter").unwrap(),
            Some("user_value".to_string())
        );

        // Deleting one doesn't affect the other
        repo.delete_global(script_id, "counter").unwrap();
        assert!(repo.get_global(script_id, "counter").unwrap().is_none());
        assert_eq!(
            repo.get_user(script_id, user_id, "counter").unwrap(),
            Some("user_value".to_string())
        );
    }

    #[test]
    fn test_list_keys() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let user_id = create_test_user(&db);
        let repo = ScriptDataRepository::new(&db);

        // Set multiple keys
        repo.set_global(script_id, "key1", "v1").unwrap();
        repo.set_global(script_id, "key2", "v2").unwrap();
        repo.set_user(script_id, user_id, "user_key1", "uv1")
            .unwrap();
        repo.set_user(script_id, user_id, "user_key2", "uv2")
            .unwrap();

        // List global keys
        let global_keys = repo.list_global_keys(script_id).unwrap();
        assert_eq!(global_keys, vec!["key1", "key2"]);

        // List user keys
        let user_keys = repo.list_user_keys(script_id, user_id).unwrap();
        assert_eq!(user_keys, vec!["user_key1", "user_key2"]);
    }

    #[test]
    fn test_delete_all_for_script() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let user_id = create_test_user(&db);
        let repo = ScriptDataRepository::new(&db);

        // Set some data
        repo.set_global(script_id, "global_key", "gv").unwrap();
        repo.set_user(script_id, user_id, "user_key", "uv").unwrap();

        // Delete all for script
        let deleted = repo.delete_all_for_script(script_id).unwrap();
        assert_eq!(deleted, 2);

        // Verify all deleted
        assert!(repo.get_global(script_id, "global_key").unwrap().is_none());
        assert!(repo
            .get_user(script_id, user_id, "user_key")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_delete_all_for_user() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let user_id = create_test_user(&db);
        let repo = ScriptDataRepository::new(&db);

        // Set some data
        repo.set_global(script_id, "global_key", "gv").unwrap();
        repo.set_user(script_id, user_id, "user_key", "uv").unwrap();

        // Delete all for user
        let deleted = repo.delete_all_for_user(user_id).unwrap();
        assert_eq!(deleted, 1);

        // Global data should remain
        assert_eq!(
            repo.get_global(script_id, "global_key").unwrap(),
            Some("gv".to_string())
        );
        // User data should be deleted
        assert!(repo
            .get_user(script_id, user_id, "user_key")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_json_values() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let repo = ScriptDataRepository::new(&db);

        // Store JSON value
        let json_value = r#"{"score":100,"level":5,"items":["sword","shield"]}"#;
        repo.set_global(script_id, "game_state", json_value)
            .unwrap();

        // Retrieve and verify
        let retrieved = repo.get_global(script_id, "game_state").unwrap().unwrap();
        assert_eq!(retrieved, json_value);
    }
}
