//! Script execution log repository.
//!
//! Provides logging for script executions.

use crate::db::Database;
use crate::error::Result;
use rusqlite::params;

/// A single script execution log entry.
#[derive(Debug, Clone)]
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
    db: &'a Database,
}

impl<'a> ScriptLogRepository<'a> {
    /// Create a new script log repository.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Log a script execution.
    pub fn log_execution(
        &self,
        script_id: i64,
        user_id: Option<i64>,
        execution_ms: i64,
        success: bool,
        error_message: Option<&str>,
    ) -> Result<i64> {
        let conn = self.db.conn();
        conn.execute(
            r#"
            INSERT INTO script_logs (script_id, user_id, execution_ms, success, error_message)
            VALUES (?, ?, ?, ?, ?)
            "#,
            params![
                script_id,
                user_id,
                execution_ms,
                success as i32,
                error_message
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get logs for a specific script.
    pub fn get_by_script(&self, script_id: i64, limit: usize) -> Result<Vec<ScriptLog>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, script_id, user_id, executed_at, execution_ms, success, error_message
            FROM script_logs
            WHERE script_id = ?
            ORDER BY id DESC
            LIMIT ?
            "#,
        )?;

        let logs = stmt
            .query_map(params![script_id, limit as i64], |row| {
                Ok(ScriptLog {
                    id: row.get(0)?,
                    script_id: row.get(1)?,
                    user_id: row.get(2)?,
                    executed_at: row.get(3)?,
                    execution_ms: row.get(4)?,
                    success: row.get::<_, i32>(5)? != 0,
                    error_message: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(logs)
    }

    /// Get logs for a specific user.
    pub fn get_by_user(&self, user_id: i64, limit: usize) -> Result<Vec<ScriptLog>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, script_id, user_id, executed_at, execution_ms, success, error_message
            FROM script_logs
            WHERE user_id = ?
            ORDER BY id DESC
            LIMIT ?
            "#,
        )?;

        let logs = stmt
            .query_map(params![user_id, limit as i64], |row| {
                Ok(ScriptLog {
                    id: row.get(0)?,
                    script_id: row.get(1)?,
                    user_id: row.get(2)?,
                    executed_at: row.get(3)?,
                    execution_ms: row.get(4)?,
                    success: row.get::<_, i32>(5)? != 0,
                    error_message: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(logs)
    }

    /// Get execution count for a script.
    pub fn get_execution_count(&self, script_id: i64) -> Result<i64> {
        let conn = self.db.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM script_logs WHERE script_id = ?",
            params![script_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Get success rate for a script (as percentage 0-100).
    pub fn get_success_rate(&self, script_id: i64) -> Result<Option<f64>> {
        let conn = self.db.conn();
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM script_logs WHERE script_id = ?",
            params![script_id],
            |row| row.get(0),
        )?;

        if total == 0 {
            return Ok(None);
        }

        let success: i64 = conn.query_row(
            "SELECT COUNT(*) FROM script_logs WHERE script_id = ? AND success = 1",
            params![script_id],
            |row| row.get(0),
        )?;

        Ok(Some((success as f64 / total as f64) * 100.0))
    }

    /// Get average execution time for a script (in milliseconds).
    pub fn get_avg_execution_time(&self, script_id: i64) -> Result<Option<f64>> {
        let conn = self.db.conn();
        let avg: Option<f64> = conn
            .query_row(
                "SELECT AVG(execution_ms) FROM script_logs WHERE script_id = ?",
                params![script_id],
                |row| row.get(0),
            )
            .ok();

        Ok(avg)
    }

    /// Delete old logs (older than specified days).
    pub fn delete_old_logs(&self, days: i32) -> Result<usize> {
        let conn = self.db.conn();
        let affected = conn.execute(
            "DELETE FROM script_logs WHERE executed_at < datetime('now', ?)",
            params![format!("-{} days", days)],
        )?;

        Ok(affected)
    }

    /// Delete all logs for a script.
    pub fn delete_by_script(&self, script_id: i64) -> Result<usize> {
        let conn = self.db.conn();
        let affected = conn.execute(
            "DELETE FROM script_logs WHERE script_id = ?",
            params![script_id],
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
    fn test_log_execution_success() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let user_id = create_test_user(&db);
        let repo = ScriptLogRepository::new(&db);

        let log_id = repo
            .log_execution(script_id, Some(user_id), 100, true, None)
            .unwrap();

        assert!(log_id > 0);

        let logs = repo.get_by_script(script_id, 10).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].script_id, script_id);
        assert_eq!(logs[0].user_id, Some(user_id));
        assert_eq!(logs[0].execution_ms, 100);
        assert!(logs[0].success);
        assert!(logs[0].error_message.is_none());
    }

    #[test]
    fn test_log_execution_error() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let repo = ScriptLogRepository::new(&db);

        repo.log_execution(
            script_id,
            None,
            50,
            false,
            Some("Script error: undefined variable"),
        )
        .unwrap();

        let logs = repo.get_by_script(script_id, 10).unwrap();
        assert_eq!(logs.len(), 1);
        assert!(!logs[0].success);
        assert_eq!(
            logs[0].error_message,
            Some("Script error: undefined variable".to_string())
        );
    }

    #[test]
    fn test_get_by_user() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let user_id = create_test_user(&db);
        let repo = ScriptLogRepository::new(&db);

        repo.log_execution(script_id, Some(user_id), 100, true, None)
            .unwrap();
        repo.log_execution(script_id, Some(user_id), 150, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 200, true, None)
            .unwrap(); // Guest

        let user_logs = repo.get_by_user(user_id, 10).unwrap();
        assert_eq!(user_logs.len(), 2);
    }

    #[test]
    fn test_get_execution_count() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let repo = ScriptLogRepository::new(&db);

        // Initially 0
        assert_eq!(repo.get_execution_count(script_id).unwrap(), 0);

        repo.log_execution(script_id, None, 100, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 150, false, Some("error"))
            .unwrap();

        assert_eq!(repo.get_execution_count(script_id).unwrap(), 2);
    }

    #[test]
    fn test_get_success_rate() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let repo = ScriptLogRepository::new(&db);

        // No logs - None
        assert!(repo.get_success_rate(script_id).unwrap().is_none());

        // 2 success, 1 failure = 66.67%
        repo.log_execution(script_id, None, 100, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 100, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 100, false, Some("error"))
            .unwrap();

        let rate = repo.get_success_rate(script_id).unwrap().unwrap();
        assert!((rate - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_get_avg_execution_time() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let repo = ScriptLogRepository::new(&db);

        // No logs - None
        assert!(repo.get_avg_execution_time(script_id).unwrap().is_none());

        repo.log_execution(script_id, None, 100, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 200, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 300, true, None)
            .unwrap();

        let avg = repo.get_avg_execution_time(script_id).unwrap().unwrap();
        assert!((avg - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_delete_by_script() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let repo = ScriptLogRepository::new(&db);

        repo.log_execution(script_id, None, 100, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 100, true, None)
            .unwrap();

        let deleted = repo.delete_by_script(script_id).unwrap();
        assert_eq!(deleted, 2);

        assert_eq!(repo.get_execution_count(script_id).unwrap(), 0);
    }

    #[test]
    fn test_logs_ordered_by_time() {
        let db = create_test_db();
        let script_id = create_test_script(&db);
        let repo = ScriptLogRepository::new(&db);

        repo.log_execution(script_id, None, 100, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 200, true, None)
            .unwrap();
        repo.log_execution(script_id, None, 300, true, None)
            .unwrap();

        let logs = repo.get_by_script(script_id, 10).unwrap();
        // Should be ordered by executed_at DESC, so newest first
        assert_eq!(logs[0].execution_ms, 300);
        assert_eq!(logs[1].execution_ms, 200);
        assert_eq!(logs[2].execution_ms, 100);
    }
}
