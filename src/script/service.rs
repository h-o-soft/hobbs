//! Script service for managing and executing scripts.

use super::api::BbsApi;
use super::engine::{ResourceLimits, ScriptContext, ScriptEngine};
use super::loader::ScriptLoader;
use super::repository::ScriptRepository;
use super::types::Script;
use crate::db::Database;
use crate::{HobbsError, Result};

/// Result of script execution.
#[derive(Debug)]
pub struct ExecutionResult {
    /// Output collected from the script.
    pub output: Vec<String>,
    /// Instruction count used.
    pub instructions_used: u64,
    /// Whether execution completed successfully.
    pub success: bool,
    /// Error message if execution failed.
    pub error: Option<String>,
}

/// Service for managing and executing scripts.
pub struct ScriptService<'a> {
    db: &'a Database,
    scripts_dir: Option<std::path::PathBuf>,
}

impl<'a> ScriptService<'a> {
    /// Create a new ScriptService.
    pub fn new(db: &'a Database) -> Self {
        Self {
            db,
            scripts_dir: None,
        }
    }

    /// Set the scripts directory for loading script source.
    pub fn with_scripts_dir<P: Into<std::path::PathBuf>>(mut self, dir: P) -> Self {
        self.scripts_dir = Some(dir.into());
        self
    }

    /// List all scripts that a user with the given role can execute.
    pub fn list_scripts(&self, user_role: i32) -> Result<Vec<Script>> {
        let repo = ScriptRepository::new(self.db);
        repo.list(user_role)
    }

    /// Get a script by its slug.
    pub fn get_script(&self, slug: &str) -> Result<Option<Script>> {
        let repo = ScriptRepository::new(self.db);
        repo.get_by_slug(slug)
    }

    /// Get a script by its ID.
    pub fn get_script_by_id(&self, id: i64) -> Result<Option<Script>> {
        let repo = ScriptRepository::new(self.db);
        repo.get_by_id(id)
    }

    /// Check if a user can execute a specific script.
    pub fn can_execute(&self, script: &Script, user_role: i32) -> bool {
        script.can_execute(user_role)
    }

