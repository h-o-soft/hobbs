//! Script repository for database operations.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use super::types::Script;
use crate::db::{DbPool, SQL_TRUE};
use crate::{HobbsError, Result};

/// Database row for Script.
#[derive(Debug, sqlx::FromRow)]
struct ScriptRow {
    id: i64,
    file_path: String,
    name: String,
    slug: String,
    description: Option<String>,
    author: Option<String>,
    file_hash: Option<String>,
    synced_at: Option<String>,
    min_role: i32,
    enabled: bool,
    max_instructions: i64,
    max_memory_mb: i32,
    max_execution_seconds: i32,
    name_i18n: Option<String>,
    description_i18n: Option<String>,
}

impl ScriptRow {
    fn into_script(self) -> Script {
        let synced_at = self.synced_at.and_then(|s| {
            chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        });

        let name_i18n: HashMap<String, String> = self
            .name_i18n
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let description_i18n: HashMap<String, String> = self
            .description_i18n
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Script {
            id: self.id,
            file_path: self.file_path,
            name: self.name,
            slug: self.slug,
            description: self.description,
            author: self.author,
            file_hash: self.file_hash,
            synced_at,
            min_role: self.min_role,
            enabled: self.enabled,
            max_instructions: self.max_instructions,
            max_memory_mb: self.max_memory_mb,
            max_execution_seconds: self.max_execution_seconds,
            name_i18n,
            description_i18n,
        }
    }
}

