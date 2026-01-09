//! Script data repository for persistent key-value storage.
//!
//! Provides storage for script-specific data, both global and per-user.

use crate::db::DbPool;
use crate::error::{HobbsError, Result};

/// A single script data entry.
#[derive(Debug, Clone, sqlx::FromRow)]
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
    pool: &'a DbPool,
}

impl<'a> ScriptDataRepository<'a> {
    /// Create a new script data repository.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Get global data for a script.
    pub async fn get_global(&self, script_id: i64, key: &str) -> Result<Option<String>> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT value FROM script_data WHERE script_id = ? AND user_id IS NULL AND key = ?",
        )
        .bind(script_id)
        .bind(key)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Set global data for a script.
    pub async fn set_global(&self, script_id: i64, key: &str, value: &str) -> Result<()> {
        // SQLite doesn't treat NULL as equal in UNIQUE constraints,
        // so we need to check and update/insert separately
        let exists: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM script_data WHERE script_id = ? AND user_id IS NULL AND key = ?",
        )
        .bind(script_id)
        .bind(key)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        if exists > 0 {
            sqlx::query(
                "UPDATE script_data SET value = ?, updated_at = datetime('now') WHERE script_id = ? AND user_id IS NULL AND key = ?",
            )
            .bind(value)
            .bind(script_id)
            .bind(key)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        } else {
            sqlx::query(
                "INSERT INTO script_data (script_id, user_id, key, value, updated_at) VALUES (?, NULL, ?, ?, datetime('now'))",
            )
            .bind(script_id)
            .bind(key)
            .bind(value)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        }

        Ok(())
    }

    /// Delete global data for a script.
    pub async fn delete_global(&self, script_id: i64, key: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM script_data WHERE script_id = ? AND user_id IS NULL AND key = ?",
        )
        .bind(script_id)
        .bind(key)
        .execute(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get user-specific data for a script.
    pub async fn get_user(
        &self,
        script_id: i64,
        user_id: i64,
        key: &str,
    ) -> Result<Option<String>> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT value FROM script_data WHERE script_id = ? AND user_id = ? AND key = ?",
        )
        .bind(script_id)
        .bind(user_id)
        .bind(key)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Set user-specific data for a script.
    pub async fn set_user(
        &self,
        script_id: i64,
        user_id: i64,
        key: &str,
        value: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO script_data (script_id, user_id, key, value, updated_at)
            VALUES (?, ?, ?, ?, datetime('now'))
            ON CONFLICT(script_id, user_id, key) DO UPDATE SET
                value = excluded.value,
                updated_at = datetime('now')
            "#,
        )
        .bind(script_id)
        .bind(user_id)
        .bind(key)
        .bind(value)
        .execute(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete user-specific data for a script.
    pub async fn delete_user(&self, script_id: i64, user_id: i64, key: &str) -> Result<bool> {
        let result =
            sqlx::query("DELETE FROM script_data WHERE script_id = ? AND user_id = ? AND key = ?")
                .bind(script_id)
                .bind(user_id)
                .bind(key)
                .execute(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// List all global keys for a script.
    pub async fn list_global_keys(&self, script_id: i64) -> Result<Vec<String>> {
        let keys = sqlx::query_scalar::<_, String>(
            "SELECT key FROM script_data WHERE script_id = ? AND user_id IS NULL ORDER BY key",
        )
        .bind(script_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(keys)
    }

    /// List all user-specific keys for a script.
    pub async fn list_user_keys(&self, script_id: i64, user_id: i64) -> Result<Vec<String>> {
        let keys = sqlx::query_scalar::<_, String>(
            "SELECT key FROM script_data WHERE script_id = ? AND user_id = ? ORDER BY key",
        )
        .bind(script_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(keys)
    }

    /// Delete all data for a script.
    pub async fn delete_all_for_script(&self, script_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM script_data WHERE script_id = ?")
            .bind(script_id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Delete all data for a user across all scripts.
    pub async fn delete_all_for_user(&self, user_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM script_data WHERE user_id = ?")
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use sqlx::SqlitePool;

    async fn create_test_pool() -> SqlitePool {
        let db = Database::open_in_memory()
            .await
            .expect("Failed to create test database");
        db.pool().clone()
    }

    async fn create_test_script(pool: &SqlitePool) -> i64 {
        sqlx::query(
            r#"
            INSERT INTO scripts (file_path, name, slug, min_role, enabled)
            VALUES ('test.lua', 'Test Script', 'test', 0, 1)
            "#,
        )
        .execute(pool)
        .await
        .expect("Failed to create test script");

        sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(pool)
            .await
            .expect("Failed to get last insert rowid")
    }

    async fn create_test_user(pool: &SqlitePool) -> i64 {
        sqlx::query(
            r#"
            INSERT INTO users (username, password, nickname, role)
            VALUES ('testuser', 'hash', 'Test User', 'member')
            "#,
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");

        sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(pool)
            .await
            .expect("Failed to get last insert rowid")
    }

    #[tokio::test]
    async fn test_global_data_crud() {
        let pool = create_test_pool().await;
        let script_id = create_test_script(&pool).await;
        let repo = ScriptDataRepository::new(&pool);

        // Initially no data
        assert!(repo.get_global(script_id, "score").await.unwrap().is_none());

        // Set data
        repo.set_global(script_id, "score", "100").await.unwrap();
        assert_eq!(
            repo.get_global(script_id, "score").await.unwrap(),
            Some("100".to_string())
        );

        // Update data
        repo.set_global(script_id, "score", "200").await.unwrap();
        assert_eq!(
            repo.get_global(script_id, "score").await.unwrap(),
            Some("200".to_string())
        );

        // Delete data
        assert!(repo.delete_global(script_id, "score").await.unwrap());
        assert!(repo.get_global(script_id, "score").await.unwrap().is_none());

        // Delete non-existent
        assert!(!repo.delete_global(script_id, "nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_user_data_crud() {
        let pool = create_test_pool().await;
        let script_id = create_test_script(&pool).await;
        let user_id = create_test_user(&pool).await;
        let repo = ScriptDataRepository::new(&pool);

        // Initially no data
        assert!(repo
            .get_user(script_id, user_id, "wins")
            .await
            .unwrap()
            .is_none());

        // Set data
        repo.set_user(script_id, user_id, "wins", "5")
            .await
            .unwrap();
        assert_eq!(
            repo.get_user(script_id, user_id, "wins").await.unwrap(),
            Some("5".to_string())
        );

        // Update data
        repo.set_user(script_id, user_id, "wins", "10")
            .await
            .unwrap();
        assert_eq!(
            repo.get_user(script_id, user_id, "wins").await.unwrap(),
            Some("10".to_string())
        );

        // Delete data
        assert!(repo.delete_user(script_id, user_id, "wins").await.unwrap());
        assert!(repo
            .get_user(script_id, user_id, "wins")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_global_and_user_data_separate() {
        let pool = create_test_pool().await;
        let script_id = create_test_script(&pool).await;
        let user_id = create_test_user(&pool).await;
        let repo = ScriptDataRepository::new(&pool);

        // Set both global and user data with same key
        repo.set_global(script_id, "counter", "global_value")
            .await
            .unwrap();
        repo.set_user(script_id, user_id, "counter", "user_value")
            .await
            .unwrap();

        // They should be separate
        assert_eq!(
            repo.get_global(script_id, "counter").await.unwrap(),
            Some("global_value".to_string())
        );
        assert_eq!(
            repo.get_user(script_id, user_id, "counter").await.unwrap(),
            Some("user_value".to_string())
        );

        // Deleting one doesn't affect the other
        repo.delete_global(script_id, "counter").await.unwrap();
        assert!(repo
            .get_global(script_id, "counter")
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            repo.get_user(script_id, user_id, "counter").await.unwrap(),
            Some("user_value".to_string())
        );
    }

    #[tokio::test]
    async fn test_list_keys() {
        let pool = create_test_pool().await;
        let script_id = create_test_script(&pool).await;
        let user_id = create_test_user(&pool).await;
        let repo = ScriptDataRepository::new(&pool);

        // Set multiple keys
        repo.set_global(script_id, "key1", "v1").await.unwrap();
        repo.set_global(script_id, "key2", "v2").await.unwrap();
        repo.set_user(script_id, user_id, "user_key1", "uv1")
            .await
            .unwrap();
        repo.set_user(script_id, user_id, "user_key2", "uv2")
            .await
            .unwrap();

        // List global keys
        let global_keys = repo.list_global_keys(script_id).await.unwrap();
        assert_eq!(global_keys, vec!["key1", "key2"]);

        // List user keys
        let user_keys = repo.list_user_keys(script_id, user_id).await.unwrap();
        assert_eq!(user_keys, vec!["user_key1", "user_key2"]);
    }

    #[tokio::test]
    async fn test_delete_all_for_script() {
        let pool = create_test_pool().await;
        let script_id = create_test_script(&pool).await;
        let user_id = create_test_user(&pool).await;
        let repo = ScriptDataRepository::new(&pool);

        // Set some data
        repo.set_global(script_id, "global_key", "gv")
            .await
            .unwrap();
        repo.set_user(script_id, user_id, "user_key", "uv")
            .await
            .unwrap();

        // Delete all for script
        let deleted = repo.delete_all_for_script(script_id).await.unwrap();
        assert_eq!(deleted, 2);

        // Verify all deleted
        assert!(repo
            .get_global(script_id, "global_key")
            .await
            .unwrap()
            .is_none());
        assert!(repo
            .get_user(script_id, user_id, "user_key")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_delete_all_for_user() {
        let pool = create_test_pool().await;
        let script_id = create_test_script(&pool).await;
        let user_id = create_test_user(&pool).await;
        let repo = ScriptDataRepository::new(&pool);

        // Set some data
        repo.set_global(script_id, "global_key", "gv")
            .await
            .unwrap();
        repo.set_user(script_id, user_id, "user_key", "uv")
            .await
            .unwrap();

        // Delete all for user
        let deleted = repo.delete_all_for_user(user_id).await.unwrap();
        assert_eq!(deleted, 1);

        // Global data should remain
        assert_eq!(
            repo.get_global(script_id, "global_key").await.unwrap(),
            Some("gv".to_string())
        );
        // User data should be deleted
        assert!(repo
            .get_user(script_id, user_id, "user_key")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_json_values() {
        let pool = create_test_pool().await;
        let script_id = create_test_script(&pool).await;
        let repo = ScriptDataRepository::new(&pool);

        // Store JSON value
        let json_value = r#"{"score":100,"level":5,"items":["sword","shield"]}"#;
        repo.set_global(script_id, "game_state", json_value)
            .await
            .unwrap();

        // Retrieve and verify
        let retrieved = repo
            .get_global(script_id, "game_state")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved, json_value);
    }
}
