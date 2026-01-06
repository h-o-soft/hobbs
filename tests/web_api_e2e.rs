//! Web API End-to-End Scenario Tests
//!
//! These tests verify complete user flows across multiple API endpoints.

use axum::http::header::AUTHORIZATION;
use axum_test::TestServer;
use hobbs::board::{BoardRepository, BoardType, NewBoard};
use hobbs::config::WebConfig;
use hobbs::db::{Role, UserRepository, UserUpdate};
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

/// Get access token from response.
fn get_access_token(response: &Value) -> String {
    response["data"]["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Get user ID from response.
fn get_user_id(response: &Value) -> i64 {
    response["data"]["user"]["id"].as_i64().unwrap()
}

// ============================================================================
// E2E Scenario: Complete Registration and Profile Flow
// ============================================================================

#[tokio::test]
async fn test_e2e_registration_and_profile_flow() {
    let (server, _db) = create_test_server().await;

    // Step 1: Register a new user
    let register_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "newuser",
            "password": "password123",
            "nickname": "New User"
        }))
        .await;

    register_response.assert_status_ok();
    let body: Value = register_response.json();
    let access_token = get_access_token(&body);
    assert_eq!(body["data"]["user"]["username"], "newuser");

    // Step 2: Verify profile via /me endpoint
    let me_response = server
        .get("/api/auth/me")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .await;

    me_response.assert_status_ok();
    let me_body: Value = me_response.json();
    assert_eq!(me_body["data"]["username"], "newuser");
    assert_eq!(me_body["data"]["nickname"], "New User");
    assert_eq!(me_body["data"]["role"], "member");

    // Step 3: Logout
    let refresh_token = body["data"]["refresh_token"].as_str().unwrap();
    let logout_response = server
        .post("/api/auth/logout")
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .await;

    logout_response.assert_status_ok();

    // Step 4: Re-login
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "newuser",
            "password": "password123"
        }))
        .await;

    login_response.assert_status_ok();
    let login_body: Value = login_response.json();
    assert!(login_body["data"]["access_token"].is_string());
}

// ============================================================================
// E2E Scenario: Board Browsing and Posting Flow
// ============================================================================

#[tokio::test]
async fn test_e2e_board_thread_post_flow() {
    let (server, db) = create_test_server().await;

    // Setup: Create a board
    {
        let db = db.lock().await;
        let repo = BoardRepository::new(&*db);
        let new_board = NewBoard::new("General Discussion")
            .with_description("Talk about anything")
            .with_board_type(BoardType::Thread)
            .with_min_read_role(Role::Member)
            .with_min_write_role(Role::Member);
        repo.create(&new_board).expect("Failed to create board");
    }

    // Step 1: Register user
    let register_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "poster",
            "password": "password123",
            "nickname": "Active Poster"
        }))
        .await;

    register_response.assert_status_ok();
    let token = get_access_token(&register_response.json::<Value>());

    // Step 2: List boards
    let boards_response = server
        .get("/api/boards")
        .add_header(AUTHORIZATION, format!("Bearer {}", token))
        .await;

    boards_response.assert_status_ok();
    let boards: Value = boards_response.json();
    assert_eq!(boards["data"].as_array().unwrap().len(), 1);
    let board_id = boards["data"][0]["id"].as_i64().unwrap();
    assert_eq!(boards["data"][0]["name"], "General Discussion");

    // Step 3: Get board details
    let board_response = server
        .get(&format!("/api/boards/{}", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token))
        .await;

    board_response.assert_status_ok();
    let board: Value = board_response.json();
    assert_eq!(board["data"]["name"], "General Discussion");
    assert_eq!(board["data"]["thread_count"], 0);

    // Step 4: Create a new thread
    let create_thread_response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token))
        .json(&json!({
            "title": "My First Thread",
            "body": "Hello everyone! This is my first post."
        }))
        .await;

    create_thread_response.assert_status_ok();
    let thread: Value = create_thread_response.json();
    let thread_id = thread["data"]["id"].as_i64().unwrap();
    assert_eq!(thread["data"]["title"], "My First Thread");

    // Step 5: List threads in board
    let threads_response = server
        .get(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token))
        .await;

    threads_response.assert_status_ok();
    let threads: Value = threads_response.json();
    assert_eq!(threads["data"].as_array().unwrap().len(), 1);
    assert_eq!(threads["data"][0]["title"], "My First Thread");

    // Step 6: Get thread with posts
    let thread_response = server
        .get(&format!("/api/threads/{}", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token))
        .await;

    thread_response.assert_status_ok();
    let thread_detail: Value = thread_response.json();
    assert_eq!(thread_detail["data"]["title"], "My First Thread");

    // Step 7: Reply to thread
    let reply_response = server
        .post(&format!("/api/threads/{}/posts", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token))
        .json(&json!({
            "body": "This is a reply to my own thread!"
        }))
        .await;

    reply_response.assert_status_ok();
    let reply: Value = reply_response.json();
    assert_eq!(reply["data"]["body"], "This is a reply to my own thread!");

    // Step 8: Verify posts in thread
    let posts_response = server
        .get(&format!("/api/threads/{}/posts", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token))
        .await;

    posts_response.assert_status_ok();
    let posts: Value = posts_response.json();
    // Should have 2 posts: original + reply
    assert_eq!(posts["data"].as_array().unwrap().len(), 2);
}

