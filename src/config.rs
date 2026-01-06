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
    /// Read timeout in seconds for unauthenticated connections (DoS protection).
    #[serde(default = "default_read_timeout")]
    pub read_timeout_secs: u64,
    /// Read timeout in seconds for guest users.
    #[serde(default = "default_guest_timeout")]
    pub guest_timeout_secs: u64,
    /// Timezone for displaying dates (e.g., "Asia/Tokyo", "UTC").
    #[serde(default = "default_timezone")]
    pub timezone: String,
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

fn default_read_timeout() -> u64 {
    30
}

fn default_guest_timeout() -> u64 {
    120
}

fn default_timezone() -> String {
    "Asia/Tokyo".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            max_connections: default_max_connections(),
            idle_timeout_secs: default_idle_timeout(),
            read_timeout_secs: default_read_timeout(),
            guest_timeout_secs: default_guest_timeout(),
            timezone: default_timezone(),
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

/// Terminal configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TerminalConfig {
    /// Default terminal profile (standard, c64, c64_ansi).
    #[serde(default = "default_terminal_profile")]
    pub default_profile: String,
    /// Enable auto-paging for terminals without scroll capability.
    #[serde(default = "default_auto_paging")]
    pub auto_paging: bool,
    /// Lines before auto-pause (0 = auto-calculate from terminal height - 4).
    #[serde(default)]
    pub paging_lines: usize,
}

fn default_terminal_profile() -> String {
    "standard".to_string()
}

fn default_auto_paging() -> bool {
    true
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            default_profile: default_terminal_profile(),
            auto_paging: default_auto_paging(),
            paging_lines: 0,
        }
    }
}

/// RSS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RssConfig {
    /// Whether RSS feature is enabled.
    #[serde(default = "default_rss_enabled")]
    pub enabled: bool,
    /// Update check interval in seconds.
    #[serde(default = "default_rss_update_interval")]
    pub update_interval_secs: u64,
    /// Default fetch interval per feed in seconds.
    #[serde(default = "default_rss_fetch_interval")]
    pub default_fetch_interval_secs: u64,
    /// Maximum feed size in bytes.
    #[serde(default = "default_rss_max_feed_size")]
    pub max_feed_size_bytes: u64,
    /// Maximum items per feed.
    #[serde(default = "default_rss_max_items")]
    pub max_items_per_feed: usize,
    /// Maximum description/content length in characters.
    #[serde(default = "default_rss_max_content_length")]
    pub max_content_length: usize,
    /// Connection timeout in seconds.
    #[serde(default = "default_rss_connect_timeout")]
    pub connect_timeout_secs: u64,
    /// Read timeout in seconds.
    #[serde(default = "default_rss_read_timeout")]
    pub read_timeout_secs: u64,
    /// Total request timeout in seconds.
    #[serde(default = "default_rss_total_timeout")]
    pub total_timeout_secs: u64,
    /// Maximum number of redirects.
    #[serde(default = "default_rss_max_redirects")]
    pub max_redirects: usize,
    /// Maximum consecutive errors before disabling feed.
    #[serde(default = "default_rss_max_errors")]
    pub max_consecutive_errors: i32,
}

fn default_rss_enabled() -> bool {
    true
}

fn default_rss_update_interval() -> u64 {
    300 // 5 minutes
}

fn default_rss_fetch_interval() -> u64 {
    3600 // 1 hour
}

fn default_rss_max_feed_size() -> u64 {
    5 * 1024 * 1024 // 5MB
}

fn default_rss_max_items() -> usize {
    100
}

fn default_rss_max_content_length() -> usize {
    10000
}

fn default_rss_connect_timeout() -> u64 {
    10
}

fn default_rss_read_timeout() -> u64 {
    20
}

fn default_rss_total_timeout() -> u64 {
    30
}

fn default_rss_max_redirects() -> usize {
    5
}

fn default_rss_max_errors() -> i32 {
    5
}

impl Default for RssConfig {
    fn default() -> Self {
        Self {
            enabled: default_rss_enabled(),
            update_interval_secs: default_rss_update_interval(),
            default_fetch_interval_secs: default_rss_fetch_interval(),
            max_feed_size_bytes: default_rss_max_feed_size(),
            max_items_per_feed: default_rss_max_items(),
            max_content_length: default_rss_max_content_length(),
            connect_timeout_secs: default_rss_connect_timeout(),
            read_timeout_secs: default_rss_read_timeout(),
            total_timeout_secs: default_rss_total_timeout(),
            max_redirects: default_rss_max_redirects(),
            max_consecutive_errors: default_rss_max_errors(),
        }
    }
}

