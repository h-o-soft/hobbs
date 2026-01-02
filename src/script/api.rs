//! BBS API for Lua scripts.
//!
//! Provides the `bbs` global table with functions for script interaction.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use mlua::{Lua, Result as LuaResult, Table, Value};
use rand::Rng;

use super::engine::ScriptContext;
use super::input_bridge::ScriptInputHandle;

/// Maximum sleep duration in seconds.
const MAX_SLEEP_SECONDS: f64 = 5.0;

/// Output callback type for print/println.
pub type OutputCallback = Box<dyn Fn(&str) + 'static>;

/// Input callback type for input functions.
pub type InputCallback = Box<dyn Fn(&str) -> Option<String> + 'static>;

/// BBS API builder for registering functions with Lua.
pub struct BbsApi {
    context: ScriptContext,
    output_buffer: Rc<RefCell<Vec<String>>>,
    input_handler: Rc<RefCell<Option<InputCallback>>>,
    input_bridge: Option<Arc<ScriptInputHandle>>,
}

impl BbsApi {
    /// Create a new BbsApi with the given context.
    pub fn new(context: ScriptContext) -> Self {
        Self {
            context,
            output_buffer: Rc::new(RefCell::new(Vec::new())),
            input_handler: Rc::new(RefCell::new(None)),
            input_bridge: None,
        }
    }

    /// Set the input handler callback.
    pub fn with_input_handler<F>(self, handler: F) -> Self
    where
        F: Fn(&str) -> Option<String> + 'static,
    {
        *self.input_handler.borrow_mut() = Some(Box::new(handler));
        self
    }

    /// Set the input bridge for async input handling.
    pub fn with_input_bridge(mut self, bridge: Arc<ScriptInputHandle>) -> Self {
        self.input_bridge = Some(bridge);
        self
    }

    /// Get the output buffer contents.
    pub fn get_output(&self) -> Vec<String> {
        self.output_buffer.borrow().clone()
    }

