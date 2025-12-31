//! Logging configuration and initialization for HOBBS.

use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;

use tracing::Level;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use crate::config::LoggingConfig;
use crate::Result;

/// Parse log level string to tracing Level.
fn parse_level(level: &str) -> Level {
    match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

/// Initialize the logging system with the given configuration.
///
/// Sets up both console output and optional file logging based on the config.
pub fn init(config: &LoggingConfig) -> Result<()> {
    let level = parse_level(&config.level);
    let filter = EnvFilter::from_default_env().add_directive(level.into());

    // Ensure log directory exists
    if let Some(parent) = Path::new(&config.file).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Create log file
    let log_file = File::create(&config.file)?;
    let log_file = Arc::new(log_file);

    // Create writer that writes to both stdout and file
    let writer = std::io::stdout.and(log_file);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .with(filter)
        .init();

    Ok(())
}

/// Initialize console-only logging (for development/testing).
///
/// This is a simpler initialization that only outputs to the console.
pub fn init_console_only(level: &str) {
    let level = parse_level(level);
    let filter = EnvFilter::from_default_env().add_directive(level.into());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_target(true),
        )
        .with(filter)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_level_trace() {
        assert_eq!(parse_level("trace"), Level::TRACE);
        assert_eq!(parse_level("TRACE"), Level::TRACE);
    }

    #[test]
    fn test_parse_level_debug() {
        assert_eq!(parse_level("debug"), Level::DEBUG);
        assert_eq!(parse_level("DEBUG"), Level::DEBUG);
    }

    #[test]
    fn test_parse_level_info() {
        assert_eq!(parse_level("info"), Level::INFO);
        assert_eq!(parse_level("INFO"), Level::INFO);
    }

    #[test]
    fn test_parse_level_warn() {
        assert_eq!(parse_level("warn"), Level::WARN);
        assert_eq!(parse_level("WARN"), Level::WARN);
        assert_eq!(parse_level("warning"), Level::WARN);
    }

    #[test]
    fn test_parse_level_error() {
        assert_eq!(parse_level("error"), Level::ERROR);
        assert_eq!(parse_level("ERROR"), Level::ERROR);
    }

    #[test]
    fn test_parse_level_default() {
        assert_eq!(parse_level("invalid"), Level::INFO);
        assert_eq!(parse_level(""), Level::INFO);
    }
}
