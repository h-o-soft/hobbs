//! Template renderer module.
//!
//! Renders parsed template nodes with the given context.

use super::parser::Node;
use super::{display_width, truncate_to_width, Result, TemplateContext, TemplateError, Value};

/// Template renderer.
pub struct Renderer<'a> {
    context: &'a TemplateContext,
}

impl<'a> Renderer<'a> {
    /// Create a new renderer with the given context.
    pub fn new(context: &'a TemplateContext) -> Self {
        Self { context }
    }

    /// Render a list of nodes to a string.
    pub fn render(&self, nodes: &[Node]) -> Result<String> {
        let mut output = String::new();

        for node in nodes {
            output.push_str(&self.render_node(node)?);
        }

        Ok(output)
    }

    /// Render a single node.
    fn render_node(&self, node: &Node) -> Result<String> {
        match node {
            Node::Text(text) => Ok(text.clone()),
            Node::Variable(name) => self.render_variable(name),
            Node::Translation { key, params } => self.render_translation(key, params),
            Node::If {
                condition,
                then_branch,
                else_branch,
            } => self.render_if(condition, then_branch, else_branch),
            Node::Each {
                variable,
                item_name,
                body,
            } => self.render_each(variable, item_name.as_deref(), body),
            Node::Unless { condition, body } => self.render_unless(condition, body),
            Node::With { variable, body } => self.render_with(variable, body),
            Node::Pad { variable, width } => self.render_pad(variable, width),
        }
    }

    /// Render a variable reference.
    fn render_variable(&self, name: &str) -> Result<String> {
        match self.context.get(name) {
            Some(value) => Ok(value.to_display_string()),
            None => {
                // Return empty string for missing variables (like Handlebars)
                Ok(String::new())
            }
        }
    }

    /// Render a translation reference.
    fn render_translation(&self, key: &str, params: &[(String, String)]) -> Result<String> {
        let i18n = self.context.i18n();

        if params.is_empty() {
            Ok(i18n.t(key).to_string())
        } else {
            // Build parameter list, resolving variable references
            let resolved_params: Vec<(&str, String)> = params
                .iter()
                .map(|(name, value)| {
                    let resolved = if value.starts_with('"') && value.ends_with('"') {
                        // Literal string - strip the quotes
                        value[1..value.len() - 1].to_string()
                    } else {
                        // Variable reference
                        self.context
                            .get(value)
                            .map(|v| v.to_display_string())
                            .unwrap_or_default()
                    };
                    (name.as_str(), resolved)
                })
                .collect();

            let param_refs: Vec<(&str, &str)> = resolved_params
                .iter()
                .map(|(name, value)| (*name, value.as_str()))
                .collect();

            Ok(i18n.t_with(key, &param_refs))
        }
    }

    /// Render an if block.
    fn render_if(
        &self,
        condition: &str,
        then_branch: &[Node],
        else_branch: &[Node],
    ) -> Result<String> {
        let is_truthy = self
            .context
            .get(condition)
            .map(|v| v.is_truthy())
            .unwrap_or(false);

        if is_truthy {
            self.render(then_branch)
        } else {
            self.render(else_branch)
        }
    }

    /// Render an each block.
    fn render_each(
        &self,
        variable: &str,
        item_name: Option<&str>,
        body: &[Node],
    ) -> Result<String> {
        let list = match self.context.get(variable) {
            Some(Value::List(items)) => items,
            Some(_) => {
                return Err(TemplateError::Render(format!("'{variable}' is not a list")));
            }
            None => return Ok(String::new()),
        };

        let mut output = String::new();
        let item_var_name = item_name.unwrap_or("this");

        for (index, item) in list.iter().enumerate() {
            // Create a child context with loop variables
            let mut child_context = self.context.child();
            child_context.set(item_var_name, item.clone());
            child_context.set("@index", Value::Number(index as i64));
            child_context.set("@first", Value::Bool(index == 0));
            child_context.set("@last", Value::Bool(index == list.len() - 1));

            // If item is an object, also expose its fields directly
            if let Value::Object(obj) = item {
                for (key, value) in obj {
                    child_context.set(key.clone(), value.clone());
                }
            }

            let child_renderer = Renderer::new(&child_context);
            output.push_str(&child_renderer.render(body)?);
        }

        Ok(output)
    }

    /// Render an unless block.
    fn render_unless(&self, condition: &str, body: &[Node]) -> Result<String> {
        let is_truthy = self
            .context
            .get(condition)
            .map(|v| v.is_truthy())
            .unwrap_or(false);

        if !is_truthy {
            self.render(body)
        } else {
            Ok(String::new())
        }
    }

