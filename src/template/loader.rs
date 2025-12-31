//! Template loader module.
//!
//! Provides automatic template loading based on terminal width.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{Result, TemplateContext, TemplateEngine, TemplateError};
use crate::i18n::I18n;

/// Default template directory for 80-column terminals.
pub const WIDTH_80: u16 = 80;

/// Default template directory for 40-column terminals.
pub const WIDTH_40: u16 = 40;

/// Template loader with width-based selection.
#[derive(Debug)]
pub struct TemplateLoader {
    /// Base path for templates.
    base_path: PathBuf,
}

impl TemplateLoader {
    /// Create a new template loader.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Base directory containing template folders (80/, 40/).
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Get the template directory for a given width.
    fn get_width_dir(&self, width: u16) -> PathBuf {
        let dir_name = if width >= WIDTH_80 { "80" } else { "40" };
        self.base_path.join(dir_name)
    }

    /// Get the full path to a template file.
    fn get_template_path(&self, name: &str, width: u16) -> PathBuf {
        let width_dir = self.get_width_dir(width);
        width_dir.join(format!("{name}.txt"))
    }

    /// Load a template for the given width.
    ///
    /// # Arguments
    ///
    /// * `name` - Template name (e.g., "welcome", "board/list")
    /// * `width` - Terminal width in columns
    ///
    /// # Returns
    ///
    /// The template content as a string.
    pub fn load(&self, name: &str, width: u16) -> Result<String> {
        let path = self.get_template_path(name, width);

        if path.exists() {
            fs::read_to_string(&path).map_err(|e| {
                TemplateError::Render(format!("Failed to read template '{name}': {e}"))
            })
        } else {
            Err(TemplateError::NotFound(format!(
                "Template '{name}' not found at {path:?}"
            )))
        }
    }

    /// Load a template with fallback.
    ///
    /// First tries to load the template for the specified width,
    /// then falls back to the other width if not found.
    ///
    /// # Arguments
    ///
    /// * `name` - Template name
    /// * `width` - Terminal width in columns
    pub fn load_with_fallback(&self, name: &str, width: u16) -> Result<String> {
        // Try primary width first
        match self.load(name, width) {
            Ok(content) => Ok(content),
            Err(_) => {
                // Fallback to other width
                let fallback_width = if width >= WIDTH_80 { WIDTH_40 } else { WIDTH_80 };
                self.load(name, fallback_width)
            }
        }
    }

    /// Render a template for the given width.
    ///
    /// # Arguments
    ///
    /// * `name` - Template name
    /// * `width` - Terminal width in columns
    /// * `context` - Template context with variables
    pub fn render(&self, name: &str, width: u16, context: &TemplateContext) -> Result<String> {
        let content = self.load_with_fallback(name, width)?;
        TemplateEngine::render_string(&content, context)
    }

    /// List available templates for a given width.
    pub fn list_templates(&self, width: u16) -> Result<Vec<String>> {
        let width_dir = self.get_width_dir(width);

        if !width_dir.exists() {
            return Ok(Vec::new());
        }

        let mut templates = Vec::new();
        self.collect_templates(&width_dir, "", &mut templates)?;
        templates.sort();
        Ok(templates)
    }

    /// Recursively collect template names from a directory.
    fn collect_templates(
        &self,
        dir: &Path,
        prefix: &str,
        templates: &mut Vec<String>,
    ) -> Result<()> {
        collect_templates_recursive(dir, prefix, templates)
    }
}

/// Recursively collect template names from a directory.
fn collect_templates_recursive(
    dir: &Path,
    prefix: &str,
    templates: &mut Vec<String>,
) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|e| {
        TemplateError::Render(format!("Failed to read directory {dir:?}: {e}"))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            TemplateError::Render(format!("Failed to read entry: {e}"))
        })?;
        let path = entry.path();

        if path.is_dir() {
            let name = path.file_name().unwrap().to_string_lossy();
            let new_prefix = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}/{name}")
            };
            collect_templates_recursive(&path, &new_prefix, templates)?;
        } else if path.extension().is_some_and(|ext| ext == "txt") {
            let name = path.file_stem().unwrap().to_string_lossy();
            let template_name = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}/{name}")
            };
            templates.push(template_name);
        }
    }

    Ok(())
}

impl TemplateLoader {
    /// Check if a template exists for the given width.
    pub fn has_template(&self, name: &str, width: u16) -> bool {
        self.get_template_path(name, width).exists()
    }

    /// Check if a template exists for any width.
    pub fn has_template_any(&self, name: &str) -> bool {
        self.has_template(name, WIDTH_80) || self.has_template(name, WIDTH_40)
    }

