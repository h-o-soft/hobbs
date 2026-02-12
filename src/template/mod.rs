//! Template engine module for HOBBS.
//!
//! Provides a Handlebars-style template engine for rendering dynamic content.
//!
//! # Features
//!
//! - Variable expansion: `{{variable}}`
//! - Translation reference: `{{t "key"}}` or `{{t "key" name=value}}`
//! - Conditionals: `{{#if condition}}...{{else}}...{{/if}}`
//! - Loops: `{{#each items}}...{{/each}}`
//! - Escaping: `\{{` to output literal `{{`
//!
//! # Example
//!
//! ```
//! use hobbs::template::{TemplateEngine, TemplateContext, Value};
//! use hobbs::i18n::I18n;
//! use std::sync::Arc;
//!
//! let mut engine = TemplateEngine::new();
//! engine.load("greeting", "Hello, {{name}}!");
//!
//! let i18n = Arc::new(I18n::empty("en"));
//! let mut context = TemplateContext::new(i18n);
//! context.set("name", Value::String("World".to_string()));
//!
//! let result = engine.render("greeting", &context).unwrap();
//! assert_eq!(result, "Hello, World!");
//! ```

mod loader;
mod parser;
mod renderer;

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;

use crate::i18n::I18n;

pub use loader::{create_system_context, TemplateLoader, WIDTH_40, WIDTH_80};
pub use parser::{Node, Parser};
pub use renderer::Renderer;

/// Calculate the display width of a string considering CJK character width.
pub fn display_width(s: &str, cjk_width: usize) -> usize {
    if cjk_width == 1 {
        s.chars().count()
    } else {
        s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
    }
}

/// Truncate a string to fit within the specified display width.
pub fn truncate_to_width(s: &str, max_width: usize, cjk_width: usize) -> String {
    let mut result = String::new();
    let mut current_width = 0;

    for c in s.chars() {
        let char_width = if cjk_width == 1 || c.is_ascii() {
            1
        } else {
            2
        };

        if current_width + char_width > max_width {
            break;
        }

        result.push(c);
        current_width += char_width;
    }

    result
}

/// Template-related errors.
#[derive(Error, Debug)]
pub enum TemplateError {
    /// Template not found.
    #[error("Template not found: {0}")]
    NotFound(String),

    /// Parse error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Render error.
    #[error("Render error: {0}")]
    Render(String),

    /// Variable not found.
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
}

/// Result type for template operations.
pub type Result<T> = std::result::Result<T, TemplateError>;

/// A value that can be used in templates.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A string value.
    String(String),
    /// A numeric value.
    Number(i64),
    /// A floating-point value.
    Float(f64),
    /// A boolean value.
    Bool(bool),
    /// A list of values.
    List(Vec<Value>),
    /// An object (key-value pairs).
    Object(HashMap<String, Value>),
    /// A null/empty value.
    Null,
}

impl Value {
    /// Convert the value to a string for display.
    pub fn to_display_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::List(_) => "[list]".to_string(),
            Value::Object(_) => "[object]".to_string(),
            Value::Null => "".to_string(),
        }
    }

    /// Check if the value is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Bool(b) => *b,
            Value::List(l) => !l.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Null => false,
        }
    }

    /// Get a nested value by dot-separated path.
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                Value::List(list) => {
                    let index: usize = part.parse().ok()?;
                    current = list.get(index)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Create a Value from a string.
    pub fn string(s: impl Into<String>) -> Self {
        Value::String(s.into())
    }

    /// Create a Value from a number.
    pub fn number(n: i64) -> Self {
        Value::Number(n)
    }

    /// Create a Value from a boolean.
    pub fn bool(b: bool) -> Self {
        Value::Bool(b)
    }

    /// Create a list Value.
    pub fn list(items: Vec<Value>) -> Self {
        Value::List(items)
    }

    /// Create an object Value.
    pub fn object(items: HashMap<String, Value>) -> Self {
        Value::Object(items)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(n as i64)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::List(v.into_iter().map(Into::into).collect())
    }
}

/// Context for template rendering.
#[derive(Debug, Clone)]
pub struct TemplateContext {
    /// Variables available in the template.
    variables: HashMap<String, Value>,
    /// Internationalization instance.
    i18n: Arc<I18n>,
    /// CJK character width (1 or 2). Used by {{pad}} helper.
    cjk_width: usize,
}

impl TemplateContext {
    /// Create a new template context.
    pub fn new(i18n: Arc<I18n>) -> Self {
        Self {
            variables: HashMap::new(),
            i18n,
            cjk_width: 2,
        }
    }

