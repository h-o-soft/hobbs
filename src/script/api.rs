//! BBS API for Lua scripts.
//!
//! Provides the `bbs` global table with functions for script interaction.

use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Lua, Result as LuaResult, Table, Value};
use rand::Rng;

use super::engine::ScriptContext;

/// Output callback type for print/println.
pub type OutputCallback = Box<dyn Fn(&str) + 'static>;

/// Input callback type for input functions.
pub type InputCallback = Box<dyn Fn(&str) -> Option<String> + 'static>;

/// BBS API builder for registering functions with Lua.
pub struct BbsApi {
    context: ScriptContext,
    output_buffer: Rc<RefCell<Vec<String>>>,
    input_handler: Option<InputCallback>,
}

impl BbsApi {
    /// Create a new BbsApi with the given context.
    pub fn new(context: ScriptContext) -> Self {
        Self {
            context,
            output_buffer: Rc::new(RefCell::new(Vec::new())),
            input_handler: None,
        }
    }

    /// Set the input handler callback.
    pub fn with_input_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&str) -> Option<String> + 'static,
    {
        self.input_handler = Some(Box::new(handler));
        self
    }

    /// Get the output buffer contents.
    pub fn get_output(&self) -> Vec<String> {
        self.output_buffer.borrow().clone()
    }

    /// Clear the output buffer.
    pub fn clear_output(&self) {
        self.output_buffer.borrow_mut().clear();
    }

    /// Register the BBS API with the Lua environment.
    pub fn register(self, lua: &Lua) -> LuaResult<()> {
        let bbs = lua.create_table()?;

        // === Output functions ===
        self.register_print_functions(lua, &bbs)?;

        // === User functions ===
        self.register_user_functions(lua, &bbs)?;

        // === Utility functions ===
        self.register_utility_functions(lua, &bbs)?;

        // === Terminal table ===
        self.register_terminal_table(lua, &bbs)?;

        // Set bbs as global
        lua.globals().set("bbs", bbs)?;

        Ok(())
    }

    /// Register print/println functions.
    fn register_print_functions(&self, lua: &Lua, bbs: &Table) -> LuaResult<()> {
        let output = Rc::clone(&self.output_buffer);

        // bbs.print(text) - output without newline
        let print_output = Rc::clone(&output);
        let print_fn = lua.create_function(move |_, text: Value| {
            let text_str = value_to_string(&text);
            print_output.borrow_mut().push(text_str);
            Ok(())
        })?;
        bbs.set("print", print_fn)?;

        // bbs.println(text) - output with newline
        let println_output = Rc::clone(&output);
        let println_fn = lua.create_function(move |_, text: Value| {
            let text_str = value_to_string(&text);
            println_output.borrow_mut().push(format!("{}\n", text_str));
            Ok(())
        })?;
        bbs.set("println", println_fn)?;

        Ok(())
    }

    /// Register user-related functions.
    fn register_user_functions(&self, lua: &Lua, bbs: &Table) -> LuaResult<()> {
        // bbs.get_user() - returns user info table
        let user_id = self.context.user_id;
        let username = self.context.username.clone();
        let nickname = self.context.nickname.clone();
        let user_role = self.context.user_role;

        let get_user_fn = lua.create_function(move |lua, ()| {
            let user_table = lua.create_table()?;
            match user_id {
                Some(id) => user_table.set("id", id)?,
                None => user_table.set("id", Value::Nil)?,
            }
            user_table.set("username", username.clone())?;
            user_table.set("nickname", nickname.clone())?;
            user_table.set("role", user_role)?;
            Ok(user_table)
        })?;
        bbs.set("get_user", get_user_fn)?;

        // bbs.is_guest() - check if user is guest
        let is_guest = self.context.user_id.is_none();
        let is_guest_fn = lua.create_function(move |_, ()| Ok(is_guest))?;
        bbs.set("is_guest", is_guest_fn)?;

        // bbs.is_sysop() - check if user is SysOp (role >= 3)
        let is_sysop = self.context.user_role >= 3;
        let is_sysop_fn = lua.create_function(move |_, ()| Ok(is_sysop))?;
        bbs.set("is_sysop", is_sysop_fn)?;

        Ok(())
    }

    /// Register utility functions.
    fn register_utility_functions(&self, lua: &Lua, bbs: &Table) -> LuaResult<()> {
        // bbs.random(min, max) - generate random number
        let random_fn = lua.create_function(|_, (min, max): (i64, i64)| {
            if min > max {
                return Err(mlua::Error::RuntimeError(
                    "random: min must be less than or equal to max".to_string(),
                ));
            }
            let mut rng = rand::rng();
            Ok(rng.random_range(min..=max))
        })?;
        bbs.set("random", random_fn)?;

        // bbs.get_time() - get current time as HH:MM:SS
        let get_time_fn = lua.create_function(|_, ()| {
            let now = chrono::Local::now();
            Ok(now.format("%H:%M:%S").to_string())
        })?;
        bbs.set("get_time", get_time_fn)?;

        // bbs.get_date() - get current date as YYYY-MM-DD
        let get_date_fn = lua.create_function(|_, ()| {
            let now = chrono::Local::now();
            Ok(now.format("%Y-%m-%d").to_string())
        })?;
        bbs.set("get_date", get_date_fn)?;

        Ok(())
    }

    /// Register terminal info table.
    fn register_terminal_table(&self, lua: &Lua, bbs: &Table) -> LuaResult<()> {
        let terminal = lua.create_table()?;

        terminal.set("width", self.context.terminal_width)?;
        terminal.set("height", self.context.terminal_height)?;
        terminal.set("has_ansi", self.context.has_ansi)?;

        bbs.set("terminal", terminal)?;

        Ok(())
    }
}

