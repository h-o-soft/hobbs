//! Lua script engine with sandboxing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use mlua::{Function, HookTriggers, Lua, Result as LuaResult, Value, VmState};

use crate::{HobbsError, Result};

/// Resource limits for script execution.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum number of instructions (0 = unlimited).
    pub max_instructions: u64,
    /// Maximum memory in bytes (0 = unlimited).
    pub max_memory: usize,
    /// Maximum execution time in seconds (handled externally).
    pub max_execution_seconds: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_instructions: 1_000_000,
            max_memory: 10 * 1024 * 1024, // 10MB
            max_execution_seconds: 30,
        }
    }
}

/// Script execution context.
pub struct ScriptContext {
    /// User information.
    pub user_id: Option<i64>,
    pub username: String,
    pub nickname: String,
    pub user_role: i32,
    /// Terminal information.
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub has_ansi: bool,
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self {
            user_id: None,
            username: "guest".to_string(),
            nickname: "Guest".to_string(),
            user_role: 0,
            terminal_width: 80,
            terminal_height: 24,
            has_ansi: true,
        }
    }
}

/// Lua script execution engine with sandboxing.
pub struct ScriptEngine {
    lua: Lua,
    instruction_count: Arc<AtomicU64>,
    limits: ResourceLimits,
}

impl ScriptEngine {
    /// Create a new ScriptEngine with default resource limits.
    pub fn new() -> Result<Self> {
        Self::with_limits(ResourceLimits::default())
    }

    /// Create a new ScriptEngine with custom resource limits.
    pub fn with_limits(limits: ResourceLimits) -> Result<Self> {
        // Create Lua with safe standard libraries
        let lua = Lua::new();

        // Apply sandbox restrictions
        Self::apply_sandbox(&lua)?;

        // Set memory limit if specified
        if limits.max_memory > 0 {
            lua.set_memory_limit(limits.max_memory)
                .map_err(|e| HobbsError::Script(format!("Failed to set memory limit: {}", e)))?;
        }

        Ok(Self {
            lua,
            instruction_count: Arc::new(AtomicU64::new(0)),
            limits,
        })
    }