    /// Set the CJK character width (1 or 2).
    pub fn set_cjk_width(&mut self, width: usize) {
        self.cjk_width = width;
    }

    /// Get the CJK character width.
    pub fn cjk_width(&self) -> usize {
        self.cjk_width
    }

    /// Set a variable in the context.
    pub fn set(&mut self, name: impl Into<String>, value: Value) {
        self.variables.insert(name.into(), value);
    }

    /// Get a variable from the context.
    pub fn get(&self, name: &str) -> Option<&Value> {
        // First try direct lookup
        if let Some(value) = self.variables.get(name) {
            return Some(value);
        }

        // Try dot-notation path lookup
        if name.contains('.') {
            let parts: Vec<&str> = name.splitn(2, '.').collect();
            if parts.len() == 2 {
                if let Some(root) = self.variables.get(parts[0]) {
                    return root.get_path(parts[1]);
                }
            }
        }

        None
    }

    /// Get the i18n instance.
    pub fn i18n(&self) -> &I18n {
        &self.i18n
    }

    /// Set multiple variables from a HashMap.
    pub fn set_many(&mut self, variables: HashMap<String, Value>) {
        self.variables.extend(variables);
    }

    /// Clear all variables.
    pub fn clear(&mut self) {
        self.variables.clear();
    }

    /// Create a child context with additional variables.
    ///
    /// The child context inherits all variables from the parent.
    pub fn child(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            i18n: Arc::clone(&self.i18n),
            cjk_width: self.cjk_width,
        }
    }
}

/// Template engine for parsing and rendering templates.
#[derive(Debug, Default)]
pub struct TemplateEngine {
    /// Parsed templates.
    templates: HashMap<String, Vec<Node>>,
}

impl TemplateEngine {
    /// Create a new template engine.
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Load a template from a string.
    pub fn load(&mut self, name: impl Into<String>, content: &str) -> Result<()> {
        let parser = Parser::new(content);
        let nodes = parser.parse()?;
        self.templates.insert(name.into(), nodes);
        Ok(())
    }

    /// Render a template with the given context.
    pub fn render(&self, name: &str, context: &TemplateContext) -> Result<String> {
        let nodes = self
            .templates
            .get(name)
            .ok_or_else(|| TemplateError::NotFound(name.to_string()))?;

        let renderer = Renderer::new(context);
        renderer.render(nodes)
    }

    /// Render a template string directly without loading.
    pub fn render_string(content: &str, context: &TemplateContext) -> Result<String> {
        let parser = Parser::new(content);
        let nodes = parser.parse()?;
        let renderer = Renderer::new(context);
        renderer.render(&nodes)
    }