    /// Render a pad helper: pad or truncate a value to a fixed display width.
    /// If the source is quoted (e.g. `"board.title"`), it's resolved as a translation key.
    /// Otherwise, it's resolved as a variable reference.
    fn render_pad(&self, variable: &str, width_str: &str) -> Result<String> {
        let text = if variable.starts_with('"') && variable.ends_with('"') {
            // Translation key - strip quotes and look up
            let key = &variable[1..variable.len() - 1];
            self.context.i18n().t(key).to_string()
        } else {
            self.context
                .get(variable)
                .map(|v| v.to_display_string())
                .unwrap_or_default()
        };

        let target_width: usize = width_str.parse().map_err(|_| {
            TemplateError::Render(format!("Invalid pad width: {width_str}"))
        })?;

        if target_width == 0 {
            return Ok(String::new());
        }

        let cjk_width = self.context.cjk_width();
        let display_w = display_width(&text, cjk_width);

        if display_w <= target_width {
            // Pad with spaces
            let padding = target_width - display_w;
            Ok(format!("{}{}", text, " ".repeat(padding)))
        } else {
            // Truncate to (target_width - 1) and append '~'
            let truncated = truncate_to_width(&text, target_width - 1, cjk_width);
            let truncated_w = display_width(&truncated, cjk_width);
            let padding = target_width - 1 - truncated_w;
            Ok(format!("{}{}~", truncated, " ".repeat(padding)))
        }
    }

