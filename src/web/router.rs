//! Router configuration for Web API.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, patch, post, put},
    Router,
};
use std::path::Path;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::chat::ChatRoomManager;
use crate::config::WebConfig;

use super::handlers::{
    // RSS handlers
    add_feed,
    // Admin handlers
    admin_create_board,
    admin_create_folder,
    admin_delete_board,
    admin_delete_folder,
    admin_list_boards,
    admin_list_folders,
    admin_list_users,
    admin_reset_password,
    admin_update_board,
    admin_update_folder,
    admin_update_role,
    admin_update_status,
    admin_update_user,
    // User handlers
    change_password,
    // Board handlers
    create_flat_post,
    create_thread,
    create_thread_post,
    delete_feed,
    // File handlers
    delete_file,
    // Mail handlers
    delete_mail,
    delete_post,
    download_file,
    get_board,
    get_feed,
    get_file,
    get_folder,
    get_item,
    get_mail,
    get_my_profile,
    // Config handlers
    get_public_config,
    get_thread,
    get_unread_count,
    get_user,
    get_user_by_username,
    list_boards,
    list_feeds,
    list_files,
    list_flat_posts,
    list_folders,
    list_inbox,
    list_items,
    list_sent,
    list_thread_posts,
    list_threads,
    list_users,
    // Auth handlers
    login,
    logout,
    mark_as_read,
    me,
    refresh,
    register,
    send_mail,
    update_my_profile,
    update_post,
    update_thread,
    upload_file,
    // State
    AppState,
};
use super::middleware::{
    api_rate_limit, create_cors_layer, jwt_auth, login_rate_limit, security_headers, JwtState,
    RateLimitState,
};
use super::openapi::ApiDoc;
use super::ws::{chat_ws_handler, ChatWsState};

