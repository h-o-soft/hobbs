//! Validation utilities for Web API DTOs.

use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, Request},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::web::error::ApiError;

/// A JSON extractor that validates the request body.
///
/// This extractor deserializes the request body as JSON and then validates it
/// using the `validator` crate. If validation fails, it returns a detailed
/// error response with field-level error information.
///
/// # Example
///
/// ```ignore
/// use hobbs::web::dto::ValidatedJson;
///
/// async fn create_user(
///     ValidatedJson(payload): ValidatedJson<CreateUserRequest>,
/// ) -> Result<Json<User>, ApiError> {
///     // payload is already validated
///     // ...
/// }
/// ```
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // First, extract the JSON body
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ApiError::bad_request(format!("Invalid JSON: {}", e)))?;

        // Then, validate the deserialized value
        value.validate().map_err(ApiError::from_validation_errors)?;

        Ok(ValidatedJson(value))
    }
}

// ============================================================================
// Custom Validators
// ============================================================================

/// Validate that a string does not contain control characters or NULL bytes.
pub fn no_control_chars(value: &str) -> Result<(), validator::ValidationError> {
    if value
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(validator::ValidationError::new("no_control_chars")
            .with_message("Must not contain control characters".into()));
    }
    Ok(())
}

/// Validate that a string is not empty after trimming whitespace.
pub fn not_empty_trimmed(value: &str) -> Result<(), validator::ValidationError> {
    if value.trim().is_empty() {
        return Err(validator::ValidationError::new("not_empty_trimmed")
            .with_message("Must not be empty".into()));
    }
    Ok(())
}

/// Sanitize a string by removing control characters except newlines, carriage returns, and tabs.
pub fn sanitize_string(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_control_chars_valid() {
        assert!(no_control_chars("Hello, world!").is_ok());
        assert!(no_control_chars("Line 1\nLine 2").is_ok());
        assert!(no_control_chars("Tab\there").is_ok());
        assert!(no_control_chars("Return\rhere").is_ok());
    }

    #[test]
    fn test_no_control_chars_invalid() {
        assert!(no_control_chars("Hello\x00World").is_err()); // NULL byte
        assert!(no_control_chars("Hello\x07World").is_err()); // Bell
        assert!(no_control_chars("Hello\x1bWorld").is_err()); // Escape
    }

    #[test]
    fn test_not_empty_trimmed_valid() {
        assert!(not_empty_trimmed("Hello").is_ok());
        assert!(not_empty_trimmed("  Hello  ").is_ok());
    }

    #[test]
    fn test_not_empty_trimmed_invalid() {
        assert!(not_empty_trimmed("").is_err());
        assert!(not_empty_trimmed("   ").is_err());
        assert!(not_empty_trimmed("\t\n").is_err());
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("Hello"), "Hello");
        assert_eq!(sanitize_string("Hello\nWorld"), "Hello\nWorld");
        assert_eq!(sanitize_string("Hello\x00World"), "HelloWorld");
        assert_eq!(sanitize_string("Hello\x07World"), "HelloWorld");
    }
}