    /// Render a with block.
    fn render_with(&self, variable: &str, body: &[Node]) -> Result<String> {
        let value = match self.context.get(variable) {
            Some(v) => v.clone(),
            None => return Ok(String::new()),
        };

        let mut child_context = self.context.child();

        // If value is an object, expose its fields directly
        if let Value::Object(obj) = &value {
            for (key, val) in obj {
                child_context.set(key.clone(), val.clone());
            }
        }

        // Also expose the value as "this"
        child_context.set("this", value);

        let child_renderer = Renderer::new(&child_context);
        child_renderer.render(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::I18n;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn create_context() -> TemplateContext {
        let i18n = Arc::new(I18n::empty("ja"));
        TemplateContext::new(i18n)
    }

    fn create_context_with_i18n() -> TemplateContext {
        let content = r#"
[menu]
main = "メインメニュー"

[welcome]
message = "こんにちは、{{name}}さん"
"#;
        let i18n = Arc::new(I18n::from_str("ja", content).unwrap());
        TemplateContext::new(i18n)
    }

    #[test]
    fn test_render_text() {
        let context = create_context();
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Text("Hello, World!".to_string())];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_render_variable() {
        let mut context = create_context();
        context.set("name", Value::String("Alice".to_string()));
        let renderer = Renderer::new(&context);

        let nodes = vec![
            Node::Text("Hello, ".to_string()),
            Node::Variable("name".to_string()),
            Node::Text("!".to_string()),
        ];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_render_variable_missing() {
        let context = create_context();
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Variable("missing".to_string())];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_render_variable_nested() {
        let mut context = create_context();
        let mut user = HashMap::new();
        user.insert("name".to_string(), Value::String("Bob".to_string()));
        context.set("user", Value::Object(user));

        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Variable("user.name".to_string())];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "Bob");
    }

    #[test]
    fn test_render_translation_simple() {
        let context = create_context_with_i18n();
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Translation {
            key: "menu.main".to_string(),
            params: vec![],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "メインメニュー");
    }

    #[test]
    fn test_render_translation_with_params() {
        let context = create_context_with_i18n();
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Translation {
            key: "welcome.message".to_string(),
            // Literal strings are wrapped in quotes by parser
            params: vec![("name".to_string(), "\"太郎\"".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "こんにちは、太郎さん");
    }

    #[test]
    fn test_render_translation_with_variable_param() {
        let mut context = create_context_with_i18n();
        context.set("username", Value::String("花子".to_string()));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Translation {
            key: "welcome.message".to_string(),
            params: vec![("name".to_string(), "username".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "こんにちは、花子さん");
    }

    #[test]
    fn test_render_if_true() {
        let mut context = create_context();
        context.set("show", Value::Bool(true));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::If {
            condition: "show".to_string(),
            then_branch: vec![Node::Text("visible".to_string())],
            else_branch: vec![Node::Text("hidden".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "visible");
    }

    #[test]
    fn test_render_if_false() {
        let mut context = create_context();
        context.set("show", Value::Bool(false));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::If {
            condition: "show".to_string(),
            then_branch: vec![Node::Text("visible".to_string())],
            else_branch: vec![Node::Text("hidden".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "hidden");
    }

    #[test]
    fn test_render_if_missing_variable() {
        let context = create_context();
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::If {
            condition: "missing".to_string(),
            then_branch: vec![Node::Text("yes".to_string())],
            else_branch: vec![Node::Text("no".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        // Missing variable is falsy
        assert_eq!(result, "no");
    }

    #[test]
    fn test_render_if_truthy_string() {
        let mut context = create_context();
        context.set("name", Value::String("Alice".to_string()));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::If {
            condition: "name".to_string(),
            then_branch: vec![Node::Text("has name".to_string())],
            else_branch: vec![],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "has name");
    }

    #[test]
    fn test_render_each() {
        let mut context = create_context();
        context.set(
            "items",
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ]),
        );
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Each {
            variable: "items".to_string(),
            item_name: Some("item".to_string()),
            body: vec![
                Node::Text("[".to_string()),
                Node::Variable("item".to_string()),
                Node::Text("]".to_string()),
            ],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "[a][b][c]");
    }

    #[test]
    fn test_render_each_with_index() {
        let mut context = create_context();
        context.set(
            "items",
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ]),
        );
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Each {
            variable: "items".to_string(),
            item_name: Some("item".to_string()),
            body: vec![
                Node::Variable("@index".to_string()),
                Node::Text(":".to_string()),
                Node::Variable("item".to_string()),
                Node::Text(" ".to_string()),
            ],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "0:a 1:b ");
    }

    #[test]
    fn test_render_each_with_objects() {
        let mut context = create_context();
        context.set(
            "users",
            Value::List(vec![
                Value::Object({
                    let mut m = HashMap::new();
                    m.insert("name".to_string(), Value::String("Alice".to_string()));
                    m
                }),
                Value::Object({
                    let mut m = HashMap::new();
                    m.insert("name".to_string(), Value::String("Bob".to_string()));
                    m
                }),
            ]),
        );
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Each {
            variable: "users".to_string(),
            item_name: None,
            body: vec![
                Node::Variable("name".to_string()),
                Node::Text(", ".to_string()),
            ],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "Alice, Bob, ");
    }

    #[test]
    fn test_render_each_empty_list() {
        let mut context = create_context();
        context.set("items", Value::List(vec![]));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Each {
            variable: "items".to_string(),
            item_name: None,
            body: vec![Node::Text("item".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_render_each_missing_variable() {
        let context = create_context();
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Each {
            variable: "missing".to_string(),
            item_name: None,
            body: vec![Node::Text("item".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_render_unless_false() {
        let mut context = create_context();
        context.set("hidden", Value::Bool(false));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Unless {
            condition: "hidden".to_string(),
            body: vec![Node::Text("shown".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "shown");
    }

    #[test]
    fn test_render_unless_true() {
        let mut context = create_context();
        context.set("hidden", Value::Bool(true));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Unless {
            condition: "hidden".to_string(),
            body: vec![Node::Text("shown".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_render_with() {
        let mut context = create_context();
        context.set(
            "user",
            Value::Object({
                let mut m = HashMap::new();
                m.insert("name".to_string(), Value::String("Charlie".to_string()));
                m.insert("age".to_string(), Value::Number(30));
                m
            }),
        );
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::With {
            variable: "user".to_string(),
            body: vec![
                Node::Variable("name".to_string()),
                Node::Text(" is ".to_string()),
                Node::Variable("age".to_string()),
                Node::Text(" years old".to_string()),
            ],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "Charlie is 30 years old");
    }

    #[test]
    fn test_render_with_missing() {
        let context = create_context();
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::With {
            variable: "missing".to_string(),
            body: vec![Node::Text("content".to_string())],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_render_nested_blocks() {
        let mut context = create_context();
        context.set("show_list", Value::Bool(true));
        context.set(
            "items",
            Value::List(vec![
                Value::String("x".to_string()),
                Value::String("y".to_string()),
            ]),
        );
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::If {
            condition: "show_list".to_string(),
            then_branch: vec![
                Node::Text("Items: ".to_string()),
                Node::Each {
                    variable: "items".to_string(),
                    item_name: Some("i".to_string()),
                    body: vec![Node::Variable("i".to_string()), Node::Text(" ".to_string())],
                },
            ],
            else_branch: vec![],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "Items: x y ");
    }

    #[test]
    fn test_render_pad_short_string() {
        let mut context = create_context();
        context.set("name", Value::String("Alice".to_string()));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Pad {
            variable: "name".to_string(),
            width: "10".to_string(),
        }];
        let result = renderer.render(&nodes).unwrap();
        assert_eq!(result, "Alice     ");
    }

    #[test]
    fn test_render_pad_exact_width() {
        let mut context = create_context();
        context.set("name", Value::String("Hello".to_string()));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Pad {
            variable: "name".to_string(),
            width: "5".to_string(),
        }];
        let result = renderer.render(&nodes).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_render_pad_truncate() {
        let mut context = create_context();
        context.set("name", Value::String("Hello, World!".to_string()));
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Pad {
            variable: "name".to_string(),
            width: "8".to_string(),
        }];
        let result = renderer.render(&nodes).unwrap();
        // 7 chars + '~' = 8
        assert_eq!(result, "Hello, ~");
    }

    #[test]
    fn test_render_pad_cjk() {
        let mut context = create_context();
        context.set_cjk_width(2);
        context.set("name", Value::String("こんにちは".to_string()));
        let renderer = Renderer::new(&context);

        // "こんにちは" = 10 display columns with cjk_width=2
        // pad to 12 → "こんにちは  "
        let nodes = vec![Node::Pad {
            variable: "name".to_string(),
            width: "12".to_string(),
        }];
        let result = renderer.render(&nodes).unwrap();
        assert_eq!(result, "こんにちは  ");
    }

    #[test]
    fn test_render_pad_cjk_truncate() {
        let mut context = create_context();
        context.set_cjk_width(2);
        context.set("name", Value::String("こんにちは世界".to_string()));
        let renderer = Renderer::new(&context);

        // "こんにちは世界" = 14 display columns
        // pad to 8 → truncate to 7 cols = "こんに" (6) + "~" (1) = 7... need padding to 8
        // Actually: truncate_to_width("こんにちは世界", 7, 2) → "こんに" (6 cols, next char is 2 cols which exceeds 7)
        // So "こんに" + " " + "~" = 8
        let nodes = vec![Node::Pad {
            variable: "name".to_string(),
            width: "8".to_string(),
        }];
        let result = renderer.render(&nodes).unwrap();
        assert_eq!(result, "こんに ~");
    }

    #[test]
    fn test_render_pad_translation() {
        let context = create_context_with_i18n();
        let renderer = Renderer::new(&context);

        // "menu.main" = "メインメニュー" (7 CJK chars = 14 display cols)
        let nodes = vec![Node::Pad {
            variable: "\"menu.main\"".to_string(),
            width: "20".to_string(),
        }];
        let result = renderer.render(&nodes).unwrap();
        assert_eq!(result, "メインメニュー      ");
    }

    #[test]
    fn test_render_pad_translation_truncate() {
        let context = create_context_with_i18n();
        let renderer = Renderer::new(&context);

        // "menu.main" = "メインメニュー" (14 display cols), truncate to 8
        let nodes = vec![Node::Pad {
            variable: "\"menu.main\"".to_string(),
            width: "8".to_string(),
        }];
        let result = renderer.render(&nodes).unwrap();
        // truncate to 7 cols = "メイン" (6 cols) + " " + "~" = 8
        assert_eq!(result, "メイン ~");
    }

    #[test]
    fn test_render_pad_missing_variable() {
        let context = create_context();
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Pad {
            variable: "missing".to_string(),
            width: "5".to_string(),
        }];
        let result = renderer.render(&nodes).unwrap();
        assert_eq!(result, "     ");
    }

    #[test]
    fn test_render_first_last() {
        let mut context = create_context();
        context.set(
            "items",
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ]),
        );
        let renderer = Renderer::new(&context);

        let nodes = vec![Node::Each {
            variable: "items".to_string(),
            item_name: Some("item".to_string()),
            body: vec![
                Node::If {
                    condition: "@first".to_string(),
                    then_branch: vec![Node::Text("[FIRST]".to_string())],
                    else_branch: vec![],
                },
                Node::Variable("item".to_string()),
                Node::If {
                    condition: "@last".to_string(),
                    then_branch: vec![Node::Text("[LAST]".to_string())],
                    else_branch: vec![],
                },
                Node::Text(" ".to_string()),
            ],
        }];
        let result = renderer.render(&nodes).unwrap();

        assert_eq!(result, "[FIRST]a b c[LAST] ");
    }
}
