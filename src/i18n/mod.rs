//! Internationalization (i18n) module for HOBBS.
//!
//! This module provides multi-language support through TOML-based language resources.
//!
//! # Usage
//!
//! ```no_run
//! use hobbs::i18n::I18n;
//!
//! // Load Japanese language resources
//! let i18n = I18n::load("ja", "locales").unwrap();
//!
//! // Simple translation
//! let text = i18n.t("menu.main");
//!
//! // Translation with parameters
//! let welcome = i18n.t_with("welcome.message", &[("name", "太郎")]);
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use thiserror::Error;

/// Default locale.
pub const DEFAULT_LOCALE: &str = "ja";

/// I18n-related errors.
#[derive(Error, Debug)]
pub enum I18nError {
    /// Failed to read locale file.
    #[error("Failed to read locale file: {0}")]
    FileRead(#[from] std::io::Error),

    /// Failed to parse TOML.
    #[error("Failed to parse locale file: {0}")]
    Parse(#[from] toml::de::Error),

    /// Locale not found.
    #[error("Locale not found: {0}")]
    LocaleNotFound(String),
}

/// Result type for i18n operations.
pub type Result<T> = std::result::Result<T, I18nError>;

/// Internationalization manager.
///
/// Loads and manages language resources from TOML files.
#[derive(Debug, Clone)]
pub struct I18n {
    /// Current locale (e.g., "ja", "en").
    locale: String,
    /// Flattened message map (key -> value).
    messages: HashMap<String, String>,
}

impl I18n {
    /// Create a new I18n instance with the given locale.
    ///
    /// # Arguments
    ///
    /// * `locale` - The locale code (e.g., "ja", "en")
    /// * `locales_dir` - Path to the directory containing locale files
    ///
    /// # Returns
    ///
    /// An I18n instance loaded with the specified locale.
    ///
    /// # Errors
    ///
    /// Returns an error if the locale file cannot be read or parsed.
    pub fn load<P: AsRef<Path>>(locale: &str, locales_dir: P) -> Result<Self> {
        let path = locales_dir.as_ref().join(format!("{locale}.toml"));

        if !path.exists() {
            return Err(I18nError::LocaleNotFound(locale.to_string()));
        }

        let content = fs::read_to_string(&path)?;
        let table: toml::Table = toml::from_str(&content)?;

        let mut messages = HashMap::new();
        flatten_toml("", &toml::Value::Table(table), &mut messages);

        Ok(Self {
            locale: locale.to_string(),
            messages,
        })
    }

    /// Create an I18n instance from a TOML string.
    ///
    /// Useful for testing or embedding resources.
    pub fn from_str(locale: &str, content: &str) -> Result<Self> {
        let table: toml::Table = toml::from_str(content)?;

        let mut messages = HashMap::new();
        flatten_toml("", &toml::Value::Table(table), &mut messages);

        Ok(Self {
            locale: locale.to_string(),
            messages,
        })
    }

    /// Create an empty I18n instance.
    ///
    /// All translations will return the key itself.
    pub fn empty(locale: &str) -> Self {
        Self {
            locale: locale.to_string(),
            messages: HashMap::new(),
        }
    }

    /// Get the current locale.
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Get all loaded message keys.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.messages.keys()
    }

    /// Get the number of loaded messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if no messages are loaded.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Translate a key to the current locale.
    ///
    /// If the key is not found, returns the key itself.
    ///
    /// # Arguments
    ///
    /// * `key` - The translation key (e.g., "menu.main", "error.not_found")
    ///
    /// # Returns
    ///
    /// The translated string, or the key if not found.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.messages.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Translate a key with parameter substitution.
    ///
    /// Parameters in the translation are marked as `{{name}}` and will be
    /// replaced with the provided values.
    ///
    /// # Arguments
    ///
    /// * `key` - The translation key
    /// * `params` - A slice of (name, value) pairs for substitution
    ///
    /// # Returns
    ///
    /// The translated string with parameters substituted.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hobbs::i18n::I18n;
    /// let i18n = I18n::from_str("ja", r#"
    /// [welcome]
    /// message = "こんにちは、{{name}}さん"
    /// "#).unwrap();
    ///
    /// let result = i18n.t_with("welcome.message", &[("name", "太郎")]);
    /// assert_eq!(result, "こんにちは、太郎さん");
    /// ```
    pub fn t_with(&self, key: &str, params: &[(&str, &str)]) -> String {
        let template = self.t(key);
        let mut result = template.to_string();

        for (name, value) in params {
            let placeholder = format!("{{{{{name}}}}}");
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Check if a translation key exists.
    pub fn has_key(&self, key: &str) -> bool {
        self.messages.contains_key(key)
    }

    /// Get a translation with a fallback.
    ///
    /// If the key is not found, returns the fallback string.
    pub fn t_or<'a>(&'a self, key: &str, fallback: &'a str) -> &'a str {
        self.messages
            .get(key)
            .map(|s| s.as_str())
            .unwrap_or(fallback)
    }

    /// Merge another I18n instance into this one.
    ///
    /// Messages from the other instance will override existing ones.
    pub fn merge(&mut self, other: &I18n) {
        for (key, value) in &other.messages {
            self.messages.insert(key.clone(), value.clone());
        }
    }
}

impl Default for I18n {
    fn default() -> Self {
        Self::empty(DEFAULT_LOCALE)
    }
}

/// Flatten a TOML value into a HashMap with dot-separated keys.
fn flatten_toml(prefix: &str, value: &toml::Value, map: &mut HashMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_toml(&new_prefix, val, map);
            }
        }
        toml::Value::String(s) => {
            map.insert(prefix.to_string(), s.clone());
        }
        toml::Value::Integer(i) => {
            map.insert(prefix.to_string(), i.to_string());
        }
        toml::Value::Float(f) => {
            map.insert(prefix.to_string(), f.to_string());
        }
        toml::Value::Boolean(b) => {
            map.insert(prefix.to_string(), b.to_string());
        }
        toml::Value::Array(_) => {
            // Arrays are not supported for translations
        }
        toml::Value::Datetime(dt) => {
            map.insert(prefix.to_string(), dt.to_string());
        }
    }
}