/// Convert a Lua Value to a string for output.
fn value_to_string(value: &Value) -> String {
    match value {
        Value::Nil => "nil".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_default(),
        Value::Table(_) => "[table]".to_string(),
        Value::Function(_) => "[function]".to_string(),
        Value::Thread(_) => "[thread]".to_string(),
        Value::UserData(_) => "[userdata]".to_string(),
        Value::LightUserData(_) => "[lightuserdata]".to_string(),
        Value::Error(e) => format!("[error: {}]", e),
        _ => "[unknown]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptEngine;

    fn create_test_engine_with_api() -> (ScriptEngine, Rc<RefCell<Vec<String>>>) {
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext {
            user_id: Some(42),
            username: "testuser".to_string(),
            nickname: "Test User".to_string(),
            user_role: 1,
            terminal_width: 80,
            terminal_height: 24,
            has_ansi: true,
        };
        let api = BbsApi::new(context);
        let output = Rc::clone(&api.output_buffer);
        api.register(engine.lua()).unwrap();
        (engine, output)
    }

    #[test]
    fn test_bbs_print() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute(r#"bbs.print("Hello")"#).unwrap();

        let out = output.borrow();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "Hello");
    }

    #[test]
    fn test_bbs_println() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute(r#"bbs.println("Hello")"#).unwrap();

        let out = output.borrow();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "Hello\n");
    }

    #[test]
    fn test_bbs_print_multiple() {
        let (engine, output) = create_test_engine_with_api();

        engine
            .execute(
                r#"
                bbs.print("A")
                bbs.print("B")
                bbs.println("C")
            "#,
            )
            .unwrap();

        let out = output.borrow();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], "A");
        assert_eq!(out[1], "B");
        assert_eq!(out[2], "C\n");
    }

    #[test]
    fn test_bbs_print_number() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute(r#"bbs.println(42)"#).unwrap();

        let out = output.borrow();
        assert_eq!(out[0], "42\n");
    }

    #[test]
    fn test_bbs_print_nil() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute(r#"bbs.println(nil)"#).unwrap();

        let out = output.borrow();
        assert_eq!(out[0], "nil\n");
    }

    #[test]
    fn test_bbs_get_user() {
        let (engine, _) = create_test_engine_with_api();

        engine
            .execute(
                r#"
                local user = bbs.get_user()
                user_id = user.id
                username = user.username
                nickname = user.nickname
                role = user.role
            "#,
            )
            .unwrap();

        assert_eq!(engine.get_global::<i64>("user_id").unwrap(), 42);
        assert_eq!(
            engine.get_global::<String>("username").unwrap(),
            "testuser"
        );
        assert_eq!(
            engine.get_global::<String>("nickname").unwrap(),
            "Test User"
        );
        assert_eq!(engine.get_global::<i32>("role").unwrap(), 1);
    }

    #[test]
    fn test_bbs_get_user_guest() {
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext::default(); // guest
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        engine
            .execute(
                r#"
                local user = bbs.get_user()
                is_nil = user.id == nil
            "#,
            )
            .unwrap();

        assert!(engine.get_global::<bool>("is_nil").unwrap());
    }

    #[test]
    fn test_bbs_is_guest() {
        // Test with guest
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext::default();
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        engine.execute("result = bbs.is_guest()").unwrap();
        assert!(engine.get_global::<bool>("result").unwrap());

        // Test with member
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext {
            user_id: Some(1),
            ..Default::default()
        };
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        engine.execute("result = bbs.is_guest()").unwrap();
        assert!(!engine.get_global::<bool>("result").unwrap());
    }

    #[test]
    fn test_bbs_is_sysop() {
        // Test with member (role 1)
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext {
            user_role: 1,
            ..Default::default()
        };
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        engine.execute("result = bbs.is_sysop()").unwrap();
        assert!(!engine.get_global::<bool>("result").unwrap());

        // Test with SysOp (role 3)
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext {
            user_role: 3,
            ..Default::default()
        };
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        engine.execute("result = bbs.is_sysop()").unwrap();
        assert!(engine.get_global::<bool>("result").unwrap());
    }

    #[test]
    fn test_bbs_random() {
        let (engine, _) = create_test_engine_with_api();

        // Test that random is within bounds
        engine
            .execute(
                r#"
                result = bbs.random(1, 10)
            "#,
            )
            .unwrap();

        let result: i64 = engine.get_global("result").unwrap();
        assert!(result >= 1 && result <= 10);
    }

    #[test]
    fn test_bbs_random_same_value() {
        let (engine, _) = create_test_engine_with_api();

        engine.execute("result = bbs.random(5, 5)").unwrap();

        let result: i64 = engine.get_global("result").unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_bbs_random_invalid_range() {
        let (engine, _) = create_test_engine_with_api();

        let result = engine.execute("result = bbs.random(10, 1)");
        assert!(result.is_err());
    }

    #[test]
    fn test_bbs_get_time() {
        let (engine, _) = create_test_engine_with_api();

        engine.execute("result = bbs.get_time()").unwrap();

        let result: String = engine.get_global("result").unwrap();
        // Should be in HH:MM:SS format
        assert_eq!(result.len(), 8);
        assert!(result.chars().nth(2).unwrap() == ':');
        assert!(result.chars().nth(5).unwrap() == ':');
    }

    #[test]
    fn test_bbs_get_date() {
        let (engine, _) = create_test_engine_with_api();

        engine.execute("result = bbs.get_date()").unwrap();

        let result: String = engine.get_global("result").unwrap();
        // Should be in YYYY-MM-DD format
        assert_eq!(result.len(), 10);
        assert!(result.chars().nth(4).unwrap() == '-');
        assert!(result.chars().nth(7).unwrap() == '-');
    }

    #[test]
    fn test_bbs_terminal() {
        let (engine, _) = create_test_engine_with_api();

        engine
            .execute(
                r#"
                width = bbs.terminal.width
                height = bbs.terminal.height
                has_ansi = bbs.terminal.has_ansi
            "#,
            )
            .unwrap();

        assert_eq!(engine.get_global::<u16>("width").unwrap(), 80);
        assert_eq!(engine.get_global::<u16>("height").unwrap(), 24);
        assert!(engine.get_global::<bool>("has_ansi").unwrap());
    }

    #[test]
    fn test_combined_script() {
        let (engine, output) = create_test_engine_with_api();

        engine
            .execute(
                r#"
                local user = bbs.get_user()
                bbs.println("=== Welcome ===")
                bbs.println("Hello, " .. user.nickname .. "!")
                bbs.println("Terminal: " .. bbs.terminal.width .. "x" .. bbs.terminal.height)
                bbs.println("Random: " .. bbs.random(1, 100))
            "#,
            )
            .unwrap();

        let out = output.borrow();
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], "=== Welcome ===\n");
        assert_eq!(out[1], "Hello, Test User!\n");
        assert_eq!(out[2], "Terminal: 80x24\n");
        assert!(out[3].starts_with("Random: "));
    }
}
