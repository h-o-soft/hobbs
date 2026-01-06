//! Web API Authentication Tests
//!
//! Integration tests for authentication endpoints.

use axum::http::header::AUTHORIZATION;
use axum_test::TestServer;
use hobbs::config::WebConfig;
use hobbs::web::handlers::AppState;
use hobbs::web::middleware::JwtState;
use hobbs::web::router::create_router;
use hobbs::Database;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create a test configuration.
fn create_test_config() -> WebConfig {
    WebConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port: 0,
        cors_origins: vec![],
        jwt_secret: "test-secret-key-for-testing-only".to_string(),
        jwt_access_token_expiry_secs: 900,
        jwt_refresh_token_expiry_days: 7,
        serve_static: false,
        static_path: "web/dist".to_string(),
        login_rate_limit: 100,
        api_rate_limit: 1000,
    }
}

/// Create a test server with an in-memory database.
async fn create_test_server() -> (TestServer, Arc<Mutex<Database>>) {
    let config = create_test_config();

    // Create in-memory database
    let db = Database::open_in_memory().expect("Failed to create test database");
    let shared_db = Arc::new(Mutex::new(db));

    // Create app state
    let app_state = Arc::new(AppState::new(
        shared_db.clone(),
        &config.jwt_secret,
        config.jwt_access_token_expiry_secs,
        config.jwt_refresh_token_expiry_days,
    ));

    // Create JWT state
    let jwt_state = Arc::new(JwtState::new(&config.jwt_secret));

    // Create router
    let router = create_router(app_state, jwt_state, None, &config);

    // Create test server
    let server = TestServer::new(router).expect("Failed to create test server");

    (server, shared_db)
}

/// Helper to register a test user and return tokens.
async fn register_test_user(
    server: &TestServer,
    username: &str,
    password: &str,
    nickname: &str,
) -> Value {
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": username,
            "password": password,
            "nickname": nickname
        }))
        .await;

    response.json::<Value>()
}

/// Helper to login and return tokens.
async fn login_user(server: &TestServer, username: &str, password: &str) -> Value {
    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": username,
            "password": password
        }))
        .await;

    response.json::<Value>()
}

// ============================================================================
// Registration Tests
// ============================================================================

#[tokio::test]
async fn test_register_success() {
    let (server, _db) = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "testuser",
            "password": "password123",
            "nickname": "Test User"
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert!(body["data"]["access_token"].is_string());
    assert!(body["data"]["refresh_token"].is_string());
    assert_eq!(body["data"]["user"]["username"], "testuser");
    assert_eq!(body["data"]["user"]["nickname"], "Test User");
    assert_eq!(body["data"]["user"]["role"], "member");
}

#[tokio::test]
async fn test_register_duplicate_username() {
    let (server, _db) = create_test_server().await;

    // Register first user
    server
        .post("/api/auth/register")
        .json(&json!({
            "username": "testuser",
            "password": "password123",
            "nickname": "Test User"
        }))
        .await
        .assert_status_ok();

    // Try to register with same username
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "testuser",
            "password": "password456",
            "nickname": "Another User"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::CONFLICT);

    let body: Value = response.json();
    assert_eq!(body["error"]["code"], "CONFLICT");
}

#[tokio::test]
async fn test_register_short_password() {
    let (server, _db) = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "testuser",
            "password": "short",
            "nickname": "Test User"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_register_empty_username() {
    let (server, _db) = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "",
            "password": "password123",
            "nickname": "Test User"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_with_email() {
    let (server, _db) = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "testuser",
            "password": "password123",
            "nickname": "Test User",
            "email": "test@example.com"
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["user"]["username"], "testuser");
}

// ============================================================================
// Login Tests
// ============================================================================

#[tokio::test]
async fn test_login_success() {
    let (server, _db) = create_test_server().await;

    // Register user first
    register_test_user(&server, "loginuser", "password123", "Login User").await;

    // Login
    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "loginuser",
            "password": "password123"
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert!(body["data"]["access_token"].is_string());
    assert!(body["data"]["refresh_token"].is_string());
    assert_eq!(body["data"]["user"]["username"], "loginuser");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let (server, _db) = create_test_server().await;

    // Register user first
    register_test_user(&server, "loginuser", "password123", "Login User").await;

    // Try wrong password
    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "loginuser",
            "password": "wrongpassword"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let (server, _db) = create_test_server().await;

    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "nonexistent",
            "password": "password123"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_empty_credentials() {
    let (server, _db) = create_test_server().await;

    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "",
            "password": ""
        }))
        .await;

    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