/// I18n manager that can hold multiple locales.
#[derive(Debug, Clone)]
pub struct I18nManager {
    /// Available locales.
    locales: HashMap<String, I18n>,
    /// Current locale.
    current: String,
}

impl I18nManager {
    /// Create a new I18nManager.
    pub fn new() -> Self {
        Self {
            locales: HashMap::new(),
            current: DEFAULT_LOCALE.to_string(),
        }
    }

    /// Load all locale files from a directory.
    ///
    /// Locale files should be named `{locale}.toml` (e.g., `ja.toml`, `en.toml`).
    pub fn load_all<P: AsRef<Path>>(locales_dir: P) -> Result<Self> {
        let mut manager = Self::new();
        let dir = locales_dir.as_ref();

        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().is_some_and(|ext| ext == "toml") {
                    if let Some(stem) = path.file_stem() {
                        let locale = stem.to_string_lossy().to_string();
                        let i18n = I18n::load(&locale, dir)?;
                        manager.locales.insert(locale, i18n);
                    }
                }
            }
        }

        Ok(manager)
    }

    /// Add a locale to the manager.
    pub fn add_locale(&mut self, i18n: I18n) {
        let locale = i18n.locale().to_string();
        self.locales.insert(locale, i18n);
    }

    /// Set the current locale.
    ///
    /// Returns true if the locale was set, false if the locale is not available.
    pub fn set_locale(&mut self, locale: &str) -> bool {
        if self.locales.contains_key(locale) {
            self.current = locale.to_string();
            true
        } else {
            false
        }
    }

    /// Get the current locale.
    pub fn current_locale(&self) -> &str {
        &self.current
    }

    /// Get the list of available locales.
    pub fn available_locales(&self) -> Vec<&str> {
        self.locales.keys().map(|s| s.as_str()).collect()
    }

    /// Get the I18n instance for the current locale.
    pub fn current(&self) -> Option<&I18n> {
        self.locales.get(&self.current)
    }

    /// Get the I18n instance for a specific locale.
    pub fn get(&self, locale: &str) -> Option<&I18n> {
        self.locales.get(locale)
    }

    /// Translate a key using the current locale.
    ///
    /// Falls back to the key itself if not found.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.current().map(|i18n| i18n.t(key)).unwrap_or(key)
    }

    /// Translate a key with parameters using the current locale.
    pub fn t_with(&self, key: &str, params: &[(&str, &str)]) -> String {
        self.current()
            .map(|i18n| i18n.t_with(key, params))
            .unwrap_or_else(|| key.to_string())
    }
}

impl Default for I18nManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_locale_file(dir: &Path, locale: &str, content: &str) {
        let path = dir.join(format!("{locale}.toml"));
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    // I18n basic tests
    #[test]
    fn test_i18n_from_str() {
        let content = r#"
[menu]
main = "メインメニュー"
board = "掲示板"
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        assert_eq!(i18n.locale(), "ja");
        assert_eq!(i18n.t("menu.main"), "メインメニュー");
        assert_eq!(i18n.t("menu.board"), "掲示板");
    }