/// Web UI configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WebConfig {
    /// Whether Web UI is enabled.
    #[serde(default = "default_web_enabled")]
    pub enabled: bool,
    /// Host address to bind.
    #[serde(default = "default_web_host")]
    pub host: String,
    /// Port number for Web API.
    #[serde(default = "default_web_port")]
    pub port: u16,
    /// CORS allowed origins.
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// JWT secret key (must be set if enabled).
    #[serde(default)]
    pub jwt_secret: String,
    /// Access token expiry in seconds.
    #[serde(default = "default_jwt_access_expiry")]
    pub jwt_access_token_expiry_secs: u64,
    /// Refresh token expiry in days.
    #[serde(default = "default_jwt_refresh_expiry")]
    pub jwt_refresh_token_expiry_days: u64,
    /// Whether to serve static files.
    #[serde(default)]
    pub serve_static: bool,
    /// Path to static files directory.
    #[serde(default = "default_static_path")]
    pub static_path: String,
    /// Rate limit for login endpoint (requests per minute).
    #[serde(default = "default_login_rate_limit")]
    pub login_rate_limit: u32,
    /// Rate limit for general API endpoints (requests per minute).
    #[serde(default = "default_api_rate_limit")]
    pub api_rate_limit: u32,
}

fn default_web_enabled() -> bool {
    false
}

fn default_web_host() -> String {
    "0.0.0.0".to_string()
}

fn default_web_port() -> u16 {
    8080
}

fn default_jwt_access_expiry() -> u64 {
    900 // 15 minutes
}

fn default_jwt_refresh_expiry() -> u64 {
    7 // 7 days
}

fn default_static_path() -> String {
    "web/dist".to_string()
}

fn default_login_rate_limit() -> u32 {
    5 // 5 requests per minute
}

fn default_api_rate_limit() -> u32 {
    100 // 100 requests per minute
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: default_web_enabled(),
            host: default_web_host(),
            port: default_web_port(),
            cors_origins: vec![],
            jwt_secret: String::new(),
            jwt_access_token_expiry_secs: default_jwt_access_expiry(),
            jwt_refresh_token_expiry_days: default_jwt_refresh_expiry(),
            serve_static: false,
            static_path: default_static_path(),
            login_rate_limit: default_login_rate_limit(),
            api_rate_limit: default_api_rate_limit(),
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
    /// Terminal configuration.
    #[serde(default)]
    pub terminal: TerminalConfig,
    /// RSS configuration.
    #[serde(default)]
    pub rss: RssConfig,
    /// Web UI configuration.
    #[serde(default)]
    pub web: WebConfig,
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(HobbsError::Io)?;
        Self::parse(&content)
    }

    /// Load configuration from a TOML file and apply environment variable overrides.
    pub fn load_with_env<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = Self::load(path)?;
        config.apply_env_overrides();
        Ok(config)
    }

    /// Parse configuration from a TOML string.
    pub fn parse(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| HobbsError::Validation(format!("config parse error: {e}")))
    }

    /// Apply environment variable overrides to the configuration.
    ///
    /// Supported environment variables:
    /// - `HOBBS_JWT_SECRET`: Override the JWT secret key
    pub fn apply_env_overrides(&mut self) {
        // JWT secret from environment variable (highest priority)
        if let Ok(jwt_secret) = std::env::var("HOBBS_JWT_SECRET") {
            if !jwt_secret.is_empty() {
                self.web.jwt_secret = jwt_secret;
            }
        }
    }

    /// Validate the configuration.
    ///
    /// Returns an error if:
    /// - Web UI is enabled but JWT secret is not set
    pub fn validate(&self) -> Result<()> {
        if self.web.enabled && self.web.jwt_secret.is_empty() {
            return Err(HobbsError::Validation(
                "Web UI is enabled but jwt_secret is not set. \
                 Set it in config.toml or via HOBBS_JWT_SECRET environment variable."
                    .to_string(),
            ));
        }
        Ok(())
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
        assert_eq!(config.server.read_timeout_secs, 30);
        assert_eq!(config.server.guest_timeout_secs, 120);
        assert_eq!(config.server.timezone, "Asia/Tokyo");

        assert_eq!(config.database.path, "data/hobbs.db");

        assert_eq!(config.files.storage_path, "data/files");
        assert_eq!(config.files.max_upload_size_mb, 10);

        assert_eq!(config.bbs.name, "HOBBS - Hobbyist BBS");
        assert_eq!(config.bbs.sysop_name, "SysOp");

        assert_eq!(config.locale.language, "ja");

        assert_eq!(config.templates.path, "templates");

        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.file, "logs/hobbs.log");

        assert_eq!(config.terminal.default_profile, "standard");
        assert!(config.terminal.auto_paging);
        assert_eq!(config.terminal.paging_lines, 0);

        assert!(config.rss.enabled);
        assert_eq!(config.rss.update_interval_secs, 300);
        assert_eq!(config.rss.default_fetch_interval_secs, 3600);
        assert_eq!(config.rss.max_feed_size_bytes, 5 * 1024 * 1024);
        assert_eq!(config.rss.max_items_per_feed, 100);
        assert_eq!(config.rss.max_content_length, 10000);
        assert_eq!(config.rss.connect_timeout_secs, 10);
        assert_eq!(config.rss.read_timeout_secs, 20);
        assert_eq!(config.rss.total_timeout_secs, 30);
        assert_eq!(config.rss.max_redirects, 5);
        assert_eq!(config.rss.max_consecutive_errors, 5);

        assert!(!config.web.enabled);
        assert_eq!(config.web.host, "0.0.0.0");
        assert_eq!(config.web.port, 8080);
        assert!(config.web.cors_origins.is_empty());
        assert!(config.web.jwt_secret.is_empty());
        assert_eq!(config.web.jwt_access_token_expiry_secs, 900);
        assert_eq!(config.web.jwt_refresh_token_expiry_days, 7);
        assert!(!config.web.serve_static);
        assert_eq!(config.web.static_path, "web/dist");
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