// ============================================================================
// E2E Scenario: Mail Communication Flow
// ============================================================================

#[tokio::test]
async fn test_e2e_mail_send_receive_flow() {
    let (server, _db) = create_test_server().await;

    // Step 1: Register sender
    let sender_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "sender",
            "password": "password123",
            "nickname": "Mail Sender"
        }))
        .await;

    sender_response.assert_status_ok();
    let sender_token = get_access_token(&sender_response.json::<Value>());

    // Step 2: Register receiver
    let receiver_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "receiver",
            "password": "password123",
            "nickname": "Mail Receiver"
        }))
        .await;

    receiver_response.assert_status_ok();
    let receiver_token = get_access_token(&receiver_response.json::<Value>());

    // Step 3: Sender checks empty inbox
    let inbox_empty_response = server
        .get("/api/mail/inbox")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .await;

    inbox_empty_response.assert_status_ok();
    let inbox_empty: Value = inbox_empty_response.json();
    assert_eq!(inbox_empty["data"].as_array().unwrap().len(), 0);

    // Step 4: Sender sends mail to receiver
    let send_response = server
        .post("/api/mail")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .json(&json!({
            "recipient": "receiver",
            "subject": "Hello!",
            "body": "Nice to meet you!"
        }))
        .await;

    send_response.assert_status_ok();
    let sent_mail: Value = send_response.json();
    assert_eq!(sent_mail["data"]["subject"], "Hello!");

    // Step 5: Receiver checks unread count
    let unread_response = server
        .get("/api/mail/unread-count")
        .add_header(AUTHORIZATION, format!("Bearer {}", receiver_token))
        .await;

    unread_response.assert_status_ok();
    let unread: Value = unread_response.json();
    assert_eq!(unread["data"]["count"], 1);

    // Step 6: Receiver checks inbox
    let inbox_response = server
        .get("/api/mail/inbox")
        .add_header(AUTHORIZATION, format!("Bearer {}", receiver_token))
        .await;

    inbox_response.assert_status_ok();
    let inbox: Value = inbox_response.json();
    let mails = inbox["data"].as_array().unwrap();
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0]["subject"], "Hello!");
    assert_eq!(mails[0]["sender"]["nickname"], "Mail Sender");
    let mail_id = mails[0]["id"].as_i64().unwrap();

    // Step 7: Receiver reads the mail
    let mail_response = server
        .get(&format!("/api/mail/{}", mail_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", receiver_token))
        .await;

    mail_response.assert_status_ok();
    let mail: Value = mail_response.json();
    assert_eq!(mail["data"]["subject"], "Hello!");
    assert_eq!(mail["data"]["body"], "Nice to meet you!");

    // Step 8: Verify unread count is now 0
    let unread_after_response = server
        .get("/api/mail/unread-count")
        .add_header(AUTHORIZATION, format!("Bearer {}", receiver_token))
        .await;

    unread_after_response.assert_status_ok();
    let unread_after: Value = unread_after_response.json();
    assert_eq!(unread_after["data"]["count"], 0);

    // Step 9: Sender checks sent mail
    let sent_response = server
        .get("/api/mail/sent")
        .add_header(AUTHORIZATION, format!("Bearer {}", sender_token))
        .await;

    sent_response.assert_status_ok();
    let sent: Value = sent_response.json();
    assert_eq!(sent["data"].as_array().unwrap().len(), 1);

    // Step 10: Receiver deletes the mail
    let delete_response = server
        .delete(&format!("/api/mail/{}", mail_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", receiver_token))
        .await;

    delete_response.assert_status_ok();

    // Step 11: Verify mail is deleted from inbox
    let inbox_after_delete = server
        .get("/api/mail/inbox")
        .add_header(AUTHORIZATION, format!("Bearer {}", receiver_token))
        .await;

    inbox_after_delete.assert_status_ok();
    let inbox_final: Value = inbox_after_delete.json();
    assert_eq!(inbox_final["data"].as_array().unwrap().len(), 0);
}

// ============================================================================
// E2E Scenario: Token Refresh Flow
// ============================================================================

#[tokio::test]
async fn test_e2e_token_refresh_flow() {
    let (server, _db) = create_test_server().await;

    // Step 1: Register user
    let register_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "refresher",
            "password": "password123",
            "nickname": "Token Refresher"
        }))
        .await;

    register_response.assert_status_ok();
    let body: Value = register_response.json();
    let original_access_token = get_access_token(&body);
    let original_refresh_token = body["data"]["refresh_token"].as_str().unwrap().to_string();

    // Step 2: Use access token to access protected endpoint
    let me_response = server
        .get("/api/auth/me")
        .add_header(AUTHORIZATION, format!("Bearer {}", original_access_token))
        .await;

    me_response.assert_status_ok();

    // Step 3: Refresh the token
    let refresh_response = server
        .post("/api/auth/refresh")
        .json(&json!({
            "refresh_token": original_refresh_token
        }))
        .await;

    refresh_response.assert_status_ok();
    let refresh_body: Value = refresh_response.json();
    let new_access_token = get_access_token(&refresh_body);
    let new_refresh_token = refresh_body["data"]["refresh_token"]
        .as_str()
        .unwrap()
        .to_string();

    // Tokens should be different
    assert_ne!(new_access_token, original_access_token);
    assert_ne!(new_refresh_token, original_refresh_token);

    // Step 4: New access token should work
    let me_with_new_token = server
        .get("/api/auth/me")
        .add_header(AUTHORIZATION, format!("Bearer {}", new_access_token))
        .await;

    me_with_new_token.assert_status_ok();

    // Step 5: Old refresh token should be invalidated
    let old_refresh_response = server
        .post("/api/auth/refresh")
        .json(&json!({
            "refresh_token": original_refresh_token
        }))
        .await;

    old_refresh_response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ============================================================================
// E2E Scenario: Admin User Management Flow
// ============================================================================

#[tokio::test]
async fn test_e2e_admin_user_management_flow() {
    let (server, db) = create_test_server().await;

    // Step 1: Register admin user
    let admin_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "admin",
            "password": "password123",
            "nickname": "Administrator"
        }))
        .await;

    admin_response.assert_status_ok();
    let admin_body: Value = admin_response.json();
    let admin_id = get_user_id(&admin_body);

    // Promote to SysOp
    {
        let db = db.lock().await;
        let repo = UserRepository::new(&*db);
        let update = UserUpdate {
            role: Some(Role::SysOp),
            ..Default::default()
        };
        repo.update(admin_id, &update).unwrap();
    }

    // Re-login as admin
    let admin_login = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "admin",
            "password": "password123"
        }))
        .await;
    let admin_token = get_access_token(&admin_login.json::<Value>());

    // Step 2: Register regular user
    let user_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "regularuser",
            "password": "password123",
            "nickname": "Regular User"
        }))
        .await;

    user_response.assert_status_ok();
    let user_body: Value = user_response.json();
    let user_id = get_user_id(&user_body);

    // Step 3: Admin lists all users
    let users_response = server
        .get("/api/admin/users")
        .add_header(AUTHORIZATION, format!("Bearer {}", admin_token))
        .await;

    users_response.assert_status_ok();
    let users: Value = users_response.json();
    assert!(users["data"].as_array().unwrap().len() >= 2);

    // Step 4: Admin promotes user to SubOp
    let promote_response = server
        .put(&format!("/api/admin/users/{}/role", user_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", admin_token))
        .json(&json!({
            "role": "subop"
        }))
        .await;

    promote_response.assert_status_ok();

    // Step 5: Verify user role changed by listing users
    let users_after_response = server
        .get("/api/admin/users")
        .add_header(AUTHORIZATION, format!("Bearer {}", admin_token))
        .await;

    users_after_response.assert_status_ok();
    let users_after: Value = users_after_response.json();
    let users_list = users_after["data"].as_array().unwrap();
    let promoted_user = users_list.iter().find(|u| u["id"] == user_id).unwrap();
    assert_eq!(promoted_user["role"], "subop");
}

// ============================================================================
// E2E Scenario: Multi-User Board Interaction
// ============================================================================

#[tokio::test]
async fn test_e2e_multi_user_board_interaction() {
    let (server, db) = create_test_server().await;

    // Setup: Create a board
    {
        let db = db.lock().await;
        let repo = BoardRepository::new(&*db);
        let new_board = NewBoard::new("Discussion Forum")
            .with_description("A place for discussions")
            .with_board_type(BoardType::Thread)
            .with_min_read_role(Role::Member)
            .with_min_write_role(Role::Member);
        repo.create(&new_board).expect("Failed to create board");
    }

    // Step 1: Register User A
    let user_a_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "user_a",
            "password": "password123",
            "nickname": "User A"
        }))
        .await;
    let token_a = get_access_token(&user_a_response.json::<Value>());

    // Step 2: Register User B
    let user_b_response = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "user_b",
            "password": "password123",
            "nickname": "User B"
        }))
        .await;
    let token_b = get_access_token(&user_b_response.json::<Value>());

    // Step 3: User A creates a thread
    let boards: Value = server
        .get("/api/boards")
        .add_header(AUTHORIZATION, format!("Bearer {}", token_a))
        .await
        .json();
    let board_id = boards["data"][0]["id"].as_i64().unwrap();

    let thread_response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token_a))
        .json(&json!({
            "title": "Question about Rust",
            "body": "What is the best way to handle errors?"
        }))
        .await;

    thread_response.assert_status_ok();
    let thread: Value = thread_response.json();
    let thread_id = thread["data"]["id"].as_i64().unwrap();

    // Step 4: User B views the thread
    let thread_view = server
        .get(&format!("/api/threads/{}", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token_b))
        .await;

    thread_view.assert_status_ok();
    let thread_detail: Value = thread_view.json();
    assert_eq!(thread_detail["data"]["author"]["nickname"], "User A");

    // Step 5: User B replies
    let reply_response = server
        .post(&format!("/api/threads/{}/posts", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token_b))
        .json(&json!({
            "body": "You should use the Result type and the ? operator!"
        }))
        .await;

    reply_response.assert_status_ok();

    // Step 6: User A checks the thread for replies
    let posts_response = server
        .get(&format!("/api/threads/{}/posts", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", token_a))
        .await;

    posts_response.assert_status_ok();
    let posts: Value = posts_response.json();
    let posts_array = posts["data"].as_array().unwrap();
    assert_eq!(posts_array.len(), 2);

    // Verify authors
    assert_eq!(posts_array[0]["author"]["nickname"], "User A");
    assert_eq!(posts_array[1]["author"]["nickname"], "User B");
}
