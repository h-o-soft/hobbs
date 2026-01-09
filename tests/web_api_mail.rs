//! Web API Mail Tests
//!
//! Integration tests for mail endpoints.

use axum::http::header::AUTHORIZATION;
use axum_test::TestServer;
use hobbs::config::WebConfig;
use hobbs::web::handlers::AppState;
use hobbs::web::middleware::JwtState;
use hobbs::web::router::create_router;
use hobbs::Database;
use serde_json::{json, Value};
use std::sync::Arc;

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
async fn create_test_server() -> (TestServer, Arc<Database>) {
    let config = create_test_config();

    // Create in-memory database
    let db = Database::open_in_memory()
        .await
        .expect("Failed to create test database");
    let shared_db = Arc::new(db);

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

/// Get access token from response.
fn get_access_token(response: &Value) -> String {
    response["data"]["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

// ============================================================================
// Send Mail Tests
// ============================================================================

#[tokio::test]
async fn test_send_mail_success() {
    let (server, _db) = create_test_server().await;

    // Register sender
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    // Register recipient
    register_test_user(&server, "recipient", "password123", "Recipient").await;

    // Send mail
    let response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .json(&json!({
            "recipient": "recipient",
            "subject": "Hello",
            "body": "This is a test message."
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["subject"], "Hello");
}

#[tokio::test]
async fn test_send_mail_to_nonexistent_user() {
    let (server, _db) = create_test_server().await;

    // Register sender
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    // Try to send mail to nonexistent user
    let response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .json(&json!({
            "recipient": "nonexistent",
            "subject": "Hello",
            "body": "This is a test message."
        }))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_send_mail_unauthorized() {
    let (server, _db) = create_test_server().await;

    // Try to send mail without authentication
    let response = server
        .post("/api/mail")
        .json(&json!({
            "recipient": "someone",
            "subject": "Hello",
            "body": "This is a test message."
        }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_send_mail_to_self_not_allowed() {
    let (server, _db) = create_test_server().await;

    // Register user
    let user_response = register_test_user(&server, "selfmail", "password123", "Self Mail").await;
    let user_token = get_access_token(&user_response);

    // Try to send mail to self - should fail
    let response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .json(&json!({
            "recipient": "selfmail",
            "subject": "Note to self",
            "body": "This is a note to myself."
        }))
        .await;

    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

// ============================================================================
// Inbox Tests
// ============================================================================

#[tokio::test]
async fn test_list_inbox_empty() {
    let (server, _db) = create_test_server().await;

    // Register user
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // List inbox
    let response = server
        .get("/api/mail/inbox")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_inbox_with_mails() {
    let (server, _db) = create_test_server().await;

    // Register sender and recipient
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    let recipient_response =
        register_test_user(&server, "recipient", "password123", "Recipient").await;
    let recipient_token = get_access_token(&recipient_response);

    // Send multiple mails
    for i in 1..=3 {
        server
            .post("/api/mail")
            .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
            .json(&json!({
                "recipient": "recipient",
                "subject": format!("Mail {}", i),
                "body": format!("Content {}", i)
            }))
            .await
            .assert_status_ok();
    }

    // List recipient's inbox
    let response = server
        .get("/api/mail/inbox")
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    let mails = body["data"].as_array().unwrap();
    assert_eq!(mails.len(), 3);
}

#[tokio::test]
async fn test_list_inbox_unauthorized() {
    let (server, _db) = create_test_server().await;

    let response = server.get("/api/mail/inbox").await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Sent Mail Tests
// ============================================================================

#[tokio::test]
async fn test_list_sent_mails() {
    let (server, _db) = create_test_server().await;

    // Register sender and recipient
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    register_test_user(&server, "recipient", "password123", "Recipient").await;

    // Send multiple mails
    for i in 1..=2 {
        server
            .post("/api/mail")
            .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
            .json(&json!({
                "recipient": "recipient",
                "subject": format!("Sent Mail {}", i),
                "body": format!("Content {}", i)
            }))
            .await
            .assert_status_ok();
    }

    // List sender's sent mails
    let response = server
        .get("/api/mail/sent")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    let mails = body["data"].as_array().unwrap();
    assert_eq!(mails.len(), 2);
}

// ============================================================================
// Get Mail Detail Tests
// ============================================================================

#[tokio::test]
async fn test_get_mail_detail_success() {
    let (server, _db) = create_test_server().await;

    // Register sender and recipient
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    let recipient_response =
        register_test_user(&server, "recipient", "password123", "Recipient").await;
    let recipient_token = get_access_token(&recipient_response);

    // Send mail
    let send_response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .json(&json!({
            "recipient": "recipient",
            "subject": "Test Subject",
            "body": "Test Body Content"
        }))
        .await;

    let mail_id = send_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Get mail detail as recipient
    let response = server
        .get(&format!("/api/mail/{}", mail_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["subject"], "Test Subject");
    assert_eq!(body["data"]["body"], "Test Body Content");
}

#[tokio::test]
async fn test_get_mail_detail_not_found() {
    let (server, _db) = create_test_server().await;

    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    let response = server
        .get("/api/mail/99999")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_mail_detail_forbidden_for_other_user() {
    let (server, _db) = create_test_server().await;

    // Register users
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    register_test_user(&server, "recipient", "password123", "Recipient").await;

    let other_response = register_test_user(&server, "other", "password123", "Other").await;
    let other_token = get_access_token(&other_response);

    // Send mail from sender to recipient
    let send_response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .json(&json!({
            "recipient": "recipient",
            "subject": "Private",
            "body": "Private content"
        }))
        .await;

    let mail_id = send_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Try to access mail as other user
    let response = server
        .get(&format!("/api/mail/{}", mail_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", other_token))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// Delete Mail Tests
// ============================================================================

#[tokio::test]
async fn test_delete_mail_success() {
    let (server, _db) = create_test_server().await;

    // Register sender and recipient
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    let recipient_response =
        register_test_user(&server, "recipient", "password123", "Recipient").await;
    let recipient_token = get_access_token(&recipient_response);

    // Send mail
    let send_response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .json(&json!({
            "recipient": "recipient",
            "subject": "To Delete",
            "body": "This will be deleted"
        }))
        .await;

    let mail_id = send_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Delete mail as recipient
    let response = server
        .delete(&format!("/api/mail/{}", mail_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await;

    response.assert_status_ok();

    // Verify mail is deleted (should not be found)
    let response = server
        .get(&format!("/api/mail/{}", mail_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_mail_forbidden_for_other_user() {
    let (server, _db) = create_test_server().await;

    // Register users
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    register_test_user(&server, "recipient", "password123", "Recipient").await;

    let other_response = register_test_user(&server, "other", "password123", "Other").await;
    let other_token = get_access_token(&other_response);

    // Send mail
    let send_response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .json(&json!({
            "recipient": "recipient",
            "subject": "Private",
            "body": "Private content"
        }))
        .await;

    let mail_id = send_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Try to delete mail as other user
    let response = server
        .delete(&format!("/api/mail/{}", mail_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", other_token))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// Unread Count Tests
// ============================================================================

#[tokio::test]
async fn test_unread_count() {
    let (server, _db) = create_test_server().await;

    // Register sender and recipient
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    let recipient_response =
        register_test_user(&server, "recipient", "password123", "Recipient").await;
    let recipient_token = get_access_token(&recipient_response);

    // Check initial unread count
    let response = server
        .get("/api/mail/unread-count")
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["data"]["count"], 0);

    // Send mails
    for i in 1..=3 {
        server
            .post("/api/mail")
            .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
            .json(&json!({
                "recipient": "recipient",
                "subject": format!("Mail {}", i),
                "body": format!("Content {}", i)
            }))
            .await
            .assert_status_ok();
    }

    // Check unread count after receiving mails
    let response = server
        .get("/api/mail/unread-count")
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["data"]["count"], 3);
}

// ============================================================================
// Mark as Read Tests
// ============================================================================

#[tokio::test]
async fn test_reading_mail_marks_as_read() {
    let (server, _db) = create_test_server().await;

    // Register sender and recipient
    let sender_response = register_test_user(&server, "sender", "password123", "Sender").await;
    let sender_token = get_access_token(&sender_response);

    let recipient_response =
        register_test_user(&server, "recipient", "password123", "Recipient").await;
    let recipient_token = get_access_token(&recipient_response);

    // Send mail
    let send_response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .json(&json!({
            "recipient": "recipient",
            "subject": "Test",
            "body": "Test content"
        }))
        .await;

    let mail_id = send_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Check unread count before reading
    let response = server
        .get("/api/mail/unread-count")
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await;
    let body: Value = response.json();
    assert_eq!(body["data"]["count"], 1);

    // Read the mail
    server
        .get(&format!("/api/mail/{}", mail_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await
        .assert_status_ok();

    // Check unread count after reading
    let response = server
        .get("/api/mail/unread-count")
        .add_header(AUTHORIZATION, format!("Bearer {}", recipient_token))
        .await;
    let body: Value = response.json();
    assert_eq!(body["data"]["count"], 0);
}
