//! Security headers middleware.

use axum::{
    body::Body,
    http::{header::HeaderValue, Request},
    middleware::Next,
    response::Response,
};

/// Security headers middleware.
///
/// Adds the following headers to all responses:
/// - X-Content-Type-Options: nosniff
/// - X-Frame-Options: DENY
/// - Referrer-Policy: strict-origin-when-cross-origin
/// - X-XSS-Protection: 0 (deprecated but defensive)
///
/// Note: Strict-Transport-Security should be set at the reverse proxy level
/// as it requires HTTPS.
pub async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );

    // Prevent clickjacking
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));

    // Control referrer information
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // XSS protection (set to 0 as it's deprecated and can cause issues)
    // Modern browsers use Content-Security-Policy instead
    headers.insert("X-XSS-Protection", HeaderValue::from_static("0"));

    // Cache control for API responses
    if !headers.contains_key("Cache-Control") {
        headers.insert(
            "Cache-Control",
            HeaderValue::from_static("no-store, max-age=0"),
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::StatusCode, routing::get, Router};
    use tower::util::ServiceExt;

    async fn dummy_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_security_headers_added() {
        use axum::middleware;

        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(security_headers));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();
        assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
        assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
        assert_eq!(
            headers.get("Referrer-Policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(headers.get("X-XSS-Protection").unwrap(), "0");
        assert_eq!(headers.get("Cache-Control").unwrap(), "no-store, max-age=0");
    }
}
