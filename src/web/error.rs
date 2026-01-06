//! API error handling for HOBBS Web UI.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::HashMap;

/// API error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// Bad request (400).
    BadRequest,
    /// Unauthorized (401).
    Unauthorized,
    /// Forbidden (403).
    Forbidden,
    /// Not found (404).
    NotFound,
    /// Conflict (409).
    Conflict,
    /// Validation error (422) - for field-level validation errors.
    ValidationError,
    /// Unprocessable entity (422).
    UnprocessableEntity,
    /// Internal server error (500).
    InternalError,
}

impl ErrorCode {
    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict => StatusCode::CONFLICT,
            ErrorCode::ValidationError => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// API error response body.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// Error details.
    pub error: ErrorDetail,
}

/// Error detail.
#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    /// Error code.
    pub code: ErrorCode,
    /// Human-readable message.
    pub message: String,
    /// Field-level validation error details (only present for validation errors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, Vec<String>>>,
}

/// API error type.
#[derive(Debug)]
pub struct ApiError {
    code: ErrorCode,
    message: String,
    details: Option<HashMap<String, Vec<String>>>,
}

impl ApiError {
    /// Create a new API error.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Create a new API error with field-level details.
    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    /// Create a bad request error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    /// Create an unauthorized error.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    /// Create a forbidden error.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    /// Create a not found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    /// Create a conflict error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message)
    }

    /// Create an unprocessable entity error.
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::UnprocessableEntity, message)
    }

    /// Create an internal server error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// Create a validation error with field-level details.
    pub fn validation(details: HashMap<String, Vec<String>>) -> Self {
        Self::with_details(ErrorCode::ValidationError, "Validation failed", details)
    }

    /// Create a validation error from validator::ValidationErrors.
    pub fn from_validation_errors(errors: validator::ValidationErrors) -> Self {
        let mut details: HashMap<String, Vec<String>> = HashMap::new();

        for (field, field_errors) in errors.field_errors() {
            let messages: Vec<String> = field_errors
                .iter()
                .map(|e| {
                    e.message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| format!("Invalid value for {}", field))
                })
                .collect();
            details.insert(field.to_string(), messages);
        }

        Self::validation(details)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.code.status_code();
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code,
                message: self.message,
                details: self.details,
            },
        };
        (status, Json(body)).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl From<crate::HobbsError> for ApiError {
    fn from(err: crate::HobbsError) -> Self {
        match &err {
            crate::HobbsError::Auth(msg) => ApiError::unauthorized(msg.clone()),
            crate::HobbsError::NotFound(msg) => ApiError::not_found(msg.clone()),
            crate::HobbsError::Validation(msg) => ApiError::unprocessable(msg.clone()),
            crate::HobbsError::Permission(msg) => ApiError::forbidden(msg.clone()),
            _ => {
                tracing::error!("Internal error: {}", err);
                ApiError::internal("An internal error occurred")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_status() {
        assert_eq!(ErrorCode::BadRequest.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(
            ErrorCode::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(ErrorCode::Forbidden.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ErrorCode::NotFound.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(ErrorCode::Conflict.status_code(), StatusCode::CONFLICT);
        assert_eq!(
            ErrorCode::ValidationError.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ErrorCode::UnprocessableEntity.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ErrorCode::InternalError.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_api_error_constructors() {
        let err = ApiError::bad_request("bad");
        assert_eq!(err.code, ErrorCode::BadRequest);

        let err = ApiError::unauthorized("unauth");
        assert_eq!(err.code, ErrorCode::Unauthorized);

        let err = ApiError::forbidden("forbid");
        assert_eq!(err.code, ErrorCode::Forbidden);

        let err = ApiError::not_found("missing");
        assert_eq!(err.code, ErrorCode::NotFound);

        let err = ApiError::conflict("dup");
        assert_eq!(err.code, ErrorCode::Conflict);

        let err = ApiError::unprocessable("invalid");
        assert_eq!(err.code, ErrorCode::UnprocessableEntity);

        let err = ApiError::internal("error");
        assert_eq!(err.code, ErrorCode::InternalError);
    }

    #[test]
    fn test_validation_error() {
        let mut details = HashMap::new();
        details.insert("username".to_string(), vec!["Too short".to_string()]);
        details.insert(
            "email".to_string(),
            vec!["Invalid format".to_string(), "Required".to_string()],
        );

        let err = ApiError::validation(details);
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert_eq!(err.message, "Validation failed");
        assert!(err.details.is_some());

        let details = err.details.unwrap();
        assert_eq!(
            details.get("username").unwrap(),
            &vec!["Too short".to_string()]
        );
        assert_eq!(
            details.get("email").unwrap(),
            &vec!["Invalid format".to_string(), "Required".to_string()]
        );
    }
}
