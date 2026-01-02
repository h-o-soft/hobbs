//! Script repository for database operations.

use chrono::{DateTime, Utc};
use rusqlite::{params, Row};

use super::types::Script;
use crate::db::Database;
use crate::{HobbsError, Result};

/// Repository for script CRUD operations.
pub struct ScriptRepository<'a> {
    db: &'a Database,
}

impl<'a> ScriptRepository<'a> {
    /// Create a new ScriptRepository with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// List all enabled scripts that the given role can execute.
    pub fn list(&self, user_role: i32) -> Result<Vec<Script>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds
             FROM scripts
             WHERE enabled = 1 AND min_role <= ?
             ORDER BY name",
        )?;

        let scripts = stmt
            .query_map([user_role], Self::row_to_script)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(scripts)
    }

    /// List all scripts (for admin).
    pub fn list_all(&self) -> Result<Vec<Script>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds
             FROM scripts
             ORDER BY name",
        )?;

        let scripts = stmt
            .query_map([], Self::row_to_script)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(scripts)
    }

    /// Get a script by ID.
    pub fn get_by_id(&self, id: i64) -> Result<Option<Script>> {
        let result = self.db.conn().query_row(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds
             FROM scripts WHERE id = ?",
            [id],
            Self::row_to_script,
        );

        match result {
            Ok(script) => Ok(Some(script)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a script by slug.
    pub fn get_by_slug(&self, slug: &str) -> Result<Option<Script>> {
        let result = self.db.conn().query_row(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds
             FROM scripts WHERE slug = ?",
            [slug],
            Self::row_to_script,
        );

        match result {
            Ok(script) => Ok(Some(script)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a script by file path.
    pub fn get_by_file_path(&self, file_path: &str) -> Result<Option<Script>> {
        let result = self.db.conn().query_row(
            "SELECT id, file_path, name, slug, description, author, file_hash,
                    synced_at, min_role, enabled, max_instructions, max_memory_mb,
                    max_execution_seconds
             FROM scripts WHERE file_path = ?",
            [file_path],
            Self::row_to_script,
        );

        match result {
            Ok(script) => Ok(Some(script)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert or update a script (upsert).
    pub fn upsert(&self, script: &Script) -> Result<Script> {
        let synced_at = script
            .synced_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        self.db.conn().execute(
            "INSERT INTO scripts (file_path, name, slug, description, author, file_hash,
                                  synced_at, min_role, enabled, max_instructions,
                                  max_memory_mb, max_execution_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(file_path) DO UPDATE SET
                name = ?2,
                slug = ?3,
                description = ?4,
                author = ?5,
                file_hash = ?6,
                synced_at = ?7,
                min_role = ?8,
                max_instructions = ?10,
                max_memory_mb = ?11,
                max_execution_seconds = ?12",
            params![
                &script.file_path,
                &script.name,
                &script.slug,
                &script.description,
                &script.author,
                &script.file_hash,
                &synced_at,
                script.min_role,
                script.enabled,
                script.max_instructions,
                script.max_memory_mb,
                script.max_execution_seconds,
            ],
        )?;

        self.get_by_file_path(&script.file_path)?
            .ok_or_else(|| HobbsError::NotFound("script".to_string()))
    }

    /// Update the enabled status of a script.
    pub fn update_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let rows = self.db.conn().execute(
            "UPDATE scripts SET enabled = ? WHERE id = ?",
            params![enabled, id],
        )?;

        if rows == 0 {
            Err(HobbsError::NotFound("script".to_string()))
        } else {
            Ok(())
        }
    }

    /// Delete a script by ID.
    pub fn delete(&self, id: i64) -> Result<()> {
        let rows = self
            .db
            .conn()
            .execute("DELETE FROM scripts WHERE id = ?", [id])?;

        if rows == 0 {
            Err(HobbsError::NotFound("script".to_string()))
        } else {
            Ok(())
        }
    }

    /// Delete a script by file path.
    pub fn delete_by_file_path(&self, file_path: &str) -> Result<()> {
        let rows = self
            .db
            .conn()
            .execute("DELETE FROM scripts WHERE file_path = ?", [file_path])?;

        if rows == 0 {
            Err(HobbsError::NotFound("script".to_string()))
        } else {
            Ok(())
        }
    }

    /// List all file paths in the database (for sync).
    pub fn list_all_file_paths(&self) -> Result<Vec<String>> {
        let mut stmt = self.db.conn().prepare("SELECT file_path FROM scripts")?;

        let paths = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(paths)
    }

    /// Convert a database row to a Script.
    fn row_to_script(row: &Row) -> rusqlite::Result<Script> {
        let synced_at_str: Option<String> = row.get(7)?;
        let synced_at = synced_at_str.and_then(|s| {
            chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        });

        Ok(Script {
            id: row.get(0)?,
            file_path: row.get(1)?,
            name: row.get(2)?,
            slug: row.get(3)?,
            description: row.get(4)?,
            author: row.get(5)?,
            file_hash: row.get(6)?,
            synced_at,
            min_role: row.get(8)?,
            enabled: row.get(9)?,
            max_instructions: row.get(10)?,
            max_memory_mb: row.get(11)?,
            max_execution_seconds: row.get(12)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_db() -> Database {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        Database::open(db_path).unwrap()
    }

    fn create_test_script(file_path: &str) -> Script {
        Script {
            id: 0,
            file_path: file_path.to_string(),
            name: "Test Script".to_string(),
            slug: file_path.replace(".lua", "").replace('/', "_"),
            description: Some("A test script".to_string()),
            author: Some("TestAuthor".to_string()),
            file_hash: Some("abc123".to_string()),
            synced_at: Some(Utc::now()),
            min_role: 0,
            enabled: true,
            max_instructions: 1000000,
            max_memory_mb: 10,
            max_execution_seconds: 30,
        }
    }

    #[test]
    fn test_upsert_and_get() {
        let db = create_test_db();
        let repo = ScriptRepository::new(&db);

        let script = create_test_script("test.lua");
        let created = repo.upsert(&script).unwrap();

        assert!(created.id > 0);
        assert_eq!(created.file_path, "test.lua");
        assert_eq!(created.name, "Test Script");

        // Get by ID
        let fetched = repo.get_by_id(created.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Test Script");

        // Get by slug
        let fetched = repo.get_by_slug(&created.slug).unwrap().unwrap();
        assert_eq!(fetched.name, "Test Script");

        // Get by file path
        let fetched = repo.get_by_file_path("test.lua").unwrap().unwrap();
        assert_eq!(fetched.name, "Test Script");
    }

    #[test]
    fn test_upsert_updates_existing() {
        let db = create_test_db();
        let repo = ScriptRepository::new(&db);

        let mut script = create_test_script("test.lua");
        let created = repo.upsert(&script).unwrap();

        // Update the script
        script.name = "Updated Script".to_string();
        script.description = Some("Updated description".to_string());
        let updated = repo.upsert(&script).unwrap();

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "Updated Script");
        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[test]
    fn test_list_by_role() {
        let db = create_test_db();
        let repo = ScriptRepository::new(&db);

        // Create scripts with different min_role
        let mut guest_script = create_test_script("guest.lua");
        guest_script.min_role = 0;
        repo.upsert(&guest_script).unwrap();

        let mut member_script = create_test_script("member.lua");
        member_script.min_role = 1;
        repo.upsert(&member_script).unwrap();

        let mut sysop_script = create_test_script("sysop.lua");
        sysop_script.min_role = 3;
        repo.upsert(&sysop_script).unwrap();

        // Guest (0) should see 1 script
        let guest_list = repo.list(0).unwrap();
        assert_eq!(guest_list.len(), 1);

        // Member (1) should see 2 scripts
        let member_list = repo.list(1).unwrap();
        assert_eq!(member_list.len(), 2);

        // SysOp (3) should see 3 scripts
        let sysop_list = repo.list(3).unwrap();
        assert_eq!(sysop_list.len(), 3);
    }

    #[test]
    fn test_update_enabled() {
        let db = create_test_db();
        let repo = ScriptRepository::new(&db);

        let script = create_test_script("test.lua");
        let created = repo.upsert(&script).unwrap();

        // Disable
        repo.update_enabled(created.id, false).unwrap();
        let fetched = repo.get_by_id(created.id).unwrap().unwrap();
        assert!(!fetched.enabled);

        // Enable
        repo.update_enabled(created.id, true).unwrap();
        let fetched = repo.get_by_id(created.id).unwrap().unwrap();
        assert!(fetched.enabled);
    }

    #[test]
    fn test_delete() {
        let db = create_test_db();
        let repo = ScriptRepository::new(&db);

        let script = create_test_script("test.lua");
        let created = repo.upsert(&script).unwrap();

        repo.delete(created.id).unwrap();
        let fetched = repo.get_by_id(created.id).unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_delete_by_file_path() {
        let db = create_test_db();
        let repo = ScriptRepository::new(&db);

        let script = create_test_script("test.lua");
        repo.upsert(&script).unwrap();

        repo.delete_by_file_path("test.lua").unwrap();
        let fetched = repo.get_by_file_path("test.lua").unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_list_all_file_paths() {
        let db = create_test_db();
        let repo = ScriptRepository::new(&db);

        repo.upsert(&create_test_script("a.lua")).unwrap();
        repo.upsert(&create_test_script("b.lua")).unwrap();
        repo.upsert(&create_test_script("c.lua")).unwrap();

        let paths = repo.list_all_file_paths().unwrap();
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&"a.lua".to_string()));
        assert!(paths.contains(&"b.lua".to_string()));
        assert!(paths.contains(&"c.lua".to_string()));
    }

    #[test]
    fn test_disabled_scripts_not_in_list() {
        let db = create_test_db();
        let repo = ScriptRepository::new(&db);

        let mut enabled_script = create_test_script("enabled.lua");
        enabled_script.enabled = true;
        repo.upsert(&enabled_script).unwrap();

        let mut disabled_script = create_test_script("disabled.lua");
        disabled_script.enabled = false;
        repo.upsert(&disabled_script).unwrap();

        let list = repo.list(3).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].file_path, "enabled.lua");

        // But list_all should include disabled
        let all = repo.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }
}
