//! Error types for HOBBS.

use thiserror::Error;

/// Common error type for HOBBS.
#[derive(Error, Debug)]
pub enum HobbsError {
    /// Database error.
    ///
    /// This is a generic database error that wraps errors from any database backend.
    /// Database errors from sqlx are automatically converted.
    #[error("database error: {0}")]
    Database(String),

    /// Database connection error.
    #[error("database connection error: {0}")]
    DatabaseConnection(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Authentication error.
    #[error("authentication error: {0}")]
    Auth(String),

    /// Permission denied error.
    #[error("permission denied: {0}")]
    Permission(String),

    /// Validation error for user input.
    #[error("validation error: {0}")]
    Validation(String),

    /// Resource not found.
    #[error("{0} not found")]
    NotFound(String),

    /// Template error.
    #[error("template error: {0}")]
    Template(#[from] crate::template::TemplateError),

    /// Script execution error.
    #[error("script error: {0}")]
    Script(String),

    /// RSS feed error.
    #[error("RSS error: {0}")]
    Rss(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),
}

// Conversion from sqlx errors
impl From<sqlx::Error> for HobbsError {
    fn from(e: sqlx::Error) -> Self {
        HobbsError::Database(e.to_string())
    }
}

/// Result type alias for HOBBS operations.
pub type Result<T> = std::result::Result<T, HobbsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        let err = HobbsError::Auth("invalid password".to_string());
        assert_eq!(err.to_string(), "authentication error: invalid password");
    }

    #[test]
    fn test_permission_error_display() {
        let err = HobbsError::Permission("admin access required".to_string());
        assert_eq!(err.to_string(), "permission denied: admin access required");
    }

    #[test]
    fn test_validation_error_display() {
        let err = HobbsError::Validation("username too long".to_string());
        assert_eq!(err.to_string(), "validation error: username too long");
    }

    #[test]
    fn test_not_found_error_display() {
        let err = HobbsError::NotFound("user".to_string());
        assert_eq!(err.to_string(), "user not found");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: HobbsError = io_err.into();
        assert!(matches!(err, HobbsError::Io(_)));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_result_alias() {
        fn sample_ok() -> Result<i32> {
            Ok(42)
        }

        fn sample_err() -> Result<i32> {
            Err(HobbsError::Auth("test".to_string()))
        }

        assert_eq!(sample_ok().unwrap(), 42);
        assert!(sample_err().is_err());
    }

    #[test]
    fn test_rss_error_display() {
        let err = HobbsError::Rss("feed parsing failed".to_string());
        assert_eq!(err.to_string(), "RSS error: feed parsing failed");
    }
}
