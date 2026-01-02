//! Script service for managing and executing scripts.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use mlua::Table;

use super::api::BbsApi;
use super::data_repository::ScriptDataRepository;
use super::engine::{ResourceLimits, ScriptContext, ScriptEngine};
use super::loader::ScriptLoader;
use super::log_repository::ScriptLogRepository;
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
    /// Note: This method does not support interactive input. Use `execute_with_input`
    /// for scripts that require user input.
    pub fn execute(&self, script: &Script, context: ScriptContext) -> Result<ExecutionResult> {
        self.execute_with_input(script, context, None)
    }

    /// Execute a script with the given context and optional input bridge.
    ///
    /// Returns the execution result containing output and status.
    /// If an input bridge is provided, scripts can request interactive input.
    pub fn execute_with_input(
        &self,
        script: &Script,
        mut context: ScriptContext,
        input_bridge: Option<std::sync::Arc<super::input_bridge::ScriptInputHandle>>,
    ) -> Result<ExecutionResult> {
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

        // Set script_id in context
        context.script_id = Some(script.id);

        // Load source code
        let source = self.load_script_source(&script.file_path)?;

        // Create engine with script-specific limits
        let limits = ResourceLimits {
            max_instructions: script.max_instructions as u64,
            max_memory: script.max_memory_mb as usize * 1024 * 1024,
            max_execution_seconds: script.max_execution_seconds as u32,
        };

        let engine = ScriptEngine::with_limits(limits)?;

        // Load existing data from database
        let data_repo = ScriptDataRepository::new(self.db);
        let global_data = Rc::new(RefCell::new(self.load_global_data(&data_repo, script.id)?));
        let user_data = Rc::new(RefCell::new(self.load_user_data(
            &data_repo,
            script.id,
            context.user_id,
        )?));

        // Register BBS API with optional input bridge
        let mut api = BbsApi::new(context.clone());
        if let Some(bridge) = input_bridge {
            api = api.with_input_bridge(bridge);
        }
        let output_buffer = api.output_buffer_ref(); // Get shared reference before register consumes api
        api.register(engine.lua())
            .map_err(|e| HobbsError::Script(format!("Failed to register BBS API: {}", e)))?;

        // Register data API
        self.register_data_api(
            engine.lua(),
            script.id,
            context.user_id,
            &global_data,
            &user_data,
        )
        .map_err(|e| HobbsError::Script(format!("Failed to register data API: {}", e)))?;

        // Start timing
        let start_time = Instant::now();

        // Execute the script
        let result = engine.execute(&source);

        // Calculate execution time
        let execution_ms = start_time.elapsed().as_millis() as i64;

        // Save data changes back to database
        self.save_global_data(&data_repo, script.id, &global_data.borrow())?;
        if let Some(user_id) = context.user_id {
            self.save_user_data(&data_repo, script.id, user_id, &user_data.borrow())?;
        }

        // Get output regardless of success/failure
        let output = output_buffer.borrow().clone();

        // Build execution result
        let exec_result = match &result {
            Ok(()) => ExecutionResult {
                output,
                instructions_used: engine.instruction_count(),
                success: true,
                error: None,
            },
            Err(e) => ExecutionResult {
                output,
                instructions_used: engine.instruction_count(),
                success: false,
                error: Some(e.to_string()),
            },
        };

        // Log the execution
        let log_repo = ScriptLogRepository::new(self.db);
        let _ = log_repo.log_execution(
            script.id,
            context.user_id,
            execution_ms,
            exec_result.success,
            exec_result.error.as_deref(),
        );

        Ok(exec_result)
    }

    /// Load global data for a script.
    fn load_global_data(
        &self,
        repo: &ScriptDataRepository,
        script_id: i64,
    ) -> Result<HashMap<String, String>> {
        let keys = repo.list_global_keys(script_id)?;
        let mut data = HashMap::new();
        for key in keys {
            if let Some(value) = repo.get_global(script_id, &key)? {
                data.insert(key, value);
            }
        }
        Ok(data)
    }

    /// Load user-specific data for a script.
    fn load_user_data(
        &self,
        repo: &ScriptDataRepository,
        script_id: i64,
        user_id: Option<i64>,
    ) -> Result<HashMap<String, String>> {
        let Some(user_id) = user_id else {
            return Ok(HashMap::new());
        };
        let keys = repo.list_user_keys(script_id, user_id)?;
        let mut data = HashMap::new();
        for key in keys {
            if let Some(value) = repo.get_user(script_id, user_id, &key)? {
                data.insert(key, value);
            }
        }
        Ok(data)
    }

    /// Save global data for a script.
    fn save_global_data(
        &self,
        repo: &ScriptDataRepository,
        script_id: i64,
        data: &HashMap<String, String>,
    ) -> Result<()> {
        for (key, value) in data {
            repo.set_global(script_id, key, value)?;
        }
        Ok(())
    }

    /// Save user-specific data for a script.
    fn save_user_data(
        &self,
        repo: &ScriptDataRepository,
        script_id: i64,
        user_id: i64,
        data: &HashMap<String, String>,
    ) -> Result<()> {
        for (key, value) in data {
            repo.set_user(script_id, user_id, key, value)?;
        }
        Ok(())
    }

    /// Register data API functions in Lua.
    fn register_data_api(
        &self,
        lua: &mlua::Lua,
        script_id: i64,
        user_id: Option<i64>,
        global_data: &Rc<RefCell<HashMap<String, String>>>,
        user_data: &Rc<RefCell<HashMap<String, String>>>,
    ) -> mlua::Result<()> {
        let bbs: Table = lua.globals().get("bbs")?;

        // bbs.data table for global data
        let data_table = lua.create_table()?;
        self.register_global_data_functions(lua, &data_table, global_data)?;
        bbs.set("data", data_table)?;

        // bbs.user_data table for user-specific data
        let user_data_table = lua.create_table()?;
        self.register_user_data_functions(lua, &user_data_table, user_id, user_data)?;
        bbs.set("user_data", user_data_table)?;

        Ok(())
    }

    /// Register global data functions (bbs.data.get/set/delete).
    fn register_global_data_functions(
        &self,
        lua: &mlua::Lua,
        table: &Table,
        data: &Rc<RefCell<HashMap<String, String>>>,
    ) -> mlua::Result<()> {
        // bbs.data.get(key) -> value or nil
        let data_get = Rc::clone(data);
        let get_fn = lua.create_function(move |_, key: String| {
            let data = data_get.borrow();
            match data.get(&key) {
                Some(v) => Ok(Some(v.clone())),
                None => Ok(None),
            }
        })?;
        table.set("get", get_fn)?;

        // bbs.data.set(key, value)
        let data_set = Rc::clone(data);
        let set_fn = lua.create_function(move |_, (key, value): (String, String)| {
            data_set.borrow_mut().insert(key, value);
            Ok(())
        })?;
        table.set("set", set_fn)?;

        // bbs.data.delete(key) -> bool
        let data_delete = Rc::clone(data);
        let delete_fn = lua.create_function(move |_, key: String| {
            Ok(data_delete.borrow_mut().remove(&key).is_some())
        })?;
        table.set("delete", delete_fn)?;

        Ok(())
    }

    /// Register user data functions (bbs.user_data.get/set/delete).
    fn register_user_data_functions(
        &self,
        lua: &mlua::Lua,
        table: &Table,
        user_id: Option<i64>,
        data: &Rc<RefCell<HashMap<String, String>>>,
    ) -> mlua::Result<()> {
        // If no user (guest), all operations return nil/false
        if user_id.is_none() {
            let get_fn = lua.create_function(|_, _key: String| Ok(None::<String>))?;
            table.set("get", get_fn)?;

            let set_fn = lua.create_function(|_, (_key, _value): (String, String)| Ok(()))?;
            table.set("set", set_fn)?;

            let delete_fn = lua.create_function(|_, _key: String| Ok(false))?;
            table.set("delete", delete_fn)?;

            return Ok(());
        }

        // bbs.user_data.get(key) -> value or nil
        let data_get = Rc::clone(data);
        let get_fn = lua.create_function(move |_, key: String| {
            let data = data_get.borrow();
            match data.get(&key) {
                Some(v) => Ok(Some(v.clone())),
                None => Ok(None),
            }
        })?;
        table.set("get", get_fn)?;

        // bbs.user_data.set(key, value)
        let data_set = Rc::clone(data);
        let set_fn = lua.create_function(move |_, (key, value): (String, String)| {
            data_set.borrow_mut().insert(key, value);
            Ok(())
        })?;
        table.set("set", set_fn)?;

        // bbs.user_data.delete(key) -> bool
        let data_delete = Rc::clone(data);
        let delete_fn = lua.create_function(move |_, key: String| {
            Ok(data_delete.borrow_mut().remove(&key).is_some())
        })?;
        table.set("delete", delete_fn)?;

        Ok(())
    }

    /// Load script source code from the file system.
    fn load_script_source(&self, file_path: &str) -> Result<String> {
        let scripts_dir = self
            .scripts_dir
            .as_ref()
            .ok_or_else(|| HobbsError::Script("Scripts directory not configured".to_string()))?;

        let loader = ScriptLoader::new(scripts_dir);
        loader.read_script_source(file_path)
    }

    /// Sync scripts from the file system to the database.
    pub fn sync_scripts(&self) -> Result<super::types::SyncResult> {
        let scripts_dir = self
            .scripts_dir
            .as_ref()
            .ok_or_else(|| HobbsError::Script("Scripts directory not configured".to_string()))?;

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
            script_id: None, // Set by execute()
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Insufficient role"));
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

    #[test]
    fn test_execute_script_with_global_data() {
        let db = create_test_db();
        let dir = tempdir().unwrap();

        // Create a script that uses global data
        let script_content = r#"-- @name Data Test
-- @min_role 0

-- Get or initialize counter
local count = bbs.data.get("counter") or "0"
count = tonumber(count) + 1
bbs.data.set("counter", tostring(count))
bbs.println("Counter: " .. count)
"#;
        fs::write(dir.path().join("data_test.lua"), script_content).unwrap();

        let loader = ScriptLoader::new(dir.path());
        loader.sync(&db).unwrap();

        let repo = ScriptRepository::new(&db);
        let script = repo.get_by_slug("data_test").unwrap().unwrap();

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());
        let context = ScriptContext::default();

        // First execution - counter should be 1
        let result = service.execute(&script, context.clone()).unwrap();
        assert!(result.success);

        // Second execution - counter should be 2
        let result = service.execute(&script, context.clone()).unwrap();
        assert!(result.success);

        // Verify data was persisted
        let data_repo = ScriptDataRepository::new(&db);
        let counter = data_repo.get_global(script.id, "counter").unwrap();
        assert_eq!(counter, Some("2".to_string()));
    }

    #[test]
    fn test_execute_script_with_user_data() {
        let db = create_test_db();
        let dir = tempdir().unwrap();

        // Create a user
        db.conn()
            .execute(
                "INSERT INTO users (username, password, nickname, role) VALUES ('testuser', 'hash', 'Test', 'member')",
                [],
            )
            .unwrap();
        let user_id = db.conn().last_insert_rowid();

        // Create a script that uses user data
        let script_content = r#"-- @name User Data Test
-- @min_role 0

if not bbs.is_guest() then
    local wins = bbs.user_data.get("wins") or "0"
    wins = tonumber(wins) + 1
    bbs.user_data.set("wins", tostring(wins))
    bbs.println("Wins: " .. wins)
end
"#;
        fs::write(dir.path().join("user_data_test.lua"), script_content).unwrap();

        let loader = ScriptLoader::new(dir.path());
        loader.sync(&db).unwrap();

        let repo = ScriptRepository::new(&db);
        let script = repo.get_by_slug("user_data_test").unwrap().unwrap();

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        // Execute as logged-in user
        let context = ScriptContext {
            script_id: None,
            user_id: Some(user_id),
            username: "testuser".to_string(),
            nickname: "Test".to_string(),
            user_role: 1,
            ..Default::default()
        };

        // First execution
        let result = service.execute(&script, context.clone()).unwrap();
        assert!(result.success);

        // Second execution
        let result = service.execute(&script, context).unwrap();
        assert!(result.success);

        // Verify user data was persisted
        let data_repo = ScriptDataRepository::new(&db);
        let wins = data_repo.get_user(script.id, user_id, "wins").unwrap();
        assert_eq!(wins, Some("2".to_string()));
    }

    #[test]
    fn test_execute_script_guest_user_data() {
        let db = create_test_db();
        let dir = tempdir().unwrap();

        // Create a script that tries to use user data as guest
        let script_content = r#"-- @name Guest Data Test
-- @min_role 0

bbs.user_data.set("test", "value")
local val = bbs.user_data.get("test")
if val == nil then
    bbs.println("Guest data not saved (expected)")
else
    bbs.println("Guest data saved: " .. val)
end
"#;
        fs::write(dir.path().join("guest_data_test.lua"), script_content).unwrap();

        let loader = ScriptLoader::new(dir.path());
        loader.sync(&db).unwrap();

        let repo = ScriptRepository::new(&db);
        let script = repo.get_by_slug("guest_data_test").unwrap().unwrap();

        let service = ScriptService::new(&db).with_scripts_dir(dir.path());

        // Execute as guest
        let context = ScriptContext::default();
        let result = service.execute(&script, context).unwrap();
        assert!(result.success);
        // For guest, user_data operations should be no-ops
    }
}