    #[test]
    fn test_i18n_missing_key() {
        let content = r#"
[menu]
main = "メインメニュー"
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        // Missing key returns the key itself
        assert_eq!(i18n.t("menu.nonexistent"), "menu.nonexistent");
        assert_eq!(i18n.t("completely.missing"), "completely.missing");
    }

    #[test]
    fn test_i18n_nested_keys() {
        let content = r#"
[level1]
key = "level1"

[level1.level2]
key = "level2"

[level1.level2.level3]
key = "level3"
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        assert_eq!(i18n.t("level1.key"), "level1");
        assert_eq!(i18n.t("level1.level2.key"), "level2");
        assert_eq!(i18n.t("level1.level2.level3.key"), "level3");
    }

    #[test]
    fn test_i18n_t_with_params() {
        let content = r#"
[welcome]
message = "こんにちは、{{name}}さん"
count = "{{count}}件のメッセージがあります"
multiple = "{{user}}さんから{{sender}}さんへ"
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        assert_eq!(
            i18n.t_with("welcome.message", &[("name", "太郎")]),
            "こんにちは、太郎さん"
        );
        assert_eq!(
            i18n.t_with("welcome.count", &[("count", "5")]),
            "5件のメッセージがあります"
        );
        assert_eq!(
            i18n.t_with("welcome.multiple", &[("user", "Alice"), ("sender", "Bob")]),
            "AliceさんからBobさんへ"
        );
    }

    #[test]
    fn test_i18n_t_with_missing_param() {
        let content = r#"
[welcome]
message = "こんにちは、{{name}}さん"
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        // Missing parameter is left as-is
        assert_eq!(
            i18n.t_with("welcome.message", &[]),
            "こんにちは、{{name}}さん"
        );
    }

    #[test]
    fn test_i18n_empty() {
        let i18n = I18n::empty("en");

        assert_eq!(i18n.locale(), "en");
        assert!(i18n.is_empty());
        assert_eq!(i18n.t("any.key"), "any.key");
    }

    #[test]
    fn test_i18n_has_key() {
        let content = r#"
[menu]
main = "メインメニュー"
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        assert!(i18n.has_key("menu.main"));
        assert!(!i18n.has_key("menu.nonexistent"));
    }

    #[test]
    fn test_i18n_t_or() {
        let content = r#"
[menu]
main = "メインメニュー"
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        assert_eq!(i18n.t_or("menu.main", "fallback"), "メインメニュー");
        assert_eq!(i18n.t_or("menu.nonexistent", "fallback"), "fallback");
    }

    #[test]
    fn test_i18n_len_and_keys() {
        let content = r#"
[menu]
main = "メインメニュー"
board = "掲示板"
chat = "チャット"
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        assert_eq!(i18n.len(), 3);
        assert!(!i18n.is_empty());

        let keys: Vec<_> = i18n.keys().collect();
        assert!(keys.contains(&&"menu.main".to_string()));
        assert!(keys.contains(&&"menu.board".to_string()));
        assert!(keys.contains(&&"menu.chat".to_string()));
    }

    #[test]
    fn test_i18n_merge() {
        let content1 = r#"
[menu]
main = "メインメニュー"
board = "掲示板"
"#;
        let content2 = r#"
[menu]
board = "ボード"
chat = "チャット"
"#;
        let mut i18n1 = I18n::from_str("ja", content1).unwrap();
        let i18n2 = I18n::from_str("ja", content2).unwrap();

        i18n1.merge(&i18n2);

        assert_eq!(i18n1.t("menu.main"), "メインメニュー");
        assert_eq!(i18n1.t("menu.board"), "ボード"); // Overwritten
        assert_eq!(i18n1.t("menu.chat"), "チャット"); // Added
    }

    // I18n file loading tests
    #[test]
    fn test_i18n_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let content = r#"
[menu]
main = "Main Menu"
"#;
        create_test_locale_file(temp_dir.path(), "en", content);