[terminal]
default_profile = "c64"
auto_paging = false
paging_lines = 15

[rss]
enabled = false
update_interval_secs = 600
default_fetch_interval_secs = 1800
max_feed_size_bytes = 10485760
max_items_per_feed = 50
max_content_length = 5000
connect_timeout_secs = 15
read_timeout_secs = 25
total_timeout_secs = 45
max_redirects = 3
max_consecutive_errors = 3

[web]
enabled = true
host = "127.0.0.1"
port = 3000
cors_origins = ["http://localhost:3000", "http://localhost:5173"]
jwt_secret = "test-secret-key"
jwt_access_token_expiry_secs = 600
jwt_refresh_token_expiry_days = 14
serve_static = true
static_path = "public"
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

        assert_eq!(config.terminal.default_profile, "c64");

        assert!(!config.rss.enabled);
        assert_eq!(config.rss.update_interval_secs, 600);
        assert_eq!(config.rss.default_fetch_interval_secs, 1800);
        assert_eq!(config.rss.max_feed_size_bytes, 10485760);
        assert_eq!(config.rss.max_items_per_feed, 50);
        assert_eq!(config.rss.max_content_length, 5000);
        assert_eq!(config.rss.connect_timeout_secs, 15);
        assert_eq!(config.rss.read_timeout_secs, 25);
        assert_eq!(config.rss.total_timeout_secs, 45);
        assert_eq!(config.rss.max_redirects, 3);
        assert_eq!(config.rss.max_consecutive_errors, 3);

        assert!(config.web.enabled);
        assert_eq!(config.web.host, "127.0.0.1");
        assert_eq!(config.web.port, 3000);
        assert_eq!(config.web.cors_origins.len(), 2);
        assert_eq!(config.web.cors_origins[0], "http://localhost:3000");
        assert_eq!(config.web.cors_origins[1], "http://localhost:5173");
        assert_eq!(config.web.jwt_secret, "test-secret-key");
        assert_eq!(config.web.jwt_access_token_expiry_secs, 600);
        assert_eq!(config.web.jwt_refresh_token_expiry_days, 14);
        assert!(config.web.serve_static);
        assert_eq!(config.web.static_path, "public");
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

    #[test]
    fn test_apply_env_overrides_jwt_secret() {
        // Save original value if exists
        let original = std::env::var("HOBBS_JWT_SECRET").ok();

        // Set env var
        std::env::set_var("HOBBS_JWT_SECRET", "env-secret-key");

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(config.web.jwt_secret, "env-secret-key");

        // Restore original
        if let Some(val) = original {
            std::env::set_var("HOBBS_JWT_SECRET", val);
        } else {
            std::env::remove_var("HOBBS_JWT_SECRET");
        }
    }

    #[test]
    fn test_apply_env_overrides_empty_value() {
        // Save original value if exists
        let original = std::env::var("HOBBS_JWT_SECRET").ok();

        // Set empty env var
        std::env::set_var("HOBBS_JWT_SECRET", "");

        let mut config = Config::default();
        config.web.jwt_secret = "original-secret".to_string();
        config.apply_env_overrides();

        // Should not override with empty string
        assert_eq!(config.web.jwt_secret, "original-secret");

        // Restore original
        if let Some(val) = original {
            std::env::set_var("HOBBS_JWT_SECRET", val);
        } else {
            std::env::remove_var("HOBBS_JWT_SECRET");
        }
    }

    #[test]
    fn test_validate_web_enabled_no_secret() {
        let mut config = Config::default();
        config.web.enabled = true;
        config.web.jwt_secret = String::new();

        let result = config.validate();
        assert!(result.is_err());
        if let Err(HobbsError::Validation(msg)) = result {
            assert!(msg.contains("jwt_secret"));
        }
    }

    #[test]
    fn test_validate_web_enabled_with_secret() {
        let mut config = Config::default();
        config.web.enabled = true;
        config.web.jwt_secret = "secret".to_string();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_web_disabled() {
        let config = Config::default();
        // web.enabled is false by default, no secret needed
        assert!(config.validate().is_ok());
    }
}
