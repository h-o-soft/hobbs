//! Router configuration for Web API.

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use super::handlers::{
    // Auth handlers
    login, logout, me, refresh, register,
    // Board handlers
    create_flat_post, create_thread, create_thread_post, delete_post, get_board, get_thread,
    list_boards, list_flat_posts, list_thread_posts, list_threads,
    // Mail handlers
    delete_mail, get_mail, get_unread_count, list_inbox, list_sent, send_mail,
    // RSS handlers
    get_feed, get_item, list_feeds, list_items, mark_as_read,
    // User handlers
    change_password, get_my_profile, get_user, list_users, update_my_profile,
    // State
    AppState,
};
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
    let auth_protected_routes = Router::new().route("/me", get(me));

    // Combine auth routes
    let auth_routes = Router::new()
        .merge(auth_public_routes)
        .merge(auth_protected_routes);

    // Board routes
    let board_routes = Router::new()
        .route("/", get(list_boards))
        .route("/{id}", get(get_board))
        // Thread-based board routes
        .route("/{id}/threads", get(list_threads))
        .route("/{id}/threads", post(create_thread))
        // Flat board routes
        .route("/{id}/posts", get(list_flat_posts))
        .route("/{id}/posts", post(create_flat_post));

    // Thread routes
    let thread_routes = Router::new()
        .route("/{id}", get(get_thread))
        .route("/{id}/posts", get(list_thread_posts))
        .route("/{id}/posts", post(create_thread_post));

    // Post routes
    let post_routes = Router::new().route("/{id}", delete(delete_post));

    // Mail routes
    let mail_routes = Router::new()
        .route("/inbox", get(list_inbox))
        .route("/sent", get(list_sent))
        .route("/unread-count", get(get_unread_count))
        .route("/", post(send_mail))
        .route("/{id}", get(get_mail))
        .route("/{id}", delete(delete_mail));

    // User routes
    let user_routes = Router::new()
        .route("/", get(list_users))
        .route("/me", get(get_my_profile))
        .route("/me", put(update_my_profile))
        .route("/me/password", post(change_password))
        .route("/{id}", get(get_user));

    // RSS routes
    let rss_routes = Router::new()
        .route("/", get(list_feeds))
        .route("/{id}", get(get_feed))
        .route("/{id}/items", get(list_items))
        .route("/{feed_id}/items/{item_id}", get(get_item))
        .route("/{id}/mark-read", post(mark_as_read));

    // API routes
    let api_routes = Router::new()
        .nest("/auth", auth_routes)
        .nest("/boards", board_routes)
        .nest("/threads", thread_routes)
        .nest("/posts", post_routes)
        .nest("/mail", mail_routes)
        .nest("/users", user_routes)
        .nest("/rss", rss_routes);

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