/// Repository for script CRUD operations.
pub struct ScriptRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> ScriptRepository<'a> {
    /// Create a new ScriptRepository with the given database pool reference.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// List all enabled scripts that the given role can execute.
    pub async fn list(&self, user_role: i32) -> Result<Vec<Script>> {
        let rows = sqlx::query_as::<_, ScriptRow>(&format!(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds, name_i18n, description_i18n
             FROM scripts
             WHERE enabled = {} AND min_role <= $1
             ORDER BY name",
            SQL_TRUE
        ))
        .bind(user_role)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into_script()).collect())
    }

    /// List all scripts (for admin).
    pub async fn list_all(&self) -> Result<Vec<Script>> {
        let rows = sqlx::query_as::<_, ScriptRow>(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds, name_i18n, description_i18n
             FROM scripts
             ORDER BY name",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into_script()).collect())
    }

    /// Get a script by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Script>> {
        let result = sqlx::query_as::<_, ScriptRow>(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds, name_i18n, description_i18n
             FROM scripts WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(|r| r.into_script()))
    }

    /// Get a script by slug.
    pub async fn get_by_slug(&self, slug: &str) -> Result<Option<Script>> {
        let result = sqlx::query_as::<_, ScriptRow>(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds, name_i18n, description_i18n
             FROM scripts WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(|r| r.into_script()))
    }

    /// Get a script by file path.
    pub async fn get_by_file_path(&self, file_path: &str) -> Result<Option<Script>> {
        let result = sqlx::query_as::<_, ScriptRow>(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds, name_i18n, description_i18n
             FROM scripts WHERE file_path = $1",
        )
        .bind(file_path)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(|r| r.into_script()))
    }

    /// Insert or update a script (upsert).
    pub async fn upsert(&self, script: &Script) -> Result<Script> {
        let synced_at = script
            .synced_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        // Serialize i18n data to JSON (None if empty)
        let name_i18n_json = if script.name_i18n.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&script.name_i18n).unwrap_or_default())
        };

        let description_i18n_json = if script.description_i18n.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&script.description_i18n).unwrap_or_default())
        };

        sqlx::query(
            "INSERT INTO scripts (file_path, name, slug, description, author, file_hash,
                                  synced_at, min_role, enabled, max_instructions,
                                  max_memory_mb, max_execution_seconds, name_i18n, description_i18n)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT(file_path) DO UPDATE SET
                name = EXCLUDED.name,
                slug = EXCLUDED.slug,
                description = EXCLUDED.description,
                author = EXCLUDED.author,
                file_hash = EXCLUDED.file_hash,
                synced_at = EXCLUDED.synced_at,
                min_role = EXCLUDED.min_role,
                max_instructions = EXCLUDED.max_instructions,
                max_memory_mb = EXCLUDED.max_memory_mb,
                max_execution_seconds = EXCLUDED.max_execution_seconds,
                name_i18n = EXCLUDED.name_i18n,
                description_i18n = EXCLUDED.description_i18n",
        )
        .bind(&script.file_path)
        .bind(&script.name)
        .bind(&script.slug)
        .bind(&script.description)
        .bind(&script.author)
        .bind(&script.file_hash)
        .bind(&synced_at)
        .bind(script.min_role)
        .bind(script.enabled)
        .bind(script.max_instructions)
        .bind(script.max_memory_mb)
        .bind(script.max_execution_seconds)
        .bind(&name_i18n_json)
        .bind(&description_i18n_json)
        .execute(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_file_path(&script.file_path)
            .await?
            .ok_or_else(|| HobbsError::NotFound("script".to_string()))
    }

    /// Update the enabled status of a script.
    pub async fn update_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let result = sqlx::query("UPDATE scripts SET enabled = $1 WHERE id = $2")
            .bind(enabled)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            Err(HobbsError::NotFound("script".to_string()))
        } else {
            Ok(())
        }
    }

    /// Delete a script by ID.
    pub async fn delete(&self, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM scripts WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            Err(HobbsError::NotFound("script".to_string()))
        } else {
            Ok(())
        }
    }

    /// Delete a script by file path.
    pub async fn delete_by_file_path(&self, file_path: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM scripts WHERE file_path = $1")
            .bind(file_path)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            Err(HobbsError::NotFound("script".to_string()))
        } else {
            Ok(())
        }
    }

    /// List all file paths in the database (for sync).
    pub async fn list_all_file_paths(&self) -> Result<Vec<String>> {
        let paths: Vec<(String,)> = sqlx::query_as("SELECT file_path FROM scripts")
            .fetch_all(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(paths.into_iter().map(|(p,)| p).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    fn create_test_script(file_path: &str) -> Script {
        Script {
            id: 0,
            file_path: file_path.to_string(),
            name: "Test Script".to_string(),
            slug: file_path.replace(".lua", "").replace('/', "_"),
            description: Some("A test script".to_string()),
            author: Some("TestAuthor".to_string()),
            name_i18n: HashMap::new(),
            description_i18n: HashMap::new(),
            file_hash: Some("abc123".to_string()),
            synced_at: Some(Utc::now()),
            min_role: 0,
            enabled: true,
            max_instructions: 1000000,
            max_memory_mb: 10,
            max_execution_seconds: 30,
        }
    }

    #[tokio::test]
    async fn test_upsert_and_get() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        let script = create_test_script("test.lua");
        let created = repo.upsert(&script).await.unwrap();

        assert!(created.id > 0);
        assert_eq!(created.file_path, "test.lua");
        assert_eq!(created.name, "Test Script");

        // Get by ID
        let fetched = repo.get_by_id(created.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "Test Script");

        // Get by slug
        let fetched = repo.get_by_slug(&created.slug).await.unwrap().unwrap();
        assert_eq!(fetched.name, "Test Script");

        // Get by file path
        let fetched = repo.get_by_file_path("test.lua").await.unwrap().unwrap();
        assert_eq!(fetched.name, "Test Script");
    }

    #[tokio::test]
    async fn test_upsert_updates_existing() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        let mut script = create_test_script("test.lua");
        let created = repo.upsert(&script).await.unwrap();

        // Update the script
        script.name = "Updated Script".to_string();
        script.description = Some("Updated description".to_string());
        let updated = repo.upsert(&script).await.unwrap();

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "Updated Script");
        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_list_by_role() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        // Create scripts with different min_role
        let mut guest_script = create_test_script("guest.lua");
        guest_script.min_role = 0;
        repo.upsert(&guest_script).await.unwrap();

        let mut member_script = create_test_script("member.lua");
        member_script.min_role = 1;
        repo.upsert(&member_script).await.unwrap();

        let mut sysop_script = create_test_script("sysop.lua");
        sysop_script.min_role = 3;
        repo.upsert(&sysop_script).await.unwrap();

        // Guest (0) should see 1 script
        let guest_list = repo.list(0).await.unwrap();
        assert_eq!(guest_list.len(), 1);

        // Member (1) should see 2 scripts
        let member_list = repo.list(1).await.unwrap();
        assert_eq!(member_list.len(), 2);

        // SysOp (3) should see 3 scripts
        let sysop_list = repo.list(3).await.unwrap();
        assert_eq!(sysop_list.len(), 3);
    }

    #[tokio::test]
    async fn test_update_enabled() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        let script = create_test_script("test.lua");
        let created = repo.upsert(&script).await.unwrap();

        // Disable
        repo.update_enabled(created.id, false).await.unwrap();
        let fetched = repo.get_by_id(created.id).await.unwrap().unwrap();
        assert!(!fetched.enabled);

        // Enable
        repo.update_enabled(created.id, true).await.unwrap();
        let fetched = repo.get_by_id(created.id).await.unwrap().unwrap();
        assert!(fetched.enabled);
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        let script = create_test_script("test.lua");
        let created = repo.upsert(&script).await.unwrap();

        repo.delete(created.id).await.unwrap();
        let fetched = repo.get_by_id(created.id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_delete_by_file_path() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        let script = create_test_script("test.lua");
        repo.upsert(&script).await.unwrap();

        repo.delete_by_file_path("test.lua").await.unwrap();
        let fetched = repo.get_by_file_path("test.lua").await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_list_all_file_paths() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        repo.upsert(&create_test_script("a.lua")).await.unwrap();
        repo.upsert(&create_test_script("b.lua")).await.unwrap();
        repo.upsert(&create_test_script("c.lua")).await.unwrap();

        let paths = repo.list_all_file_paths().await.unwrap();
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&"a.lua".to_string()));
        assert!(paths.contains(&"b.lua".to_string()));
        assert!(paths.contains(&"c.lua".to_string()));
    }

    #[tokio::test]
    async fn test_disabled_scripts_not_in_list() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        let mut enabled_script = create_test_script("enabled.lua");
        enabled_script.enabled = true;
        repo.upsert(&enabled_script).await.unwrap();

        let mut disabled_script = create_test_script("disabled.lua");
        disabled_script.enabled = false;
        repo.upsert(&disabled_script).await.unwrap();

        let list = repo.list(3).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].file_path, "enabled.lua");

        // But list_all should include disabled
        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_i18n_metadata_persistence() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        // Create script with i18n metadata
        let mut script = create_test_script("i18n_test.lua");
        script.name = "Test Script".to_string();
        script.description = Some("A test script".to_string());
        script
            .name_i18n
            .insert("ja".to_string(), "テストスクリプト".to_string());
        script
            .name_i18n
            .insert("de".to_string(), "Testskript".to_string());
        script
            .description_i18n
            .insert("ja".to_string(), "これはテストです".to_string());

        let created = repo.upsert(&script).await.unwrap();

        // Verify i18n data was saved
        assert_eq!(
            created.name_i18n.get("ja"),
            Some(&"テストスクリプト".to_string())
        );
        assert_eq!(created.name_i18n.get("de"), Some(&"Testskript".to_string()));
        assert_eq!(
            created.description_i18n.get("ja"),
            Some(&"これはテストです".to_string())
        );

        // Verify get_name() and get_description() work correctly
        assert_eq!(created.get_name("ja"), "テストスクリプト");
        assert_eq!(created.get_name("de"), "Testskript");
        assert_eq!(created.get_name("en"), "Test Script"); // Falls back to default
        assert_eq!(created.get_description("ja"), Some("これはテストです"));
        assert_eq!(created.get_description("en"), Some("A test script")); // Falls back to default

        // Verify data persists when fetched
        let fetched = repo.get_by_id(created.id).await.unwrap().unwrap();
        assert_eq!(fetched.name_i18n.len(), 2);
        assert_eq!(fetched.description_i18n.len(), 1);
        assert_eq!(fetched.get_name("ja"), "テストスクリプト");
    }

    #[tokio::test]
    async fn test_i18n_empty_maps() {
        let db = setup_db().await;
        let repo = ScriptRepository::new(db.pool());

        // Create script without i18n metadata
        let script = create_test_script("no_i18n.lua");
        let created = repo.upsert(&script).await.unwrap();

        // Should have empty i18n maps
        assert!(created.name_i18n.is_empty());
        assert!(created.description_i18n.is_empty());

        // get_name() should fall back to default
        assert_eq!(created.get_name("ja"), "Test Script");
        assert_eq!(created.get_name("en"), "Test Script");
    }
}