    /// Check if a template is loaded.
    pub fn has_template(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Get the list of loaded template names.
    pub fn template_names(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a template.
    pub fn unload(&mut self, name: &str) -> bool {
        self.templates.remove(name).is_some()
    }

    /// Clear all loaded templates.
    pub fn clear(&mut self) {
        self.templates.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_context() -> TemplateContext {
        let i18n = Arc::new(I18n::empty("ja"));
        TemplateContext::new(i18n)
    }

    // Value tests
    #[test]
    fn test_value_to_display_string() {
        assert_eq!(
            Value::String("hello".to_string()).to_display_string(),
            "hello"
        );
        assert_eq!(Value::Number(42).to_display_string(), "42");
        assert_eq!(Value::Float(3.14).to_display_string(), "3.14");
        assert_eq!(Value::Bool(true).to_display_string(), "true");
        assert_eq!(Value::Bool(false).to_display_string(), "false");
        assert_eq!(Value::List(vec![]).to_display_string(), "[list]");
        assert_eq!(
            Value::Object(HashMap::new()).to_display_string(),
            "[object]"
        );
        assert_eq!(Value::Null.to_display_string(), "");
    }

    #[test]
    fn test_value_is_truthy() {
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(Value::Number(1).is_truthy());
        assert!(!Value::Number(0).is_truthy());
        assert!(Value::Float(0.1).is_truthy());
        assert!(!Value::Float(0.0).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::List(vec![Value::Number(1)]).is_truthy());
        assert!(!Value::List(vec![]).is_truthy());
        assert!(!Value::Null.is_truthy());
    }

    #[test]
    fn test_value_get_path() {
        let mut inner = HashMap::new();
        inner.insert("name".to_string(), Value::String("test".to_string()));
        inner.insert("count".to_string(), Value::Number(5));

        let value = Value::Object({
            let mut map = HashMap::new();
            map.insert("user".to_string(), Value::Object(inner));
            map
        });

        assert_eq!(
            value.get_path("user.name"),
            Some(&Value::String("test".to_string()))
        );
        assert_eq!(value.get_path("user.count"), Some(&Value::Number(5)));
        assert_eq!(value.get_path("user.missing"), None);
        assert_eq!(value.get_path("missing"), None);
    }

    #[test]
    fn test_value_from_traits() {
        let s: Value = "hello".into();
        assert_eq!(s, Value::String("hello".to_string()));

        let n: Value = 42i64.into();
        assert_eq!(n, Value::Number(42));

        let n32: Value = 42i32.into();
        assert_eq!(n32, Value::Number(42));

        let b: Value = true.into();
        assert_eq!(b, Value::Bool(true));

        let list: Value = vec!["a", "b", "c"].into();
        assert_eq!(
            list,
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ])
        );
    }

    // TemplateContext tests
    #[test]
    fn test_context_set_and_get() {
        let mut context = create_context();
        context.set("name", Value::String("Alice".to_string()));
        context.set("age", Value::Number(30));

        assert_eq!(
            context.get("name"),
            Some(&Value::String("Alice".to_string()))
        );
        assert_eq!(context.get("age"), Some(&Value::Number(30)));
        assert_eq!(context.get("missing"), None);
    }

    #[test]
    fn test_context_get_nested() {
        let mut context = create_context();

        let mut user = HashMap::new();
        user.insert("name".to_string(), Value::String("Bob".to_string()));
        user.insert("age".to_string(), Value::Number(25));
        context.set("user", Value::Object(user));

        assert_eq!(
            context.get("user.name"),
            Some(&Value::String("Bob".to_string()))
        );
        assert_eq!(context.get("user.age"), Some(&Value::Number(25)));
        assert_eq!(context.get("user.missing"), None);
    }

    #[test]
    fn test_context_child() {
        let mut context = create_context();
        context.set("parent", Value::String("parent_value".to_string()));

        let mut child = context.child();
        child.set("child", Value::String("child_value".to_string()));

        // Child has both parent and child variables
        assert_eq!(
            child.get("parent"),
            Some(&Value::String("parent_value".to_string()))
        );
        assert_eq!(
            child.get("child"),
            Some(&Value::String("child_value".to_string()))
        );

        // Parent doesn't have child variable
        assert_eq!(context.get("child"), None);
    }

    #[test]
    fn test_context_set_many() {
        let mut context = create_context();
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Value::Number(1));
        vars.insert("b".to_string(), Value::Number(2));
        context.set_many(vars);

        assert_eq!(context.get("a"), Some(&Value::Number(1)));
        assert_eq!(context.get("b"), Some(&Value::Number(2)));
    }

    #[test]
    fn test_context_clear() {
        let mut context = create_context();
        context.set("name", Value::String("test".to_string()));
        assert!(context.get("name").is_some());

        context.clear();
        assert!(context.get("name").is_none());
    }

    // TemplateEngine tests
    #[test]
    fn test_engine_load_and_render() {
        let mut engine = TemplateEngine::new();
        engine.load("test", "Hello, {{name}}!").unwrap();

        let mut context = create_context();
        context.set("name", Value::String("World".to_string()));

        let result = engine.render("test", &context).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_engine_render_not_found() {
        let engine = TemplateEngine::new();
        let context = create_context();

        let result = engine.render("missing", &context);
        assert!(matches!(result, Err(TemplateError::NotFound(_))));
    }

    #[test]
    fn test_engine_render_string() {
        let mut context = create_context();
        context.set("x", Value::Number(10));

        let result = TemplateEngine::render_string("x = {{x}}", &context).unwrap();
        assert_eq!(result, "x = 10");
    }

    #[test]
    fn test_engine_has_template() {
        let mut engine = TemplateEngine::new();
        assert!(!engine.has_template("test"));

        engine.load("test", "content").unwrap();
        assert!(engine.has_template("test"));
    }

    #[test]
    fn test_engine_template_names() {
        let mut engine = TemplateEngine::new();
        engine.load("a", "").unwrap();
        engine.load("b", "").unwrap();

        let names = engine.template_names();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_engine_unload() {
        let mut engine = TemplateEngine::new();
        engine.load("test", "content").unwrap();
        assert!(engine.has_template("test"));

        assert!(engine.unload("test"));
        assert!(!engine.has_template("test"));
        assert!(!engine.unload("test")); // Already removed
    }

    #[test]
    fn test_engine_clear() {
        let mut engine = TemplateEngine::new();
        engine.load("a", "").unwrap();
        engine.load("b", "").unwrap();

        engine.clear();
        assert!(!engine.has_template("a"));
        assert!(!engine.has_template("b"));
    }
}
