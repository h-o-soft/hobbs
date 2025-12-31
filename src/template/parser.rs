//! Template parser module.
//!
//! Parses template strings into an AST (Abstract Syntax Tree) of nodes.

use super::{Result, TemplateError};

/// A node in the template AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Raw text content.
    Text(String),

    /// Variable reference: `{{name}}` or `{{user.name}}`
    Variable(String),

    /// Translation reference: `{{t "key"}}` or `{{t "key" param=value}}`
    Translation {
        key: String,
        params: Vec<(String, String)>,
    },

    /// Conditional block: `{{#if condition}}...{{else}}...{{/if}}`
    If {
        condition: String,
        then_branch: Vec<Node>,
        else_branch: Vec<Node>,
    },

    /// Loop block: `{{#each items}}...{{/each}}`
    Each {
        variable: String,
        item_name: Option<String>,
        body: Vec<Node>,
    },

    /// Unless block (inverse of if): `{{#unless condition}}...{{/unless}}`
    Unless { condition: String, body: Vec<Node> },

    /// With block (scope change): `{{#with object}}...{{/with}}`
    With { variable: String, body: Vec<Node> },
}

/// Template parser.
pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given input.
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    /// Parse the template into a list of nodes.
    pub fn parse(mut self) -> Result<Vec<Node>> {
        self.parse_nodes(None)
    }

    /// Parse nodes until reaching a closing tag or end of input.
    fn parse_nodes(&mut self, end_tag: Option<&str>) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();

        while self.pos < self.input.len() {
            // Check for closing tag
            if let Some(tag) = end_tag {
                if self.peek_str(&format!("{{{{/{tag}}}}}")) {
                    break;
                }
                // Also check for {{else}} in if blocks
                if tag == "if" && self.peek_str("{{else}}") {
                    break;
                }
            }

            // Try to parse a tag
            if self.peek_str("\\{{") {
                // Escaped opening brace
                self.pos += 3; // Skip \{{
                nodes.push(Node::Text("{{".to_string()));
            } else if self.peek_str("{{") {
                let node = self.parse_tag()?;
                nodes.push(node);
            } else {
                // Collect text until next tag or escape
                let text = self.collect_text();
                if !text.is_empty() {
                    nodes.push(Node::Text(text));
                }
            }
        }

        Ok(nodes)
    }

    /// Parse a single tag.
    fn parse_tag(&mut self) -> Result<Node> {
        self.expect("{{")?;
        self.skip_whitespace();

        // Check for block tags
        if self.peek_char() == Some('#') {
            self.advance(); // Skip #
            self.skip_whitespace();
            return self.parse_block_tag();
        }

        // Check for translation
        if self.peek_str("t ") || self.peek_str("t\"") {
            return self.parse_translation();
        }

        // Parse variable
        let name = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect("}}")?;

        Ok(Node::Variable(name))
    }

    /// Parse a block tag (if, each, unless, with).
    fn parse_block_tag(&mut self) -> Result<Node> {
        let tag_name = self.parse_identifier()?;
        self.skip_whitespace();

        match tag_name.as_str() {
            "if" => self.parse_if_block(),
            "each" => self.parse_each_block(),
            "unless" => self.parse_unless_block(),
            "with" => self.parse_with_block(),
            _ => Err(TemplateError::Parse(format!(
                "Unknown block tag: {tag_name}"
            ))),
        }
    }

    /// Parse an if block.
    fn parse_if_block(&mut self) -> Result<Node> {
        let condition = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect("}}")?;

        let then_branch = self.parse_nodes(Some("if"))?;

        let else_branch = if self.peek_str("{{else}}") {
            self.expect("{{else}}")?;
            self.parse_nodes(Some("if"))?
        } else {
            Vec::new()
        };

        self.expect("{{/if}}")?;

        Ok(Node::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    /// Parse an each block.
    fn parse_each_block(&mut self) -> Result<Node> {
        let variable = self.parse_identifier()?;
        self.skip_whitespace();

        // Check for "as item" syntax
        let item_name = if self.peek_str("as ") {
            self.expect("as ")?;
            self.skip_whitespace();
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.skip_whitespace();
        self.expect("}}")?;

        let body = self.parse_nodes(Some("each"))?;
        self.expect("{{/each}}")?;

        Ok(Node::Each {
            variable,
            item_name,
            body,
        })
    }

    /// Parse an unless block.
    fn parse_unless_block(&mut self) -> Result<Node> {
        let condition = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect("}}")?;

        let body = self.parse_nodes(Some("unless"))?;
        self.expect("{{/unless}}")?;

        Ok(Node::Unless { condition, body })
    }

    /// Parse a with block.
    fn parse_with_block(&mut self) -> Result<Node> {
        let variable = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect("}}")?;

        let body = self.parse_nodes(Some("with"))?;
        self.expect("{{/with}}")?;

        Ok(Node::With { variable, body })
    }

    /// Parse a translation tag.
    fn parse_translation(&mut self) -> Result<Node> {
        self.expect("t")?;
        self.skip_whitespace();

        // Parse the key (quoted string)
        let key = self.parse_quoted_string()?;
        self.skip_whitespace();

        // Parse optional parameters
        let mut params = Vec::new();
        while self.peek_char() != Some('}') {
            let param_name = self.parse_identifier()?;
            self.skip_whitespace();
            self.expect("=")?;
            self.skip_whitespace();

            let param_value = if self.peek_char() == Some('"') {
                // Keep quotes to indicate literal string in renderer
                format!("\"{}\"", self.parse_quoted_string()?)
            } else {
                self.parse_identifier()?
            };

            params.push((param_name, param_value));
            self.skip_whitespace();
        }

        self.expect("}}")?;

        Ok(Node::Translation { key, params })
    }

    /// Parse a quoted string.
    fn parse_quoted_string(&mut self) -> Result<String> {
        self.expect("\"")?;

        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch == '"' {
                let s = self.input[start..self.pos].to_string();
                self.advance();
                return Ok(s);
            }
            if ch == '\\' && self.pos + 1 < self.input.len() {
                self.advance(); // Skip escape char
            }
            self.advance();
        }

        Err(TemplateError::Parse("Unterminated string".to_string()))
    }

    /// Parse an identifier (variable name, including dot notation).
    fn parse_identifier(&mut self) -> Result<String> {
        let start = self.pos;

        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
                self.advance();
            } else {
                break;
            }
        }

        if self.pos == start {
            return Err(TemplateError::Parse("Expected identifier".to_string()));
        }

        Ok(self.input[start..self.pos].to_string())
    }

    /// Collect text until the next tag or escape sequence.
    fn collect_text(&mut self) -> String {
        let start = self.pos;

        while self.pos < self.input.len() {
            if self.peek_str("{{") || self.peek_str("\\{{") {
                break;
            }
            self.advance();
        }

        self.input[start..self.pos].to_string()
    }

    /// Skip whitespace characters.
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current_char().is_whitespace() {
            self.advance();
        }
    }

    /// Check if the input starts with the given string at current position.
    fn peek_str(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    /// Peek at the current character.
    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Get the current character.
    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap_or('\0')
    }

    /// Advance position by one character.
    fn advance(&mut self) {
        if self.pos < self.input.len() {
            self.pos += self.current_char().len_utf8();
        }
    }

    /// Expect a specific string and consume it.
    fn expect(&mut self, s: &str) -> Result<()> {
        if self.peek_str(s) {
            self.pos += s.len();
            Ok(())
        } else {
            let found: String = self.input[self.pos..].chars().take(10).collect();
            Err(TemplateError::Parse(format!(
                "Expected '{s}' but found '{found}'"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_only() {
        let parser = Parser::new("Hello, World!");
        let nodes = parser.parse().unwrap();

        assert_eq!(nodes, vec![Node::Text("Hello, World!".to_string())]);
    }

    #[test]
    fn test_parse_variable() {
        let parser = Parser::new("Hello, {{name}}!");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![
                Node::Text("Hello, ".to_string()),
                Node::Variable("name".to_string()),
                Node::Text("!".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_variable_with_dots() {
        let parser = Parser::new("{{user.name}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(nodes, vec![Node::Variable("user.name".to_string())]);
    }

    #[test]
    fn test_parse_multiple_variables() {
        let parser = Parser::new("{{a}} and {{b}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![
                Node::Variable("a".to_string()),
                Node::Text(" and ".to_string()),
                Node::Variable("b".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_translation_simple() {
        let parser = Parser::new(r#"{{t "menu.main"}}"#);
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::Translation {
                key: "menu.main".to_string(),
                params: vec![],
            }]
        );
    }

    #[test]
    fn test_parse_translation_with_params() {
        let parser = Parser::new(r#"{{t "welcome" name="World"}}"#);
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::Translation {
                key: "welcome".to_string(),
                // Quoted strings keep quotes to distinguish from variable refs
                params: vec![("name".to_string(), "\"World\"".to_string())],
            }]
        );
    }

    #[test]
    fn test_parse_translation_with_variable_param() {
        let parser = Parser::new(r#"{{t "welcome" name=username}}"#);
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::Translation {
                key: "welcome".to_string(),
                params: vec![("name".to_string(), "username".to_string())],
            }]
        );
    }

    #[test]
    fn test_parse_if_simple() {
        let parser = Parser::new("{{#if show}}visible{{/if}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: "show".to_string(),
                then_branch: vec![Node::Text("visible".to_string())],
                else_branch: vec![],
            }]
        );
    }

    #[test]
    fn test_parse_if_else() {
        let parser = Parser::new("{{#if show}}yes{{else}}no{{/if}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: "show".to_string(),
                then_branch: vec![Node::Text("yes".to_string())],
                else_branch: vec![Node::Text("no".to_string())],
            }]
        );
    }

    #[test]
    fn test_parse_if_with_variables() {
        let parser = Parser::new("{{#if logged_in}}Hello, {{name}}{{/if}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: "logged_in".to_string(),
                then_branch: vec![
                    Node::Text("Hello, ".to_string()),
                    Node::Variable("name".to_string()),
                ],
                else_branch: vec![],
            }]
        );
    }

    #[test]
    fn test_parse_each_simple() {
        let parser = Parser::new("{{#each items}}{{name}}{{/each}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::Each {
                variable: "items".to_string(),
                item_name: None,
                body: vec![Node::Variable("name".to_string())],
            }]
        );
    }

    #[test]
    fn test_parse_each_with_as() {
        let parser = Parser::new("{{#each items as item}}{{item.name}}{{/each}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::Each {
                variable: "items".to_string(),
                item_name: Some("item".to_string()),
                body: vec![Node::Variable("item.name".to_string())],
            }]
        );
    }

    #[test]
    fn test_parse_unless() {
        let parser = Parser::new("{{#unless hidden}}shown{{/unless}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::Unless {
                condition: "hidden".to_string(),
                body: vec![Node::Text("shown".to_string())],
            }]
        );
    }

    #[test]
    fn test_parse_with() {
        let parser = Parser::new("{{#with user}}{{name}}{{/with}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::With {
                variable: "user".to_string(),
                body: vec![Node::Variable("name".to_string())],
            }]
        );
    }

    #[test]
    fn test_parse_escaped_braces() {
        let parser = Parser::new("Use \\{{name}} for variables");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![
                Node::Text("Use ".to_string()),
                Node::Text("{{".to_string()),
                Node::Text("name}} for variables".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_nested_blocks() {
        let parser = Parser::new("{{#if show}}{{#each items}}{{name}}{{/each}}{{/if}}");
        let nodes = parser.parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: "show".to_string(),
                then_branch: vec![Node::Each {
                    variable: "items".to_string(),
                    item_name: None,
                    body: vec![Node::Variable("name".to_string())],
                }],
                else_branch: vec![],
            }]
        );
    }

    #[test]
    fn test_parse_whitespace_in_tags() {
        let parser = Parser::new("{{ name }}");
        let nodes = parser.parse().unwrap();

        assert_eq!(nodes, vec![Node::Variable("name".to_string())]);
    }

    #[test]
    fn test_parse_empty() {
        let parser = Parser::new("");
        let nodes = parser.parse().unwrap();

        assert_eq!(nodes, Vec::<Node>::new());
    }

    #[test]
    fn test_parse_complex_template() {
        let template = r#"{{t "welcome.title"}}

Hello, {{username}}!

{{#if is_admin}}
Admin Menu:
{{#each admin_items as item}}
  [{{item.key}}] {{item.label}}
{{/each}}
{{else}}
User Menu:
{{/if}}"#;

        let parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        // Just verify it parses without error
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_error_unclosed_tag() {
        let parser = Parser::new("{{name");
        let result = parser.parse();

        assert!(matches!(result, Err(TemplateError::Parse(_))));
    }

    #[test]
    fn test_parse_error_unclosed_block() {
        let parser = Parser::new("{{#if show}}content");
        let result = parser.parse();

        assert!(matches!(result, Err(TemplateError::Parse(_))));
    }

    #[test]
    fn test_parse_error_unknown_block() {
        let parser = Parser::new("{{#unknown}}content{{/unknown}}");
        let result = parser.parse();

        assert!(matches!(result, Err(TemplateError::Parse(_))));
    }
}