    /// Apply sandbox restrictions to the Lua environment.
    fn apply_sandbox(lua: &Lua) -> Result<()> {
        let globals = lua.globals();

        // Disable dangerous functions
        let nil = Value::Nil;
        globals
            .set("os", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable os: {}", e)))?;
        globals
            .set("io", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable io: {}", e)))?;
        globals
            .set("loadfile", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable loadfile: {}", e)))?;
        globals
            .set("dofile", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable dofile: {}", e)))?;
        globals
            .set("load", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable load: {}", e)))?;
        globals
            .set("require", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable require: {}", e)))?;
        globals
            .set("package", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable package: {}", e)))?;
        globals
            .set("debug", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable debug: {}", e)))?;
        globals
            .set("collectgarbage", nil.clone())
            .map_err(|e| HobbsError::Script(format!("Failed to disable collectgarbage: {}", e)))?;

        Ok(())
    }

    /// Execute Lua source code.
    pub fn execute(&self, source: &str) -> Result<()> {
        // Reset instruction count
        self.instruction_count.store(0, Ordering::SeqCst);

        // Set up instruction hook for limiting execution
        if self.limits.max_instructions > 0 {
            let count = Arc::clone(&self.instruction_count);
            let limit = self.limits.max_instructions;

            self.lua.set_hook(
                HookTriggers::new().every_nth_instruction(10000),
                move |_lua, _debug| {
                    let current = count.fetch_add(10000, Ordering::SeqCst) + 10000;
                    if current > limit {
                        Err(mlua::Error::RuntimeError(
                            "Script exceeded instruction limit".to_string(),
                        ))
                    } else {
                        Ok(VmState::Continue)
                    }
                },
            );
        }

        // Execute the script
        self.lua.load(source).exec().map_err(|e| {
            // Remove hook after execution
            let _ = self.lua.remove_hook();
            HobbsError::Script(format!("Script error: {}", e))
        })?;

        // Remove hook after successful execution
        let _ = self.lua.remove_hook();

        Ok(())
    }

    /// Set a global value in the Lua environment.
    pub fn set_global<V: mlua::IntoLua>(&self, name: &str, value: V) -> Result<()> {
        self.lua
            .globals()
            .set(name, value)
            .map_err(|e| HobbsError::Script(format!("Failed to set global '{}': {}", name, e)))
    }

    /// Get a global value from the Lua environment.
    pub fn get_global<V: mlua::FromLua>(&self, name: &str) -> Result<V> {
        self.lua
            .globals()
            .get(name)
            .map_err(|e| HobbsError::Script(format!("Failed to get global '{}': {}", name, e)))
    }

    /// Create a Lua function from a Rust closure.
    pub fn create_function<F, A, R>(&self, func: F) -> Result<Function>
    where
        F: Fn(&Lua, A) -> LuaResult<R> + 'static,
        A: mlua::FromLuaMulti,
        R: mlua::IntoLuaMulti,
    {
        self.lua
            .create_function(func)
            .map_err(|e| HobbsError::Script(format!("Failed to create function: {}", e)))
    }

    /// Get the instruction count.
    pub fn instruction_count(&self) -> u64 {
        self.instruction_count.load(Ordering::SeqCst)
    }

    /// Get the resource limits.
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }

    /// Get a reference to the underlying Lua instance.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default ScriptEngine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_execution() {
        let engine = ScriptEngine::new().unwrap();
        engine.execute("x = 1 + 2").unwrap();

        let result: i32 = engine.get_global("x").unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn test_string_operations() {
        let engine = ScriptEngine::new().unwrap();
        engine
            .execute(r#"result = string.upper("hello")"#)
            .unwrap();

        let result: String = engine.get_global("result").unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_math_operations() {
        let engine = ScriptEngine::new().unwrap();
        engine.execute("result = math.floor(3.7)").unwrap();

        let result: i32 = engine.get_global("result").unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn test_table_operations() {
        let engine = ScriptEngine::new().unwrap();
        engine
            .execute(
                r#"
                t = {1, 2, 3}
                table.insert(t, 4)
                result = #t
            "#,
            )
            .unwrap();

        let result: i32 = engine.get_global("result").unwrap();
        assert_eq!(result, 4);
    }

    #[test]
    fn test_sandbox_os_disabled() {
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute("os.execute('ls')");
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_io_disabled() {
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute("io.open('/etc/passwd', 'r')");
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_loadfile_disabled() {
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute("loadfile('/etc/passwd')");
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_require_disabled() {
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute("require('os')");
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_debug_disabled() {
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute("debug.traceback()");
        assert!(result.is_err());
    }

    #[test]
    fn test_instruction_limit() {
        let limits = ResourceLimits {
            max_instructions: 1000,
            max_memory: 0,
            max_execution_seconds: 30,
        };
        let engine = ScriptEngine::with_limits(limits).unwrap();

        // This infinite loop should be stopped by the instruction limit
        let result = engine.execute("while true do end");
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("instruction limit"));
    }

    #[test]
    fn test_memory_limit() {
        let limits = ResourceLimits {
            max_instructions: 0,
            max_memory: 1024 * 100, // 100KB
            max_execution_seconds: 30,
        };
        let engine = ScriptEngine::with_limits(limits).unwrap();

        // Try to allocate a large string
        let result = engine.execute(
            r#"
            t = {}
            for i = 1, 100000 do
                t[i] = string.rep("x", 1000)
            end
        "#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_set_and_get_global() {
        let engine = ScriptEngine::new().unwrap();

        engine.set_global("my_value", 42).unwrap();
        let result: i32 = engine.get_global("my_value").unwrap();
        assert_eq!(result, 42);

        engine.set_global("my_string", "hello").unwrap();
        let result: String = engine.get_global("my_string").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_create_function() {
        let engine = ScriptEngine::new().unwrap();

        // Create a Rust function and expose it to Lua
        let add = engine
            .create_function(|_, (a, b): (i32, i32)| Ok(a + b))
            .unwrap();

        engine.set_global("add", add).unwrap();
        engine.execute("result = add(3, 4)").unwrap();

        let result: i32 = engine.get_global("result").unwrap();
        assert_eq!(result, 7);
    }

    #[test]
    fn test_syntax_error() {
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute("this is not valid lua");
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_error() {
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute("error('test error')");
        assert!(result.is_err());
    }

    #[test]
    fn test_nil_access() {
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute("x = nil; y = x.field");
        assert!(result.is_err());
    }

    #[test]
    fn test_print_available() {
        // Note: print is available in base library but we might want to override it
        let engine = ScriptEngine::new().unwrap();
        let result = engine.execute(r#"print("Hello, World!")"#);
        // print should work (goes to stdout which we might want to capture)
        assert!(result.is_ok());
    }
}
