//! Configuration module for HOBBS.

use serde::Deserialize;
use std::path::Path;

use crate::{HobbsError, Result};

/// Server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Host address to bind.
    #[serde(default = "default_host")]
    pub host: String,
    /// Port number to listen on.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Maximum number of concurrent connections.
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Idle timeout in seconds.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    2323
}

fn default_max_connections() -> usize {
    20
}

fn default_idle_timeout() -> u64 {
    300
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            max_connections: default_max_connections(),
            idle_timeout_secs: default_idle_timeout(),
        }
    }
}

/// Database configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file.
    #[serde(default = "default_db_path")]
    pub path: String,
}

fn default_db_path() -> String {
    "data/hobbs.db".to_string()
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
        }
    }
}

/// File storage configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct FilesConfig {
    /// Path to the file storage directory.
    #[serde(default = "default_storage_path")]
    pub storage_path: String,
    /// Maximum upload size in megabytes.
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size_mb: u64,
}

fn default_storage_path() -> String {
    "data/files".to_string()
}

fn default_max_upload_size() -> u64 {
    10
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            storage_path: default_storage_path(),
            max_upload_size_mb: default_max_upload_size(),
        }
    }
}

/// BBS information configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BbsConfig {
    /// Name of the BBS.
    #[serde(default = "default_bbs_name")]
    pub name: String,
    /// Description of the BBS.
    #[serde(default = "default_bbs_description")]
    pub description: String,
    /// Name of the system operator.
    #[serde(default = "default_sysop_name")]
    pub sysop_name: String,
}

fn default_bbs_name() -> String {
    "HOBBS - Hobbyist BBS".to_string()
}

fn default_bbs_description() -> String {
    "A retro BBS system".to_string()
}

fn default_sysop_name() -> String {
    "SysOp".to_string()
}

impl Default for BbsConfig {
    fn default() -> Self {
        Self {
            name: default_bbs_name(),
            description: default_bbs_description(),
            sysop_name: default_sysop_name(),
        }
    }
}

/// Locale configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LocaleConfig {
    /// Language code (ja / en).
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "ja".to_string()
}

impl Default for LocaleConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
        }
    }
}

/// Templates configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TemplatesConfig {
    /// Path to the templates directory.
    #[serde(default = "default_templates_path")]
    pub path: String,
}

fn default_templates_path() -> String {
    "templates".to_string()
}

impl Default for TemplatesConfig {
    fn default() -> Self {
        Self {
            path: default_templates_path(),
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error).
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Path to the log file.
    #[serde(default = "default_log_file")]
    pub file: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_file() -> String {
    "logs/hobbs.log".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: default_log_file(),
        }
    }
}

/// Main configuration structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    /// Server configuration.
    #[serde(default)]
    pub server: ServerConfig,
    /// Database configuration.
    #[serde(default)]
    pub database: DatabaseConfig,
    /// File storage configuration.
    #[serde(default)]
    pub files: FilesConfig,
    /// BBS information.
    #[serde(default)]
    pub bbs: BbsConfig,
    /// Locale configuration.
    #[serde(default)]
    pub locale: LocaleConfig,
    /// Templates configuration.
    #[serde(default)]
    pub templates: TemplatesConfig,
    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(HobbsError::Io)?;
        Self::parse(&content)
    }

    /// Parse configuration from a TOML string.
    pub fn parse(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| HobbsError::Validation(format!("config parse error: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 2323);
        assert_eq!(config.server.max_connections, 20);
        assert_eq!(config.server.idle_timeout_secs, 300);

        assert_eq!(config.database.path, "data/hobbs.db");

        assert_eq!(config.files.storage_path, "data/files");
        assert_eq!(config.files.max_upload_size_mb, 10);

        assert_eq!(config.bbs.name, "HOBBS - Hobbyist BBS");
        assert_eq!(config.bbs.sysop_name, "SysOp");

        assert_eq!(config.locale.language, "ja");

        assert_eq!(config.templates.path, "templates");

        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.file, "logs/hobbs.log");
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[server]
host = "127.0.0.1"
port = 8080
max_connections = 50
idle_timeout_secs = 600

[database]
path = "custom/db.sqlite"

[files]
storage_path = "custom/files"
max_upload_size_mb = 20

[bbs]
name = "My BBS"
description = "A custom BBS"
sysop_name = "Admin"

[locale]
language = "en"

[templates]
path = "custom/templates"

[logging]
level = "debug"
file = "custom/logs/app.log"
"#;

        let config = Config::parse(toml).unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.max_connections, 50);
        assert_eq!(config.server.idle_timeout_secs, 600);

        assert_eq!(config.database.path, "custom/db.sqlite");

        assert_eq!(config.files.storage_path, "custom/files");
        assert_eq!(config.files.max_upload_size_mb, 20);

        assert_eq!(config.bbs.name, "My BBS");
        assert_eq!(config.bbs.description, "A custom BBS");
        assert_eq!(config.bbs.sysop_name, "Admin");

        assert_eq!(config.locale.language, "en");

        assert_eq!(config.templates.path, "custom/templates");

        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.logging.file, "custom/logs/app.log");
    }

    #[test]
    fn test_parse_partial_config() {
        let toml = r#"
[server]
port = 3000

[bbs]
name = "Partial BBS"
"#;

        let config = Config::parse(toml).unwrap();

        // Specified values
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.bbs.name, "Partial BBS");

        // Default values
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.max_connections, 20);
        assert_eq!(config.database.path, "data/hobbs.db");
        assert_eq!(config.locale.language, "ja");
    }

    #[test]
    fn test_parse_empty_config() {
        let toml = "";
        let config = Config::parse(toml).unwrap();

        // All defaults
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 2323);
        assert_eq!(config.database.path, "data/hobbs.db");
    }

    #[test]
    fn test_parse_invalid_config() {
        let toml = "this is not valid toml [[[";
        let result = Config::parse(toml);

        assert!(result.is_err());
        if let Err(HobbsError::Validation(msg)) = result {
            assert!(msg.contains("config parse error"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = Config::load("nonexistent.toml");

        assert!(result.is_err());
        assert!(matches!(result, Err(HobbsError::Io(_))));
    }
}
