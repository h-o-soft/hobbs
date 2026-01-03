//! BBS API for Lua scripts.
//!
//! Provides the `bbs` global table with functions for script interaction.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mlua::{Lua, Result as LuaResult, Table, Value};
use rand::Rng;

use super::engine::ScriptContext;
use super::runtime::ScriptHandle;

/// Maximum sleep duration in seconds.
const MAX_SLEEP_SECONDS: f64 = 5.0;

/// BBS API builder for registering functions with Lua.
///
/// This API supports two modes:
/// 1. **Runtime mode**: Uses `ScriptHandle` for real-time I/O through message passing
/// 2. **Buffer mode**: Uses internal buffers for testing or non-interactive scripts
pub struct BbsApi {
    context: ScriptContext,
    /// Script handle for runtime mode (message passing)
    script_handle: Option<Arc<ScriptHandle>>,
    /// Output buffer for buffer mode (testing)
    output_buffer: Arc<Mutex<Vec<String>>>,
}

impl BbsApi {
    /// Create a new BbsApi with the given context.
    ///
    /// By default, uses buffer mode. Call `with_script_handle` to enable runtime mode.
    pub fn new(context: ScriptContext) -> Self {
        Self {
            context,
            script_handle: None,
            output_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Set the script handle for runtime mode (message passing).
    ///
    /// When a script handle is set, output is sent in real-time through the
    /// message channel, and input requests are handled asynchronously.
    pub fn with_script_handle(mut self, handle: Arc<ScriptHandle>) -> Self {
        self.script_handle = Some(handle);
        self
    }

    /// Get the output buffer contents (for buffer mode).
    pub fn get_output(&self) -> Vec<String> {
        self.output_buffer.lock().unwrap().clone()
    }

    /// Get a shared reference to the output buffer.
    pub fn output_buffer_ref(&self) -> Arc<Mutex<Vec<String>>> {
        Arc::clone(&self.output_buffer)
    }

    /// Clear the output buffer.
    pub fn clear_output(&self) {
        self.output_buffer.lock().unwrap().clear();
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

        // === i18n functions ===
        self.register_i18n_functions(lua, &bbs)?;

        // Set bbs as global
        lua.globals().set("bbs", bbs)?;

        Ok(())
    }

    /// Register print/println functions.
    fn register_print_functions(&self, lua: &Lua, bbs: &Table) -> LuaResult<()> {
        let handle = self.script_handle.clone();
        let buffer = Arc::clone(&self.output_buffer);

        // bbs.print(text) - output without newline
        let print_handle = handle.clone();
        let print_buffer = Arc::clone(&buffer);
        let print_fn = lua.create_function(move |_, text: Value| {
            let text_str = value_to_string(&text);
            if let Some(ref h) = print_handle {
                // Runtime mode: send through channel
                h.send_output(text_str);
            } else {
                // Buffer mode: store in buffer
                print_buffer.lock().unwrap().push(text_str);
            }
            Ok(())
        })?;
        bbs.set("print", print_fn)?;

        // bbs.println(text) - output with newline
        let println_handle = handle.clone();
        let println_buffer = Arc::clone(&buffer);
        let println_fn = lua.create_function(move |_, text: Value| {
            let text_str = format!("{}\n", value_to_string(&text));
            if let Some(ref h) = println_handle {
                // Runtime mode: send through channel
                h.send_output(text_str);
            } else {
                // Buffer mode: store in buffer
                println_buffer.lock().unwrap().push(text_str);
            }
            Ok(())
        })?;
        bbs.set("println", println_fn)?;

        Ok(())
    }

    /// Register input functions.
    fn register_input_functions(&self, lua: &Lua, bbs: &Table) -> LuaResult<()> {
        let handle = self.script_handle.clone();

        // bbs.input(prompt) - get user input
        let input_handle = handle.clone();
        let input_fn = lua.create_function(move |_, prompt: Option<String>| {
            if let Some(ref h) = input_handle {
                // Runtime mode: request input through channel
                Ok(h.request_input(prompt))
            } else {
                // Buffer mode: no input available
                Err(mlua::Error::RuntimeError(
                    "Input not available: interactive input is not supported in this context"
                        .to_string(),
                ))
            }
        })?;
        bbs.set("input", input_fn)?;

        // bbs.input_number(prompt) - get numeric input
        let input_number_handle = handle.clone();
        let input_number_fn = lua.create_function(move |_, prompt: Option<String>| {
            if let Some(ref h) = input_number_handle {
                // Runtime mode: request input through channel
                match h.request_input(prompt) {
                    Some(input) => {
                        if let Ok(n) = input.trim().parse::<f64>() {
                            Ok(Some(n))
                        } else {
                            Ok(None)
                        }
                    }
                    None => Ok(None),
                }
            } else {
                // Buffer mode: no input available
                Err(mlua::Error::RuntimeError(
                    "Input not available: interactive input is not supported in this context"
                        .to_string(),
                ))
            }
        })?;
        bbs.set("input_number", input_number_fn)?;

        // bbs.input_yn(prompt) - get Y/N input
        let input_yn_handle = handle.clone();
        let input_yn_fn = lua.create_function(move |_, prompt: Option<String>| {
            if let Some(ref h) = input_yn_handle {
                // Runtime mode: request input through channel
                match h.request_input(prompt) {
                    Some(input) => {
                        let input = input.trim().to_ascii_lowercase();
                        if input == "y" || input == "yes" {
                            Ok(Some(true))
                        } else if input == "n" || input == "no" {
                            Ok(Some(false))
                        } else {
                            Ok(None)
                        }
                    }
                    None => Ok(None),
                }
            } else {
                // Buffer mode: no input available
                Err(mlua::Error::RuntimeError(
                    "Input not available: interactive input is not supported in this context"
                        .to_string(),
                ))
            }
        })?;
        bbs.set("input_yn", input_yn_fn)?;

        // bbs.pause() - wait for user to press Enter
        let pause_handle = handle.clone();
        let pause_fn = lua.create_function(move |_, ()| {
            if let Some(ref h) = pause_handle {
                // Runtime mode: request input with default prompt
                let _ = h.request_input(Some("Press Enter to continue...".to_string()));
            }
            // Buffer mode: just continue (no-op)
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
        let handle = self.script_handle.clone();
        let buffer = Arc::clone(&self.output_buffer);
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
        let clear_handle = handle.clone();
        let clear_buffer = Arc::clone(&buffer);
        let clear_fn = lua.create_function(move |_, ()| {
            if has_ansi {
                let clear_seq = "\x1b[2J\x1b[H".to_string();
                if let Some(ref h) = clear_handle {
                    h.send_output(clear_seq);
                } else {
                    clear_buffer.lock().unwrap().push(clear_seq);
                }
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

    /// Register i18n (internationalization) functions.
    fn register_i18n_functions(&self, lua: &Lua, bbs: &Table) -> LuaResult<()> {
        let lang = self.context.lang.clone();
        let translations = self.context.translations.clone();

        // bbs.get_lang() - get current user language
        let get_lang_fn = lua.create_function(move |_, ()| Ok(lang.clone()))?;
        bbs.set("get_lang", get_lang_fn)?;

        // bbs.t(key) or bbs.t(key, default) - get translated text
        // Fallback order: translations[lang][key] -> translations["en"][key] -> default -> key
        let t_lang = self.context.lang.clone();
        let t_translations = translations.clone();
        let t_fn = lua.create_function(move |_, args: mlua::MultiValue| {
            let mut iter = args.into_iter();

            // First argument: key (required)
            let key = match iter.next() {
                Some(Value::String(s)) => s.to_str().map(|s| s.to_string()).unwrap_or_default(),
                _ => return Ok(String::new()),
            };

            // Second argument: default (optional)
            let default = match iter.next() {
                Some(Value::String(s)) => Some(s.to_str().map(|s| s.to_string()).unwrap_or_default()),
                _ => None,
            };

            // Try to find translation
            // 1. Try current language
            if let Some(lang_map) = t_translations.get(&t_lang) {
                if let Some(translated) = lang_map.get(&key) {
                    return Ok(translated.clone());
                }
            }

            // 2. Try English as fallback
            if t_lang != "en" {
                if let Some(lang_map) = t_translations.get("en") {
                    if let Some(translated) = lang_map.get(&key) {
                        return Ok(translated.clone());
                    }
                }
            }

            // 3. Return default or key
            Ok(default.unwrap_or(key))
        })?;
        bbs.set("t", t_fn)?;

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
    use crate::script::{create_script_runtime, ScriptEngine};
    use std::sync::Arc;
    use std::thread;

    fn create_test_engine_with_api() -> (ScriptEngine, Arc<Mutex<Vec<String>>>) {
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
            lang: "en".to_string(),
            translations: std::collections::HashMap::new(),
        };
        let api = BbsApi::new(context);
        let output = api.output_buffer_ref();
        api.register(engine.lua()).unwrap();
        (engine, output)
    }

    #[test]
    fn test_bbs_print() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute(r#"bbs.print("Hello")"#).unwrap();

        let out = output.lock().unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "Hello");
    }

    #[test]
    fn test_bbs_println() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute(r#"bbs.println("Hello")"#).unwrap();

        let out = output.lock().unwrap();
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

        let out = output.lock().unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], "A");
        assert_eq!(out[1], "B");
        assert_eq!(out[2], "C\n");
    }

    #[test]
    fn test_bbs_print_number() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute(r#"bbs.println(42)"#).unwrap();

        let out = output.lock().unwrap();
        assert_eq!(out[0], "42\n");
    }

    #[test]
    fn test_bbs_print_nil() {
        let (engine, output) = create_test_engine_with_api();

        engine.execute(r#"bbs.println(nil)"#).unwrap();

        let out = output.lock().unwrap();
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

        let out = output.lock().unwrap();
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

        let out = output.lock().unwrap();
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
        let output = api.output_buffer_ref();
        api.register(engine.lua()).unwrap();

        engine.execute("bbs.clear()").unwrap();

        let out = output.lock().unwrap();
        // No output when ANSI is disabled
        assert!(out.is_empty());
    }

    #[test]
    fn test_bbs_input_no_handler() {
        let (engine, _) = create_test_engine_with_api();

        // Should error when no script handle is available
        let result = engine.execute("result = bbs.input()");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Input not available"));
    }

    #[test]
    fn test_bbs_input_with_runtime() {
        let (runtime, handle) = create_script_runtime();
        let handle = Arc::new(handle);

        let context = ScriptContext {
            script_id: Some(1),
            user_id: Some(42),
            username: "testuser".to_string(),
            nickname: "Test User".to_string(),
            user_role: 1,
            terminal_width: 80,
            terminal_height: 24,
            has_ansi: true,
            lang: "en".to_string(),
            translations: std::collections::HashMap::new(),
        };

        let handle_clone = Arc::clone(&handle);

        // Run script in a separate thread
        let script_thread = thread::spawn(move || {
            let engine = ScriptEngine::new().unwrap();
            let api = BbsApi::new(context).with_script_handle(handle_clone);
            api.register(engine.lua()).unwrap();

            engine.execute(r#"result = bbs.input("Enter name: ")"#).unwrap();
            engine.get_global::<String>("result").unwrap()
        });

        // Handle the input request
        match runtime.recv() {
            Some(crate::script::ScriptMessage::InputRequest { prompt }) => {
                assert_eq!(prompt, Some("Enter name: ".to_string()));
                runtime.send_input(Some("TestUser".to_string()));
            }
            _ => panic!("Expected InputRequest"),
        }

        let result = script_thread.join().unwrap();
        assert_eq!(result, "TestUser");
    }

    #[test]
    fn test_bbs_input_number_with_runtime() {
        let (runtime, handle) = create_script_runtime();
        let handle = Arc::new(handle);

        let context = ScriptContext::default();
        let handle_clone = Arc::clone(&handle);

        let script_thread = thread::spawn(move || {
            let engine = ScriptEngine::new().unwrap();
            let api = BbsApi::new(context).with_script_handle(handle_clone);
            api.register(engine.lua()).unwrap();

            engine.execute("result = bbs.input_number()").unwrap();
            engine.get_global::<f64>("result").unwrap()
        });

        // Handle the input request
        if let Some(crate::script::ScriptMessage::InputRequest { .. }) = runtime.recv() {
            runtime.send_input(Some("42.5".to_string()));
        }

        let result = script_thread.join().unwrap();
        assert!((result - 42.5).abs() < 0.001);
    }

    #[test]
    fn test_bbs_input_yn_with_runtime() {
        let (runtime, handle) = create_script_runtime();
        let handle = Arc::new(handle);

        let context = ScriptContext::default();
        let handle_clone = Arc::clone(&handle);

        let script_thread = thread::spawn(move || {
            let engine = ScriptEngine::new().unwrap();
            let api = BbsApi::new(context).with_script_handle(handle_clone);
            api.register(engine.lua()).unwrap();

            engine.execute("result = bbs.input_yn()").unwrap();
            engine.get_global::<bool>("result").unwrap()
        });

        // Handle the input request
        if let Some(crate::script::ScriptMessage::InputRequest { .. }) = runtime.recv() {
            runtime.send_input(Some("yes".to_string()));
        }

        let result = script_thread.join().unwrap();
        assert!(result);
    }

    #[test]
    fn test_bbs_pause_with_runtime() {
        let (runtime, handle) = create_script_runtime();
        let handle = Arc::new(handle);

        let context = ScriptContext::default();
        let handle_clone = Arc::clone(&handle);

        let script_thread = thread::spawn(move || {
            let engine = ScriptEngine::new().unwrap();
            let api = BbsApi::new(context).with_script_handle(handle_clone);
            api.register(engine.lua()).unwrap();

            engine.execute("bbs.pause()").unwrap();
        });

        // Handle the pause request
        match runtime.recv() {
            Some(crate::script::ScriptMessage::InputRequest { prompt }) => {
                assert_eq!(prompt, Some("Press Enter to continue...".to_string()));
                runtime.send_input(Some(String::new()));
            }
            _ => panic!("Expected InputRequest for pause"),
        }

        script_thread.join().unwrap();
    }

    #[test]
    fn test_output_through_runtime() {
        let (runtime, handle) = create_script_runtime();
        let handle = Arc::new(handle);

        let context = ScriptContext::default();
        let handle_clone = Arc::clone(&handle);

        let script_thread = thread::spawn(move || {
            let engine = ScriptEngine::new().unwrap();
            let api = BbsApi::new(context).with_script_handle(handle_clone);
            api.register(engine.lua()).unwrap();

            engine
                .execute(
                    r#"
                bbs.println("Hello")
                bbs.println("World")
            "#,
                )
                .unwrap();
        });

        // Collect output messages
        let mut outputs = Vec::new();
        loop {
            match runtime.recv_timeout(std::time::Duration::from_millis(100)) {
                Some(crate::script::ScriptMessage::Output(text)) => outputs.push(text),
                _ => break,
            }
        }

        script_thread.join().unwrap();

        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0], "Hello\n");
        assert_eq!(outputs[1], "World\n");
    }

    #[test]
    fn test_bbs_get_lang() {
        let engine = ScriptEngine::new().unwrap();
        let context = ScriptContext {
            lang: "ja".to_string(),
            ..Default::default()
        };
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        engine.execute("result = bbs.get_lang()").unwrap();
        let result: String = engine.get_global("result").unwrap();
        assert_eq!(result, "ja");
    }

    #[test]
    fn test_bbs_t_with_translations() {
        let engine = ScriptEngine::new().unwrap();

        let mut ja_translations = std::collections::HashMap::new();
        ja_translations.insert("title".to_string(), "じゃんけん".to_string());
        ja_translations.insert("rock".to_string(), "グー".to_string());

        let mut en_translations = std::collections::HashMap::new();
        en_translations.insert("title".to_string(), "Rock-Paper-Scissors".to_string());
        en_translations.insert("rock".to_string(), "Rock".to_string());

        let mut translations = std::collections::HashMap::new();
        translations.insert("ja".to_string(), ja_translations);
        translations.insert("en".to_string(), en_translations);

        let context = ScriptContext {
            lang: "ja".to_string(),
            translations,
            ..Default::default()
        };
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        // Test getting Japanese translation
        engine.execute(r#"result = bbs.t("title")"#).unwrap();
        let result: String = engine.get_global("result").unwrap();
        assert_eq!(result, "じゃんけん");

        engine.execute(r#"result = bbs.t("rock")"#).unwrap();
        let result: String = engine.get_global("result").unwrap();
        assert_eq!(result, "グー");
    }

    #[test]
    fn test_bbs_t_fallback_to_english() {
        let engine = ScriptEngine::new().unwrap();

        let mut en_translations = std::collections::HashMap::new();
        en_translations.insert("title".to_string(), "Rock-Paper-Scissors".to_string());

        let mut translations = std::collections::HashMap::new();
        translations.insert("en".to_string(), en_translations);

        // User language is German, but we only have English translations
        let context = ScriptContext {
            lang: "de".to_string(),
            translations,
            ..Default::default()
        };
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        // Should fallback to English
        engine.execute(r#"result = bbs.t("title")"#).unwrap();
        let result: String = engine.get_global("result").unwrap();
        assert_eq!(result, "Rock-Paper-Scissors");
    }

    #[test]
    fn test_bbs_t_fallback_to_key() {
        let engine = ScriptEngine::new().unwrap();

        let context = ScriptContext {
            lang: "ja".to_string(),
            translations: std::collections::HashMap::new(),
            ..Default::default()
        };
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        // No translations, should return the key
        engine.execute(r#"result = bbs.t("unknown_key")"#).unwrap();
        let result: String = engine.get_global("result").unwrap();
        assert_eq!(result, "unknown_key");
    }

    #[test]
    fn test_bbs_t_with_default() {
        let engine = ScriptEngine::new().unwrap();

        let context = ScriptContext {
            lang: "ja".to_string(),
            translations: std::collections::HashMap::new(),
            ..Default::default()
        };
        let api = BbsApi::new(context);
        api.register(engine.lua()).unwrap();

        // No translations, should return the provided default
        engine
            .execute(r#"result = bbs.t("unknown_key", "Default Value")"#)
            .unwrap();
        let result: String = engine.get_global("result").unwrap();
        assert_eq!(result, "Default Value");
    }
}
