//! Script execution log repository.
//!
//! Provides logging for script executions.

use crate::db::{DbPool, SQL_TRUE};
use crate::error::Result;
use crate::HobbsError;
use chrono::{Duration, Utc};

/// A single script execution log entry.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScriptLog {
    /// Unique identifier.
    pub id: i64,
    /// Script ID.
    pub script_id: i64,
    /// User ID (None for guest).
    pub user_id: Option<i64>,
    /// Execution timestamp.
    pub executed_at: String,
    /// Execution time in milliseconds.
    pub execution_ms: i64,
    /// Whether execution was successful.
    pub success: bool,
    /// Error message if execution failed.
    pub error_message: Option<String>,
}

/// Repository for script execution logs.
pub struct ScriptLogRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> ScriptLogRepository<'a> {
    /// Create a new script log repository.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Log a script execution.
    pub async fn log_execution(
        &self,
        script_id: i64,
        user_id: Option<i64>,
        execution_ms: i64,
        success: bool,
        error_message: Option<&str>,
    ) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO script_logs (script_id, user_id, execution_ms, success, error_message)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(script_id)
        .bind(user_id)
        .bind(execution_ms)
        .bind(success)
        .bind(error_message)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(id)
    }

    /// Get logs for a specific script.
    pub async fn get_by_script(&self, script_id: i64, limit: usize) -> Result<Vec<ScriptLog>> {
        let logs = sqlx::query_as::<_, ScriptLog>(
            r#"
            SELECT id, script_id, user_id, executed_at, execution_ms, success, error_message
            FROM script_logs
            WHERE script_id = $1
            ORDER BY id DESC
            LIMIT $2
            "#,
        )
        .bind(script_id)
        .bind(limit as i64)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(logs)
    }

    /// Get logs for a specific user.
    pub async fn get_by_user(&self, user_id: i64, limit: usize) -> Result<Vec<ScriptLog>> {
        let logs = sqlx::query_as::<_, ScriptLog>(
            r#"
            SELECT id, script_id, user_id, executed_at, execution_ms, success, error_message
            FROM script_logs
            WHERE user_id = $1
            ORDER BY id DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit as i64)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(logs)
    }

    /// Get execution count for a script.
    pub async fn get_execution_count(&self, script_id: i64) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM script_logs WHERE script_id = $1")
            .bind(script_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }

    /// Get success rate for a script (as percentage 0-100).
    pub async fn get_success_rate(&self, script_id: i64) -> Result<Option<f64>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM script_logs WHERE script_id = $1")
            .bind(script_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if total.0 == 0 {
            return Ok(None);
        }

        let success: (i64,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM script_logs WHERE script_id = $1 AND success = {}",
            SQL_TRUE
        ))
        .bind(script_id)
                .fetch_one(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(Some((success.0 as f64 / total.0 as f64) * 100.0))
    }

    /// Get average execution time for a script (in milliseconds).
    pub async fn get_avg_execution_time(&self, script_id: i64) -> Result<Option<f64>> {
        let avg: (Option<f64>,) =
            sqlx::query_as("SELECT AVG(execution_ms) FROM script_logs WHERE script_id = $1")
                .bind(script_id)
                .fetch_one(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(avg.0)
    }

    /// Delete old logs (older than specified days).
    pub async fn delete_old_logs(&self, days: i32) -> Result<usize> {
        let cutoff = Utc::now() - Duration::days(i64::from(days));
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query("DELETE FROM script_logs WHERE executed_at < $1")
            .bind(&cutoff_str)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }

    /// Delete all logs for a script.
    pub async fn delete_by_script(&self, script_id: i64) -> Result<usize> {
        let result = sqlx::query("DELETE FROM script_logs WHERE script_id = $1")
            .bind(script_id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::Database;
    use sqlx::SqlitePool;

    async fn create_test_db() -> Database {
        Database::open_in_memory()
            .await
            .expect("Failed to create test database")
    }

    async fn create_test_script(pool: &SqlitePool) -> i64 {
        let result = sqlx::query(
            r#"
            INSERT INTO scripts (file_path, name, slug, min_role, enabled)
            VALUES ('test.lua', 'Test Script', 'test', 0, 1)
            "#,
        )
        .execute(pool)
        .await
        .expect("Failed to create test script");

        result.last_insert_rowid()
    }

    async fn create_test_user(pool: &SqlitePool) -> i64 {
        let result = sqlx::query(
            r#"
            INSERT INTO users (username, password, nickname, role)
            VALUES ('testuser', 'hash', 'Test User', 'member')
            "#,
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");

        result.last_insert_rowid()
    }

    #[tokio::test]
    async fn test_log_execution_success() {
        let db = create_test_db().await;
        let pool = db.pool();
        let script_id = create_test_script(pool).await;
        let user_id = create_test_user(pool).await;
        let repo = ScriptLogRepository::new(pool);

        let log_id = repo
            .log_execution(script_id, Some(user_id), 100, true, None)
            .await
            .unwrap();

        assert!(log_id > 0);

        let logs = repo.get_by_script(script_id, 10).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].script_id, script_id);
        assert_eq!(logs[0].user_id, Some(user_id));
        assert_eq!(logs[0].execution_ms, 100);
        assert!(logs[0].success);
        assert!(logs[0].error_message.is_none());
    }

    #[tokio::test]
    async fn test_log_execution_error() {
        let db = create_test_db().await;
        let pool = db.pool();
        let script_id = create_test_script(pool).await;
        let repo = ScriptLogRepository::new(pool);

        repo.log_execution(
            script_id,
            None,
            50,
            false,
            Some("Script error: undefined variable"),
        )
        .await
        .unwrap();

        let logs = repo.get_by_script(script_id, 10).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert!(!logs[0].success);
        assert_eq!(
            logs[0].error_message,
            Some("Script error: undefined variable".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_by_user() {
        let db = create_test_db().await;
        let pool = db.pool();
        let script_id = create_test_script(pool).await;
        let user_id = create_test_user(pool).await;
        let repo = ScriptLogRepository::new(pool);

        repo.log_execution(script_id, Some(user_id), 100, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, Some(user_id), 150, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 200, true, None)
            .await
            .unwrap(); // Guest

        let user_logs = repo.get_by_user(user_id, 10).await.unwrap();
        assert_eq!(user_logs.len(), 2);
    }

    #[tokio::test]
    async fn test_get_execution_count() {
        let db = create_test_db().await;
        let pool = db.pool();
        let script_id = create_test_script(pool).await;
        let repo = ScriptLogRepository::new(pool);

        // Initially 0
        assert_eq!(repo.get_execution_count(script_id).await.unwrap(), 0);

        repo.log_execution(script_id, None, 100, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 150, false, Some("error"))
            .await
            .unwrap();

        assert_eq!(repo.get_execution_count(script_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_success_rate() {
        let db = create_test_db().await;
        let pool = db.pool();
        let script_id = create_test_script(pool).await;
        let repo = ScriptLogRepository::new(pool);

        // No logs - None
        assert!(repo.get_success_rate(script_id).await.unwrap().is_none());

        // 2 success, 1 failure = 66.67%
        repo.log_execution(script_id, None, 100, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 100, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 100, false, Some("error"))
            .await
            .unwrap();

        let rate = repo.get_success_rate(script_id).await.unwrap().unwrap();
        assert!((rate - 66.67).abs() < 1.0);
    }

    #[tokio::test]
    async fn test_get_avg_execution_time() {
        let db = create_test_db().await;
        let pool = db.pool();
        let script_id = create_test_script(pool).await;
        let repo = ScriptLogRepository::new(pool);

        // No logs - None
        assert!(repo
            .get_avg_execution_time(script_id)
            .await
            .unwrap()
            .is_none());

        repo.log_execution(script_id, None, 100, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 200, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 300, true, None)
            .await
            .unwrap();

        let avg = repo
            .get_avg_execution_time(script_id)
            .await
            .unwrap()
            .unwrap();
        assert!((avg - 200.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_delete_by_script() {
        let db = create_test_db().await;
        let pool = db.pool();
        let script_id = create_test_script(pool).await;
        let repo = ScriptLogRepository::new(pool);

        repo.log_execution(script_id, None, 100, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 100, true, None)
            .await
            .unwrap();

        let deleted = repo.delete_by_script(script_id).await.unwrap();
        assert_eq!(deleted, 2);

        assert_eq!(repo.get_execution_count(script_id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_logs_ordered_by_time() {
        let db = create_test_db().await;
        let pool = db.pool();
        let script_id = create_test_script(pool).await;
        let repo = ScriptLogRepository::new(pool);

        repo.log_execution(script_id, None, 100, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 200, true, None)
            .await
            .unwrap();
        repo.log_execution(script_id, None, 300, true, None)
            .await
            .unwrap();

        let logs = repo.get_by_script(script_id, 10).await.unwrap();
        // Should be ordered by executed_at DESC, so newest first
        assert_eq!(logs[0].execution_ms, 300);
        assert_eq!(logs[1].execution_ms, 200);
        assert_eq!(logs[2].execution_ms, 100);
    }
}