        let i18n = I18n::load("en", temp_dir.path()).unwrap();
        assert_eq!(i18n.locale(), "en");
        assert_eq!(i18n.t("menu.main"), "Main Menu");
    }

    #[test]
    fn test_i18n_load_missing_locale() {
        let temp_dir = TempDir::new().unwrap();

        let result = I18n::load("nonexistent", temp_dir.path());
        assert!(matches!(result, Err(I18nError::LocaleNotFound(_))));
    }

    #[test]
    fn test_i18n_load_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("invalid.toml");
        fs::write(&path, "this is not valid toml [[[").unwrap();

        let result = I18n::load("invalid", temp_dir.path());
        assert!(matches!(result, Err(I18nError::Parse(_))));
    }

    // I18nManager tests
    #[test]
    fn test_i18n_manager_new() {
        let manager = I18nManager::new();

        assert_eq!(manager.current_locale(), DEFAULT_LOCALE);
        assert!(manager.available_locales().is_empty());
    }

    #[test]
    fn test_i18n_manager_add_locale() {
        let mut manager = I18nManager::new();

        let ja = I18n::from_str(
            "ja",
            r#"[menu]
main = "メインメニュー""#,
        )
        .unwrap();
        let en = I18n::from_str(
            "en",
            r#"[menu]
main = "Main Menu""#,
        )
        .unwrap();

        manager.add_locale(ja);
        manager.add_locale(en);

        assert_eq!(manager.available_locales().len(), 2);
        assert!(manager.available_locales().contains(&"ja"));
        assert!(manager.available_locales().contains(&"en"));
    }

    #[test]
    fn test_i18n_manager_set_locale() {
        let mut manager = I18nManager::new();

        let ja = I18n::from_str(
            "ja",
            r#"[menu]
main = "メインメニュー""#,
        )
        .unwrap();
        let en = I18n::from_str(
            "en",
            r#"[menu]
main = "Main Menu""#,
        )
        .unwrap();

        manager.add_locale(ja);
        manager.add_locale(en);

        assert!(manager.set_locale("ja"));
        assert_eq!(manager.current_locale(), "ja");
        assert_eq!(manager.t("menu.main"), "メインメニュー");

        assert!(manager.set_locale("en"));
        assert_eq!(manager.current_locale(), "en");
        assert_eq!(manager.t("menu.main"), "Main Menu");

        assert!(!manager.set_locale("fr")); // Not available
        assert_eq!(manager.current_locale(), "en"); // Unchanged
    }

    #[test]
    fn test_i18n_manager_load_all() {
        let temp_dir = TempDir::new().unwrap();

        create_test_locale_file(
            temp_dir.path(),
            "ja",
            r#"[menu]
main = "メインメニュー""#,
        );
        create_test_locale_file(
            temp_dir.path(),
            "en",
            r#"[menu]
main = "Main Menu""#,
        );

        let manager = I18nManager::load_all(temp_dir.path()).unwrap();

        assert_eq!(manager.available_locales().len(), 2);
        assert!(manager.available_locales().contains(&"ja"));
        assert!(manager.available_locales().contains(&"en"));
    }

    #[test]
    fn test_i18n_manager_t_with() {
        let mut manager = I18nManager::new();

        let ja = I18n::from_str(
            "ja",
            r#"[welcome]
message = "こんにちは、{{name}}さん""#,
        )
        .unwrap();

        manager.add_locale(ja);
        manager.set_locale("ja");

        assert_eq!(
            manager.t_with("welcome.message", &[("name", "太郎")]),
            "こんにちは、太郎さん"
        );
    }

    #[test]
    fn test_i18n_manager_get() {
        let mut manager = I18nManager::new();

        let ja = I18n::from_str(
            "ja",
            r#"[menu]
main = "メインメニュー""#,
        )
        .unwrap();

        manager.add_locale(ja);

        assert!(manager.get("ja").is_some());
        assert!(manager.get("en").is_none());
    }

    // I18nError display tests
    #[test]
    fn test_i18n_error_display() {
        let err = I18nError::LocaleNotFound("fr".to_string());
        assert!(err.to_string().contains("fr"));
    }

    // Default trait tests
    #[test]
    fn test_i18n_default() {
        let i18n = I18n::default();
        assert_eq!(i18n.locale(), DEFAULT_LOCALE);
        assert!(i18n.is_empty());
    }

    #[test]
    fn test_i18n_manager_default() {
        let manager = I18nManager::default();
        assert_eq!(manager.current_locale(), DEFAULT_LOCALE);
    }

    // Non-string value tests
    #[test]
    fn test_i18n_non_string_values() {
        let content = r#"
[stats]
count = 42
ratio = 3.14
enabled = true
"#;
        let i18n = I18n::from_str("ja", content).unwrap();

        assert_eq!(i18n.t("stats.count"), "42");
        assert_eq!(i18n.t("stats.ratio"), "3.14");
        assert_eq!(i18n.t("stats.enabled"), "true");
    }
}
