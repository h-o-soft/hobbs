//! Router configuration for Web API.

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use super::handlers::{login, logout, me, refresh, register, AppState};
use super::middleware::{create_cors_layer, jwt_auth, JwtState};

/// Create the main API router.
pub fn create_router(
    app_state: Arc<AppState>,
    jwt_state: Arc<JwtState>,
    cors_origins: &[String],
) -> Router {
    // Auth routes (no authentication required)
    let auth_public_routes = Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/refresh", post(refresh))
        .route("/register", post(register));

    // Auth routes (authentication required)
    let auth_protected_routes = Router::new()
        .route("/me", get(me));

    // Combine auth routes
    let auth_routes = Router::new()
        .merge(auth_public_routes)
        .merge(auth_protected_routes);

    // API routes
    let api_routes = Router::new()
        .nest("/auth", auth_routes);

    // Clone jwt_state for the middleware closure
    let jwt_state_for_middleware = jwt_state.clone();

    // Build the main router with middleware
    Router::new()
        .nest("/api", api_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(create_cors_layer(cors_origins))
                .layer(middleware::from_fn(move |req, next| {
                    let state = jwt_state_for_middleware.clone();
                    jwt_auth(state, req, next)
                })),
        )
        .with_state(app_state)
}

/// Create a health check router.
pub fn create_health_router() -> Router {
    Router::new().route("/health", get(health_check))
}

/// Health check handler.
async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_health_router() {
        let _router = create_health_router();
        // Should not panic
    }
}