// ============================================================================
// Token Refresh Tests
// ============================================================================

#[tokio::test]
async fn test_refresh_token_success() {
    let (server, _db) = create_test_server().await;

    // Register and get tokens
    let login_response =
        register_test_user(&server, "refreshuser", "password123", "Refresh User").await;
    let refresh_token = login_response["data"]["refresh_token"]
        .as_str()
        .expect("No refresh token");

    // Refresh token
    let response = server
        .post("/api/auth/refresh")
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert!(body["data"]["access_token"].is_string());
    assert!(body["data"]["refresh_token"].is_string());
    // New refresh token should be different
    assert_ne!(
        body["data"]["refresh_token"].as_str().unwrap(),
        refresh_token
    );
}

#[tokio::test]
async fn test_refresh_token_invalid() {
    let (server, _db) = create_test_server().await;

    let response = server
        .post("/api/auth/refresh")
        .json(&json!({
            "refresh_token": "invalid-token"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_token_already_used() {
    let (server, _db) = create_test_server().await;

    // Register and get tokens
    let login_response =
        register_test_user(&server, "refreshuser2", "password123", "Refresh User 2").await;
    let refresh_token = login_response["data"]["refresh_token"]
        .as_str()
        .expect("No refresh token");

    // First refresh should succeed
    let response = server
        .post("/api/auth/refresh")
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .await;
    response.assert_status_ok();

    // Second refresh with same token should fail (token is revoked after use)
    let response = server
        .post("/api/auth/refresh")
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Logout Tests
// ============================================================================

#[tokio::test]
async fn test_logout_success() {
    let (server, _db) = create_test_server().await;

    // Register and get tokens
    let login_response =
        register_test_user(&server, "logoutuser", "password123", "Logout User").await;
    let refresh_token = login_response["data"]["refresh_token"]
        .as_str()
        .expect("No refresh token");

    // Logout
    let response = server
        .post("/api/auth/logout")
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .await;

    response.assert_status_ok();

    // Try to refresh with logged out token
    let response = server
        .post("/api/auth/refresh")
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Me (Current User) Tests
// ============================================================================

#[tokio::test]
async fn test_me_success() {
    let (server, _db) = create_test_server().await;

    // Register and get tokens
    let login_response = register_test_user(&server, "meuser", "password123", "Me User").await;
    let access_token = login_response["data"]["access_token"]
        .as_str()
        .expect("No access token");

    // Get current user info
    let response = server
        .get("/api/auth/me")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["username"], "meuser");
    assert_eq!(body["data"]["nickname"], "Me User");
    assert_eq!(body["data"]["role"], "member");
}

#[tokio::test]
async fn test_me_unauthorized() {
    let (server, _db) = create_test_server().await;

    // Try to get user info without token
    let response = server.get("/api/auth/me").await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_invalid_token() {
    let (server, _db) = create_test_server().await;

    // Try to get user info with invalid token
    let response = server
        .get("/api/auth/me")
        .add_header(AUTHORIZATION, "Bearer invalid-token")
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Token Expiry Tests
// ============================================================================

#[tokio::test]
async fn test_access_token_contains_expected_claims() {
    let (server, _db) = create_test_server().await;

    // Register and get tokens
    let login_response =
        register_test_user(&server, "claimsuser", "password123", "Claims User").await;
    let access_token = login_response["data"]["access_token"]
        .as_str()
        .expect("No access token");

    // Decode JWT payload (base64 decode the middle part)
    let parts: Vec<&str> = access_token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts");

    // Base64 decode the payload
    use base64::Engine;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let payload = engine
        .decode(parts[1])
        .expect("Failed to decode JWT payload");
    let claims: Value = serde_json::from_slice(&payload).expect("Failed to parse claims");

    // Check expected claims
    assert_eq!(claims["username"], "claimsuser");
    assert_eq!(claims["role"], "member");
    assert!(claims["sub"].is_number());
    assert!(claims["iat"].is_number());
    assert!(claims["exp"].is_number());
    assert!(claims["jti"].is_string());
}