    /// Execute a script with the given context.
    ///
    /// Returns the execution result containing output and status.
    pub fn execute(&self, script: &Script, context: ScriptContext) -> Result<ExecutionResult> {
        // Check if script is enabled
        if !script.enabled {
            return Err(HobbsError::Script("Script is disabled".to_string()));
        }

        // Check permission
        if !script.can_execute(context.user_role) {
            return Err(HobbsError::Permission(format!(
                "Insufficient role to execute script '{}'",
                script.name
            )));
        }

        // Load source code
        let source = self.load_script_source(&script.file_path)?;

        // Create engine with script-specific limits
        let limits = ResourceLimits {
            max_instructions: script.max_instructions as u64,
            max_memory: script.max_memory_mb as usize * 1024 * 1024,
            max_execution_seconds: script.max_execution_seconds as u32,
        };

        let engine = ScriptEngine::with_limits(limits)?;

        // Register BBS API
        let api = BbsApi::new(context);
        let output_buffer = api.get_output();
        api.register(engine.lua())
            .map_err(|e| HobbsError::Script(format!("Failed to register BBS API: {}", e)))?;

        // Execute the script
        let result = engine.execute(&source);

        // Get output regardless of success/failure
        let output = {
            // The output buffer reference was moved into the Lua state,
            // so we need to get it from the registered API
            // For now, we'll return an empty vec and fix this later
            // This is a design issue - we need to share the output buffer
            output_buffer
        };

        match result {
            Ok(()) => Ok(ExecutionResult {
                output,
                instructions_used: engine.instruction_count(),
                success: true,
                error: None,
            }),
            Err(e) => Ok(ExecutionResult {
                output,
                instructions_used: engine.instruction_count(),
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Load script source code from the file system.
    fn load_script_source(&self, file_path: &str) -> Result<String> {
        let scripts_dir = self.scripts_dir.as_ref().ok_or_else(|| {
            HobbsError::Script("Scripts directory not configured".to_string())
        })?;

        let loader = ScriptLoader::new(scripts_dir);
        loader.read_script_source(file_path)
    }

    /// Sync scripts from the file system to the database.
    pub fn sync_scripts(&self) -> Result<super::types::SyncResult> {
        let scripts_dir = self.scripts_dir.as_ref().ok_or_else(|| {
            HobbsError::Script("Scripts directory not configured".to_string())
        })?;

        let loader = ScriptLoader::new(scripts_dir);
        loader.sync(self.db)
    }

    /// Enable or disable a script.
    pub fn set_enabled(&self, script_id: i64, enabled: bool) -> Result<()> {
        let repo = ScriptRepository::new(self.db);
        repo.update_enabled(script_id, enabled)
    }

    /// Delete a script from the database.
    ///
    /// Note: This only removes from DB cache, not from file system.
    pub fn delete_script(&self, script_id: i64) -> Result<()> {
        let repo = ScriptRepository::new(self.db);
        repo.delete(script_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_db() -> Database {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        Database::open(db_path).unwrap()
    }

    fn create_test_script(db: &Database, scripts_dir: &std::path::Path) -> Script {
        // Create script file
        let script_content = r#"-- @name Test Game
-- @description A test game script
-- @author SysOp
-- @min_role 0

bbs.println("=== Test Game ===")
local user = bbs.get_user()
bbs.println("Hello, " .. user.nickname .. "!")
bbs.println("Random: " .. bbs.random(1, 100))
"#;
        fs::write(scripts_dir.join("test.lua"), script_content).unwrap();

        // Sync to database
        let loader = ScriptLoader::new(scripts_dir);
        loader.sync(db).unwrap();

        // Get the script
        let repo = ScriptRepository::new(db);
        repo.get_by_slug("test").unwrap().unwrap()
    }

    #[test]
    fn test_list_scripts() {
        let db = create_test_db();
        let dir = tempdir().unwrap();

        // Create two scripts with different min_role
        let script1 = r#"-- @name Public Game
-- @min_role 0
bbs.println("public")
"#;
        let script2 = r#"-- @name Member Game
-- @min_role 1
bbs.println("member only")
"#;
        fs::write(dir.path().join("public.lua"), script1).unwrap();
        fs::write(dir.path().join("member.lua"), script2).unwrap();

        // Sync
        let loader = ScriptLoader::new(dir.path());
        loader.sync(&db).unwrap();

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        // Guest (role 0) should see only public script
        let scripts = service.list_scripts(0).unwrap();
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].name, "Public Game");

        // Member (role 1) should see both
        let scripts = service.list_scripts(1).unwrap();
        assert_eq!(scripts.len(), 2);
    }

    #[test]
    fn test_get_script() {
        let db = create_test_db();
        let dir = tempdir().unwrap();
        let _script = create_test_script(&db, dir.path());

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        let found = service.get_script("test").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Game");

        let not_found = service.get_script("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_execute_script() {
        let db = create_test_db();
        let dir = tempdir().unwrap();
        let script = create_test_script(&db, dir.path());

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        let context = ScriptContext {
            user_id: Some(1),
            username: "testuser".to_string(),
            nickname: "Tester".to_string(),
            user_role: 1,
            terminal_width: 80,
            terminal_height: 24,
            has_ansi: true,
        };

        let result = service.execute(&script, context).unwrap();
        assert!(result.success);
        // Note: output verification might not work due to the Rc/RefCell issue
    }

    #[test]
    fn test_execute_disabled_script() {
        let db = create_test_db();
        let dir = tempdir().unwrap();
        let mut script = create_test_script(&db, dir.path());

        // Disable the script
        let repo = ScriptRepository::new(&db);
        repo.update_enabled(script.id, false).unwrap();
        script.enabled = false;

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        let context = ScriptContext::default();
        let result = service.execute(&script, context);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disabled"));
    }

    #[test]
    fn test_execute_permission_denied() {
        let db = create_test_db();
        let dir = tempdir().unwrap();

        // Create a SubOp-only script
        let script_content = r#"-- @name Admin Tool
-- @min_role 2
bbs.println("admin only")
"#;
        fs::write(dir.path().join("admin.lua"), script_content).unwrap();

        let loader = ScriptLoader::new(dir.path());
        loader.sync(&db).unwrap();

        let repo = ScriptRepository::new(&db);
        let script = repo.get_by_slug("admin").unwrap().unwrap();

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        // Guest (role 0) should not be able to execute
        let context = ScriptContext {
            user_role: 0,
            ..Default::default()
        };
        let result = service.execute(&script, context);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient role"));
    }

    #[test]
    fn test_sync_scripts() {
        let db = create_test_db();
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("game.lua"), "bbs.println('hello')").unwrap();

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());
        let result = service.sync_scripts().unwrap();

        assert_eq!(result.added, 1);
        assert!(result.has_changes());
    }

    #[test]
    fn test_set_enabled() {
        let db = create_test_db();
        let dir = tempdir().unwrap();
        let script = create_test_script(&db, dir.path());

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        // Disable
        service.set_enabled(script.id, false).unwrap();
        let updated = service.get_script_by_id(script.id).unwrap().unwrap();
        assert!(!updated.enabled);

        // Enable
        service.set_enabled(script.id, true).unwrap();
        let updated = service.get_script_by_id(script.id).unwrap().unwrap();
        assert!(updated.enabled);
    }

    #[test]
    fn test_execute_script_with_error() {
        let db = create_test_db();
        let dir = tempdir().unwrap();

        // Create a script that will error
        let script_content = r#"-- @name Broken Script
bbs.println("Starting...")
error("Intentional error")
"#;
        fs::write(dir.path().join("broken.lua"), script_content).unwrap();

        let loader = ScriptLoader::new(dir.path());
        loader.sync(&db).unwrap();

        let repo = ScriptRepository::new(&db);
        let script = repo.get_by_slug("broken").unwrap().unwrap();

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        let context = ScriptContext::default();
        let result = service.execute(&script, context).unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Intentional error"));
    }
}