    /// Get the base path.
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

/// Create a template context with common system variables.
pub fn create_system_context(i18n: Arc<I18n>) -> TemplateContext {
    use super::Value;
    use chrono::Local;

    let mut context = TemplateContext::new(i18n);
    let now = Local::now();

    // System variables
    context.set("system.date", Value::String(now.format("%Y/%m/%d").to_string()));
    context.set("system.time", Value::String(now.format("%H:%M:%S").to_string()));
    context.set(
        "system.datetime",
        Value::String(now.format("%Y/%m/%d %H:%M:%S").to_string()),
    );

    context
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_templates(dir: &Path) {
        // Create 80-column templates
        let dir_80 = dir.join("80");
        fs::create_dir_all(&dir_80).unwrap();
        fs::write(dir_80.join("welcome.txt"), "Welcome to HOBBS (80 col)!\n").unwrap();
        fs::write(dir_80.join("main_menu.txt"), "Main Menu (80 col)\n").unwrap();

        // Create subdirectory
        let board_dir = dir_80.join("board");
        fs::create_dir_all(&board_dir).unwrap();
        fs::write(board_dir.join("list.txt"), "Board List (80 col)\n").unwrap();

        // Create 40-column templates
        let dir_40 = dir.join("40");
        fs::create_dir_all(&dir_40).unwrap();
        fs::write(dir_40.join("welcome.txt"), "Welcome (40)!\n").unwrap();

        // Create subdirectory
        let board_dir_40 = dir_40.join("board");
        fs::create_dir_all(&board_dir_40).unwrap();
        fs::write(board_dir_40.join("list.txt"), "Board (40)\n").unwrap();
    }

    fn create_template_with_vars(dir: &Path) {
        let dir_80 = dir.join("80");
        fs::create_dir_all(&dir_80).unwrap();
        fs::write(
            dir_80.join("greeting.txt"),
            "Hello, {{user.name}}!\nYou have {{user.unread_mail}} unread messages.\n",
        )
        .unwrap();
    }

    #[test]
    fn test_loader_new() {
        let loader = TemplateLoader::new("/tmp/templates");
        assert_eq!(loader.base_path(), Path::new("/tmp/templates"));
    }

    #[test]
    fn test_load_80_column() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        let content = loader.load("welcome", 80).unwrap();
        assert_eq!(content, "Welcome to HOBBS (80 col)!\n");
    }

    #[test]
    fn test_load_40_column() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        let content = loader.load("welcome", 40).unwrap();
        assert_eq!(content, "Welcome (40)!\n");
    }

    #[test]
    fn test_load_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        let content = loader.load("board/list", 80).unwrap();
        assert_eq!(content, "Board List (80 col)\n");
    }

    #[test]
    fn test_load_not_found() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        let result = loader.load("nonexistent", 80);
        assert!(matches!(result, Err(TemplateError::NotFound(_))));
    }

    #[test]
    fn test_load_with_fallback_primary() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        // Should load 80-column version
        let content = loader.load_with_fallback("welcome", 80).unwrap();
        assert_eq!(content, "Welcome to HOBBS (80 col)!\n");
    }

    #[test]
    fn test_load_with_fallback_to_other_width() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        // main_menu only exists in 80-column, should fallback
        let content = loader.load_with_fallback("main_menu", 40).unwrap();
        assert_eq!(content, "Main Menu (80 col)\n");
    }

    #[test]
    fn test_load_with_fallback_not_found() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        let result = loader.load_with_fallback("nonexistent", 80);
        assert!(matches!(result, Err(TemplateError::NotFound(_))));
    }

    #[test]
    fn test_render_with_variables() {
        let temp_dir = TempDir::new().unwrap();
        create_template_with_vars(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        let i18n = Arc::new(I18n::empty("ja"));
        let mut context = TemplateContext::new(i18n);
        context.set("user.name", super::super::Value::String("たろう".to_string()));
        context.set("user.unread_mail", super::super::Value::Number(5));

        let result = loader.render("greeting", 80, &context).unwrap();
        assert_eq!(result, "Hello, たろう!\nYou have 5 unread messages.\n");
    }

    #[test]
    fn test_list_templates() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        let templates = loader.list_templates(80).unwrap();
        assert!(templates.contains(&"welcome".to_string()));
        assert!(templates.contains(&"main_menu".to_string()));
        assert!(templates.contains(&"board/list".to_string()));
    }

    #[test]
    fn test_has_template() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        assert!(loader.has_template("welcome", 80));
        assert!(loader.has_template("welcome", 40));
        assert!(!loader.has_template("main_menu", 40));
        assert!(!loader.has_template("nonexistent", 80));
    }

    #[test]
    fn test_has_template_any() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        assert!(loader.has_template_any("welcome"));
        assert!(loader.has_template_any("main_menu")); // only in 80
        assert!(!loader.has_template_any("nonexistent"));
    }

    #[test]
    fn test_width_selection() {
        let temp_dir = TempDir::new().unwrap();
        create_test_templates(temp_dir.path());

        let loader = TemplateLoader::new(temp_dir.path());

        // Width >= 80 should use 80-column directory
        assert!(loader.load("welcome", 80).unwrap().contains("80 col"));
        assert!(loader.load("welcome", 100).unwrap().contains("80 col"));
        assert!(loader.load("welcome", 132).unwrap().contains("80 col"));

        // Width < 80 should use 40-column directory
        assert!(loader.load("welcome", 40).unwrap().contains("40"));
        assert!(loader.load("welcome", 60).unwrap().contains("40"));
    }

    #[test]
    fn test_create_system_context() {
        let i18n = Arc::new(I18n::empty("ja"));
        let context = create_system_context(i18n);

        // System variables should be set
        assert!(context.get("system.date").is_some());
        assert!(context.get("system.time").is_some());
        assert!(context.get("system.datetime").is_some());
    }
}