/// Create the main API router.
pub fn create_router(
    app_state: Arc<AppState>,
    jwt_state: Arc<JwtState>,
    chat_manager: Option<Arc<ChatRoomManager>>,
    web_config: &WebConfig,
) -> Router {
    // Create rate limit state
    let rate_limit_state = Arc::new(RateLimitState::new(
        web_config.login_rate_limit,
        web_config.api_rate_limit,
    ));

    // Start cleanup task for rate limiters
    rate_limit_state.clone().start_cleanup_task();

    // Clone for login rate limit middleware
    let login_rate_limit_state = rate_limit_state.clone();

    // Auth routes with login rate limiting
    let auth_public_routes = Router::new()
        .route(
            "/login",
            post(login).layer(middleware::from_fn(move |req, next| {
                let state = login_rate_limit_state.clone();
                login_rate_limit(state, req, next)
            })),
        )
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
        .route("/:id", get(get_board))
        // Thread-based board routes
        .route("/:id/threads", get(list_threads))
        .route("/:id/threads", post(create_thread))
        // Flat board routes
        .route("/:id/posts", get(list_flat_posts))
        .route("/:id/posts", post(create_flat_post));

    // Thread routes
    let thread_routes = Router::new()
        .route("/:id", get(get_thread))
        .route("/:id", patch(update_thread))
        .route("/:id/posts", get(list_thread_posts))
        .route("/:id/posts", post(create_thread_post));

    // Post routes
    let post_routes = Router::new()
        .route("/:id", delete(delete_post))
        .route("/:id", patch(update_post));

    // Mail routes
    let mail_routes = Router::new()
        .route("/inbox", get(list_inbox))
        .route("/sent", get(list_sent))
        .route("/unread-count", get(get_unread_count))
        .route("/", post(send_mail))
        .route("/:id", get(get_mail))
        .route("/:id", delete(delete_mail));

    // User routes
    let user_routes = Router::new()
        .route("/", get(list_users))
        .route("/me", get(get_my_profile))
        .route("/me", put(update_my_profile))
        .route("/me/password", post(change_password))
        .route("/by-username/:username", get(get_user_by_username))
        .route("/:id", get(get_user));

    // RSS routes (personal RSS reader)
    let rss_routes = Router::new()
        .route("/", get(list_feeds))
        .route("/feeds", post(add_feed))
        .route("/feeds/:id", delete(delete_feed))
        .route("/:id", get(get_feed))
        .route("/:id/items", get(list_items))
        .route("/:feed_id/items/:item_id", get(get_item))
        .route("/:id/mark-read", post(mark_as_read));

    // Folder routes
    let folder_routes = Router::new()
        .route("/", get(list_folders))
        .route("/:id", get(get_folder))
        .route("/:id/files", get(list_files))
        .route("/:id/files", post(upload_file));

    // File routes
    let file_routes = Router::new()
        .route("/:id", get(get_file))
        .route("/:id", delete(delete_file))
        .route("/:id/download", get(download_file));

    // Admin routes
    let admin_user_routes = Router::new()
        .route("/", get(admin_list_users))
        .route("/:id", put(admin_update_user))
        .route("/:id/role", put(admin_update_role))
        .route("/:id/status", put(admin_update_status))
        .route("/:id/reset-password", post(admin_reset_password));

    let admin_board_routes = Router::new()
        .route("/", get(admin_list_boards))
        .route("/", post(admin_create_board))
        .route("/:id", put(admin_update_board))
        .route("/:id", delete(admin_delete_board));

    let admin_folder_routes = Router::new()
        .route("/", get(admin_list_folders))
        .route("/", post(admin_create_folder))
        .route("/:id", put(admin_update_folder))
        .route("/:id", delete(admin_delete_folder));

    // Note: Admin RSS routes removed - RSS is now personal per-user
    let admin_routes = Router::new()
        .nest("/users", admin_user_routes)
        .nest("/boards", admin_board_routes)
        .nest("/folders", admin_folder_routes);

    // Chat WebSocket routes (if chat manager is provided)
    let chat_routes = if let Some(ref manager) = chat_manager {
        let chat_ws_state = Arc::new(ChatWsState::new(jwt_state.clone(), manager.clone()));
        Router::new()
            .route("/ws", get(chat_ws_handler))
            .with_state(chat_ws_state)
    } else {
        Router::new()
    };

    // Config routes (public, no auth required)
    let config_routes = Router::new().route("/public", get(get_public_config));

    // API routes
    let api_routes = Router::new()
        .nest("/auth", auth_routes)
        .nest("/boards", board_routes)
        .nest("/threads", thread_routes)
        .nest("/posts", post_routes)
        .nest("/mail", mail_routes)
        .nest("/users", user_routes)
        .nest("/rss", rss_routes)
        .nest("/folders", folder_routes)
        .nest("/files", file_routes)
        .nest("/admin", admin_routes)
        .nest("/chat", chat_routes)
        .nest("/config", config_routes);

    // Clone for middleware closures
    let jwt_state_for_middleware = jwt_state.clone();
    let api_rate_limit_state = rate_limit_state.clone();

    // Build the main router with middleware
    Router::new()
        .nest("/api", api_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::from_fn(security_headers))
                .layer(create_cors_layer(&web_config.cors_origins))
                .layer(middleware::from_fn(move |req, next| {
                    let state = api_rate_limit_state.clone();
                    api_rate_limit(state, req, next)
                }))
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

/// Create the Swagger UI router for API documentation.
pub fn create_swagger_router() -> Router {
    Router::new()
        .merge(SwaggerUi::new("/api/docs").url("/api/docs/openapi.json", ApiDoc::openapi()))
}

/// Middleware to add Cache-Control headers for static files.
///
/// - index.html and fallback: no-cache (always revalidate)
/// - Assets with hash (js, css in /assets/): max-age=31536000, immutable (1 year)
/// - Other static files: max-age=3600 (1 hour)
async fn static_cache_headers(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    // Only add cache headers for successful responses
    if response.status() != StatusCode::OK {
        return response;
    }

    let cache_control = if path == "/" || path.ends_with(".html") || path.ends_with("/") {
        // HTML files: always revalidate
        "no-cache"
    } else if path.starts_with("/assets/") {
        // Vite-built assets with content hash: cache forever
        "public, max-age=31536000, immutable"
    } else if path.ends_with(".js")
        || path.ends_with(".css")
        || path.ends_with(".woff2")
        || path.ends_with(".woff")
        || path.ends_with(".ttf")
    {
        // Other JS/CSS/fonts: long cache but not immutable
        "public, max-age=86400"
    } else if path.ends_with(".png")
        || path.ends_with(".jpg")
        || path.ends_with(".jpeg")
        || path.ends_with(".gif")
        || path.ends_with(".svg")
        || path.ends_with(".ico")
    {
        // Images: cache for 1 week
        "public, max-age=604800"
    } else {
        // Default: cache for 1 hour
        "public, max-age=3600"
    };

    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, cache_control.parse().unwrap());

    response
}

/// Create a static file serving router for SPA.
///
/// This serves static files from the specified directory and falls back to
/// index.html for unknown routes (SPA routing support).
///
/// Includes Cache-Control headers:
/// - index.html: no-cache
/// - /assets/* (hashed files): max-age=31536000, immutable
/// - Other files: appropriate caching based on type
pub fn create_static_router<P: AsRef<Path>>(static_path: P) -> Option<Router> {
    let path = static_path.as_ref();

    // Check if the directory exists
    if !path.exists() || !path.is_dir() {
        tracing::warn!(
            "Static files directory not found: {}. Static file serving disabled.",
            path.display()
        );
        return None;
    }

    let index_path = path.join("index.html");
    if !index_path.exists() {
        tracing::warn!(
            "index.html not found in {}. Static file serving disabled.",
            path.display()
        );
        return None;
    }

    tracing::info!("Serving static files from: {}", path.display());

    // Create ServeDir with fallback to index.html for SPA routing
    let serve_dir = ServeDir::new(path).not_found_service(ServeFile::new(&index_path));

    Some(
        Router::new()
            .fallback_service(serve_dir)
            .layer(middleware::from_fn(static_cache_headers)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_health_router() {
        let _router = create_health_router();
        // Should not panic
    }

    #[test]
    fn test_create_static_router_nonexistent_path() {
        let result = create_static_router("/nonexistent/path");
        assert!(result.is_none());
    }

    #[test]
    fn test_create_static_router_without_index() {
        let temp_dir = TempDir::new().unwrap();
        let result = create_static_router(temp_dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_create_static_router_with_index() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("index.html"), "<html></html>").unwrap();
        let result = create_static_router(temp_dir.path());
        assert!(result.is_some());
    }
}
