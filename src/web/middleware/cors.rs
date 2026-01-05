//! CORS middleware configuration.

use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use tower_http::cors::{Any, CorsLayer};

/// Create a CORS layer from configuration.
pub fn create_cors_layer(origins: &[String]) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::OPTIONS,
    ];

    // When origins are specified, use credentials mode with explicit headers
    // When no origins specified (dev mode), use permissive mode without credentials
    if origins.is_empty() {
        // Development mode: allow any origin but without credentials
        CorsLayer::new()
            .allow_methods(methods)
            .allow_headers(Any)
            .allow_origin(Any)
    } else {
        // Production mode: specific origins with credentials
        let parsed_origins: Vec<HeaderValue> =
            origins.iter().filter_map(|o| o.parse().ok()).collect();

        if parsed_origins.is_empty() {
            // Fallback to dev mode if no valid origins
            CorsLayer::new()
                .allow_methods(methods)
                .allow_headers(Any)
                .allow_origin(Any)
        } else {
            CorsLayer::new()
                .allow_methods(methods)
                .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
                .allow_credentials(true)
                .allow_origin(parsed_origins)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cors_layer_empty_origins() {
        let _layer = create_cors_layer(&[]);
        // Should not panic
    }

    #[test]
    fn test_create_cors_layer_with_origins() {
        let origins = vec![
            "http://localhost:3000".to_string(),
            "http://localhost:5173".to_string(),
        ];
        let _layer = create_cors_layer(&origins);
        // Should not panic
    }
}