    /// Get a shared reference to the output buffer.
    /// Use this before calling register() to retain access to output after registration.
    pub fn output_buffer_ref(&self) -> Rc<RefCell<Vec<String>>> {
        Rc::clone(&self.output_buffer)
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

        // === Input functions ===
        self.register_input_functions(lua, &bbs)?;

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

    /// Register input functions.
    fn register_input_functions(&self, lua: &Lua, bbs: &Table) -> LuaResult<()> {
        let output = Rc::clone(&self.output_buffer);
        let input_handler = Rc::clone(&self.input_handler);
        let input_bridge = self.input_bridge.clone();

        // Helper to get input - tries bridge first, then callback
        fn get_input(
            bridge: &Option<Arc<ScriptInputHandle>>,
            handler: &Rc<RefCell<Option<InputCallback>>>,
            prompt: Option<String>,
        ) -> Result<Option<String>, mlua::Error> {
            // Try bridge first
            if let Some(ref b) = bridge {
                return Ok(b.request_input(prompt));
            }

            // Try callback handler
            let handler = handler.borrow();
            if let Some(ref h) = *handler {
                return Ok(h(""));
            }

            // No input available
            Err(mlua::Error::RuntimeError(
                "Input not available: interactive input is not supported in this context"
                    .to_string(),
            ))
        }

        // bbs.input(prompt) - get user input
        let input_output = Rc::clone(&output);
        let input_handler_clone = Rc::clone(&input_handler);
        let input_bridge_clone = input_bridge.clone();
        let input_fn = lua.create_function(move |_, prompt: Option<String>| {
            // Output prompt if provided (only if not using bridge, since bridge outputs it)
            if input_bridge_clone.is_none() {
                if let Some(p) = &prompt {
                    input_output.borrow_mut().push(p.clone());
                }
            }

            get_input(&input_bridge_clone, &input_handler_clone, prompt)
        })?;
        bbs.set("input", input_fn)?;

        // bbs.input_number(prompt) - get numeric input
        let input_number_output = Rc::clone(&output);
        let input_number_handler = Rc::clone(&input_handler);
        let input_number_bridge = input_bridge.clone();
        let input_number_fn = lua.create_function(move |_, prompt: Option<String>| {
            // Output prompt if provided (only if not using bridge)
            if input_number_bridge.is_none() {
                if let Some(p) = &prompt {
                    input_number_output.borrow_mut().push(p.clone());
                }
            }

            match get_input(&input_number_bridge, &input_number_handler, prompt) {
                Ok(Some(input)) => {
                    // Try to parse as number
                    if let Ok(n) = input.trim().parse::<f64>() {
                        Ok(Some(n))
                    } else {
                        Ok(None)
                    }
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        })?;
        bbs.set("input_number", input_number_fn)?;

        // bbs.input_yn(prompt) - get Y/N input (returns true for Y, false for N, nil otherwise)
        let input_yn_output = Rc::clone(&output);
        let input_yn_handler = Rc::clone(&input_handler);
        let input_yn_bridge = input_bridge.clone();
        let input_yn_fn = lua.create_function(move |_, prompt: Option<String>| {
            // Output prompt if provided (only if not using bridge)
            if input_yn_bridge.is_none() {
                if let Some(p) = &prompt {
                    input_yn_output.borrow_mut().push(p.clone());
                }
            }

            match get_input(&input_yn_bridge, &input_yn_handler, prompt) {
                Ok(Some(input)) => {
                    let input = input.trim().to_ascii_lowercase();
                    if input == "y" || input == "yes" {
                        Ok(Some(true))
                    } else if input == "n" || input == "no" {
                        Ok(Some(false))
                    } else {
                        Ok(None)
                    }
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        })?;
        bbs.set("input_yn", input_yn_fn)?;

        // bbs.pause() - wait for user to press Enter
        let pause_output = Rc::clone(&output);
        let pause_handler = Rc::clone(&input_handler);
        let pause_bridge = input_bridge.clone();
        let pause_fn = lua.create_function(move |_, ()| {
            // Output default prompt (only if not using bridge)
            if pause_bridge.is_none() {
                pause_output
                    .borrow_mut()
                    .push("Press Enter to continue...".to_string());
            }

            // Wait for input - pause doesn't need to return anything
            if let Some(ref b) = pause_bridge {
                let _ = b.request_input(Some("Press Enter to continue...".to_string()));
            } else {
                let handler = pause_handler.borrow();
                if let Some(ref h) = *handler {
                    let _ = h("");
                }
            }
            Ok(())
        })?;
        bbs.set("pause", pause_fn)?;

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
        let output = Rc::clone(&self.output_buffer);
        let has_ansi = self.context.has_ansi;

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

        // bbs.sleep(seconds) - wait for specified seconds (max 5)
        let sleep_fn = lua.create_function(|_, seconds: f64| {
            if seconds < 0.0 {
                return Err(mlua::Error::RuntimeError(
                    "sleep: seconds must be non-negative".to_string(),
                ));
            }
            let clamped = seconds.min(MAX_SLEEP_SECONDS);
            std::thread::sleep(Duration::from_secs_f64(clamped));
            Ok(())
        })?;
        bbs.set("sleep", sleep_fn)?;

        // bbs.clear() - clear screen (ANSI terminals only)
        let clear_output = Rc::clone(&output);
        let clear_fn = lua.create_function(move |_, ()| {
            if has_ansi {
                // ANSI escape: clear screen and move cursor to home
                clear_output.borrow_mut().push("\x1b[2J\x1b[H".to_string());
            }
            Ok(())
        })?;
        bbs.set("clear", clear_fn)?;

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
            script_id: Some(1),
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
        assert_eq!(engine.get_global::<String>("username").unwrap(), "testuser");
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

    #[test]
    fn test_bbs_sleep() {
        let (engine, _) = create_test_engine_with_api();

        let start = std::time::Instant::now();
        engine.execute("bbs.sleep(0.1)").unwrap();
        let elapsed = start.elapsed();

        // Should sleep at least 100ms
        assert!(elapsed.as_millis() >= 100);
        // But not too long
        assert!(elapsed.as_millis() < 200);
    }

    #[test]
    fn test_bbs_sleep_clamped() {
        let (engine, _) = create_test_engine_with_api();

        let start = std::time::Instant::now();
        // Request 10 seconds, should be clamped to 5
        engine.execute("bbs.sleep(10)").unwrap();
        let elapsed = start.elapsed();

        // Should be around 5 seconds (clamped)
        assert!(elapsed.as_secs() >= 5);
        assert!(elapsed.as_secs() < 6);
    }

    #[test]
    fn test_bbs_sleep_negative_error() {
        let (engine, _) = create_test_engine_with_api();

        let result = engine.execute("bbs.sleep(-1)");
        assert!(result.is_err());
    }

    #[test]
    fn test_bbs_clear_ansi() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute("bbs.clear()").unwrap();

        let out = output.borrow();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "\x1b[2J\x1b[H");
    }

    #[test]
    fn test_bbs_clear_no_ansi() {
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext {
            script_id: Some(1),
            has_ansi: false,
            ..Default::default()
        };
        let api = BbsApi::new(context);
        let output = Rc::clone(&api.output_buffer);
        api.register(engine.lua()).unwrap();

        engine.execute("bbs.clear()").unwrap();

        let out = output.borrow();
        // No output when ANSI is disabled
        assert!(out.is_empty());
    }

    fn create_test_engine_with_input(
        input_responses: Vec<String>,
    ) -> (ScriptEngine, Rc<RefCell<Vec<String>>>) {
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext {
            script_id: Some(1),
            user_id: Some(42),
            username: "testuser".to_string(),
            nickname: "Test User".to_string(),
            user_role: 1,
            terminal_width: 80,
            terminal_height: 24,
            has_ansi: true,
        };

        let responses = Rc::new(RefCell::new(input_responses));
        let response_index = Rc::new(RefCell::new(0usize));

        let responses_clone = Rc::clone(&responses);
        let index_clone = Rc::clone(&response_index);

        let api = BbsApi::new(context).with_input_handler(move |_| {
            let mut idx = index_clone.borrow_mut();
            let resps = responses_clone.borrow();
            if *idx < resps.len() {
                let result = resps[*idx].clone();
                *idx += 1;
                Some(result)
            } else {
                None
            }
        });

        let output = Rc::clone(&api.output_buffer);
        api.register(engine.lua()).unwrap();
        (engine, output)
    }

    #[test]
    fn test_bbs_input() {
        let (engine, output) = create_test_engine_with_input(vec!["hello".to_string()]);

        engine.execute(r#"result = bbs.input("Enter: ")"#).unwrap();

        let result: String = engine.get_global("result").unwrap();
        assert_eq!(result, "hello");

        let out = output.borrow();
        assert_eq!(out[0], "Enter: ");
    }

    #[test]
    fn test_bbs_input_no_handler() {
        let (engine, _) = create_test_engine_with_api();

        // Should error when no input handler is available
        let result = engine.execute("result = bbs.input()");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Input not available"));
    }

    #[test]
    fn test_bbs_input_number() {
        let (engine, _) = create_test_engine_with_input(vec!["42".to_string()]);

        engine.execute("result = bbs.input_number()").unwrap();

        let result: f64 = engine.get_global("result").unwrap();
        assert!((result - 42.0).abs() < 0.001);
    }

    #[test]
    fn test_bbs_input_number_float() {
        let (engine, _) = create_test_engine_with_input(vec!["3.14".to_string()]);

        engine.execute("result = bbs.input_number()").unwrap();

        let result: f64 = engine.get_global("result").unwrap();
        assert!((result - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_bbs_input_number_invalid() {
        let (engine, _) = create_test_engine_with_input(vec!["not a number".to_string()]);

        engine.execute("result = bbs.input_number()").unwrap();

        // Should return nil for invalid input
        engine.execute("is_nil = result == nil").unwrap();
        assert!(engine.get_global::<bool>("is_nil").unwrap());
    }

    #[test]
    fn test_bbs_input_yn_yes() {
        let (engine, _) = create_test_engine_with_input(vec!["y".to_string()]);

        engine.execute("result = bbs.input_yn()").unwrap();

        assert!(engine.get_global::<bool>("result").unwrap());
    }

    #[test]
    fn test_bbs_input_yn_yes_full() {
        let (engine, _) = create_test_engine_with_input(vec!["YES".to_string()]);

        engine.execute("result = bbs.input_yn()").unwrap();

        assert!(engine.get_global::<bool>("result").unwrap());
    }

    #[test]
    fn test_bbs_input_yn_no() {
        let (engine, _) = create_test_engine_with_input(vec!["n".to_string()]);

        engine.execute("result = bbs.input_yn()").unwrap();

        assert!(!engine.get_global::<bool>("result").unwrap());
    }

    #[test]
    fn test_bbs_input_yn_invalid() {
        let (engine, _) = create_test_engine_with_input(vec!["maybe".to_string()]);

        engine.execute("result = bbs.input_yn()").unwrap();

        // Should return nil for invalid input
        engine.execute("is_nil = result == nil").unwrap();
        assert!(engine.get_global::<bool>("is_nil").unwrap());
    }

    #[test]
    fn test_bbs_pause() {
        let (engine, output) = create_test_engine_with_input(vec!["".to_string()]);

        engine.execute("bbs.pause()").unwrap();

        let out = output.borrow();
        assert_eq!(out[0], "Press Enter to continue...");
    }
}
