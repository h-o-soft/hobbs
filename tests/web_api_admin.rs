//! Web API Admin Tests
//!
//! Integration tests for admin endpoints and permission checks.

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

/// Get user ID from registration response.
fn get_user_id(response: &Value) -> i64 {
    response["data"]["user"]["id"].as_i64().unwrap()
}

/// Get access token from response.
fn get_access_token(response: &Value) -> String {
    response["data"]["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Update a user's role in the database.
async fn set_user_role(db: &Arc<Mutex<Database>>, user_id: i64, role: Role) {
    let db = db.lock().await;
    let repo = UserRepository::new(&*db);
    let update = UserUpdate {
        role: Some(role),
        ..Default::default()
    };
    repo.update(user_id, &update)
        .expect("Failed to update user role");
}

/// Create a test board in the database.
async fn create_test_board(db: &Arc<Mutex<Database>>, name: &str) -> i64 {
    let db = db.lock().await;
    let repo = BoardRepository::new(&*db);
    let new_board = NewBoard::new(name)
        .with_description(format!("{} board", name))
        .with_board_type(BoardType::Thread)
        .with_min_read_role(Role::Guest)
        .with_min_write_role(Role::Member);
    repo.create(&new_board)
        .expect("Failed to create test board")
        .id
}

// ============================================================================
// Admin User Management Tests
// ============================================================================

#[tokio::test]
async fn test_admin_list_users_as_sysop() {
    let (server, db) = create_test_server().await;

    // Register a user and make them sysop
    let login_response =
        register_test_user(&server, "sysopuser", "password123", "SysOp User").await;
    let user_id = get_user_id(&login_response);
    set_user_role(&db, user_id, Role::SysOp).await;

    // Re-login to get new token with updated role
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "sysopuser",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // List users
    let response = server
        .get("/api/admin/users")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn test_admin_list_users_as_subop() {
    let (server, db) = create_test_server().await;

    // Register a user and make them subop
    let login_response =
        register_test_user(&server, "subopuser", "password123", "SubOp User").await;
    let user_id = get_user_id(&login_response);
    set_user_role(&db, user_id, Role::SubOp).await;

    // Re-login to get new token with updated role
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "subopuser",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // List users - SubOp should have access
    let response = server
        .get("/api/admin/users")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_admin_list_users_as_member_forbidden() {
    let (server, _db) = create_test_server().await;

    // Register a regular member
    let login_response =
        register_test_user(&server, "memberuser", "password123", "Member User").await;
    let access_token = get_access_token(&login_response);

    // Try to list users - should be forbidden
    let response = server
        .get("/api/admin/users")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_list_users_unauthorized() {
    let (server, _db) = create_test_server().await;

    // Try to list users without auth
    let response = server.get("/api/admin/users").await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_update_user_role() {
    let (server, db) = create_test_server().await;

    // Create sysop
    let sysop_response = register_test_user(&server, "sysop", "password123", "SysOp").await;
    let sysop_id = get_user_id(&sysop_response);
    set_user_role(&db, sysop_id, Role::SysOp).await;

    // Create target user
    let target_response = register_test_user(&server, "target", "password123", "Target").await;
    let target_id = get_user_id(&target_response);

    // Re-login as sysop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "sysop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // Update target user's role to subop
    let response = server
        .put(&format!("/api/admin/users/{}/role", target_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "role": "subop"
        }))
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_admin_update_user_role_subop_cannot_change_roles() {
    let (server, db) = create_test_server().await;

    // Create subop
    let subop_response = register_test_user(&server, "subop", "password123", "SubOp").await;
    let subop_id = get_user_id(&subop_response);
    set_user_role(&db, subop_id, Role::SubOp).await;

    // Create target user
    let target_response = register_test_user(&server, "target", "password123", "Target").await;
    let target_id = get_user_id(&target_response);

    // Re-login as subop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "subop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // SubOp should not be able to change roles (requires SysOp)
    let response = server
        .put(&format!("/api/admin/users/{}/role", target_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "role": "subop"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_update_user_status() {
    let (server, db) = create_test_server().await;

    // Create sysop
    let sysop_response = register_test_user(&server, "sysop", "password123", "SysOp").await;
    let sysop_id = get_user_id(&sysop_response);
    set_user_role(&db, sysop_id, Role::SysOp).await;

    // Create target user
    let target_response = register_test_user(&server, "target", "password123", "Target").await;
    let target_id = get_user_id(&target_response);

    // Re-login as sysop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "sysop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // Deactivate user
    let response = server
        .put(&format!("/api/admin/users/{}/status", target_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "is_active": false
        }))
        .await;

    response.assert_status_ok();

    // Target user should not be able to login anymore
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "target",
            "password": "password123"
        }))
        .await;

    login_response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_reset_password() {
    let (server, db) = create_test_server().await;

    // Create sysop
    let sysop_response = register_test_user(&server, "sysop", "password123", "SysOp").await;
    let sysop_id = get_user_id(&sysop_response);
    set_user_role(&db, sysop_id, Role::SysOp).await;

    // Create target user
    let target_response = register_test_user(&server, "target", "password123", "Target").await;
    let target_id = get_user_id(&target_response);

    // Re-login as sysop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "sysop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // Reset password
    let response = server
        .post(&format!("/api/admin/users/{}/reset-password", target_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "new_password": "newpassword123"
        }))
        .await;

    response.assert_status_ok();

    // Target user should be able to login with new password
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "target",
            "password": "newpassword123"
        }))
        .await;

    login_response.assert_status_ok();

    // Old password should not work
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "target",
            "password": "password123"
        }))
        .await;

    login_response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Admin Board Management Tests
// ============================================================================

#[tokio::test]
async fn test_admin_create_board() {
    let (server, db) = create_test_server().await;

    // Create sysop
    let sysop_response = register_test_user(&server, "sysop", "password123", "SysOp").await;
    let sysop_id = get_user_id(&sysop_response);
    set_user_role(&db, sysop_id, Role::SysOp).await;

    // Re-login as sysop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "sysop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // Create board
    let response = server
        .post("/api/admin/boards")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "name": "New Board",
            "description": "A new test board",
            "board_type": "thread"
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["name"], "New Board");
}

#[tokio::test]
async fn test_admin_create_board_member_forbidden() {
    let (server, _db) = create_test_server().await;

    // Register a regular member
    let login_response = register_test_user(&server, "member", "password123", "Member").await;
    let access_token = get_access_token(&login_response);

    // Try to create board - should be forbidden
    let response = server
        .post("/api/admin/boards")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "name": "New Board",
            "description": "A new test board",
            "board_type": "thread"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_update_board() {
    let (server, db) = create_test_server().await;

    // Create a board
    let board_id = create_test_board(&db, "Test Board").await;

    // Create sysop
    let sysop_response = register_test_user(&server, "sysop", "password123", "SysOp").await;
    let sysop_id = get_user_id(&sysop_response);
    set_user_role(&db, sysop_id, Role::SysOp).await;

    // Re-login as sysop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "sysop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // Update board
    let response = server
        .put(&format!("/api/admin/boards/{}", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "name": "Updated Board Name"
        }))
        .await;

    response.assert_status_ok();

    // Verify board was updated
    let response = server.get(&format!("/api/boards/{}", board_id)).await;
    let body: Value = response.json();
    assert_eq!(body["data"]["name"], "Updated Board Name");
}

#[tokio::test]
async fn test_admin_delete_board() {
    let (server, db) = create_test_server().await;

    // Create a board
    let board_id = create_test_board(&db, "To Delete").await;

    // Create sysop
    let sysop_response = register_test_user(&server, "sysop", "password123", "SysOp").await;
    let sysop_id = get_user_id(&sysop_response);
    set_user_role(&db, sysop_id, Role::SysOp).await;

    // Re-login as sysop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "sysop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // Delete board
    let response = server
        .delete(&format!("/api/admin/boards/{}", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .await;

    response.assert_status_ok();

    // Verify board was deleted
    let response = server.get(&format!("/api/boards/{}", board_id)).await;
    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

// ============================================================================
// Permission Edge Cases
// ============================================================================

#[tokio::test]
async fn test_subop_can_manage_users_but_not_roles() {
    let (server, db) = create_test_server().await;

    // Create subop
    let subop_response = register_test_user(&server, "subop", "password123", "SubOp").await;
    let subop_id = get_user_id(&subop_response);
    set_user_role(&db, subop_id, Role::SubOp).await;

    // Create target user
    let target_response = register_test_user(&server, "target", "password123", "Target").await;
    let target_id = get_user_id(&target_response);

    // Re-login as subop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "subop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // SubOp can update user profile
    let response = server
        .put(&format!("/api/admin/users/{}", target_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "nickname": "Updated Nickname"
        }))
        .await;

    response.assert_status_ok();

    // SubOp cannot change roles
    let response = server
        .put(&format!("/api/admin/users/{}/role", target_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "role": "subop"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_sysop_cannot_demote_themselves() {
    let (server, db) = create_test_server().await;

    // Create sysop
    let sysop_response = register_test_user(&server, "sysop", "password123", "SysOp").await;
    let sysop_id = get_user_id(&sysop_response);
    set_user_role(&db, sysop_id, Role::SysOp).await;

    // Re-login as sysop
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "sysop",
            "password": "password123"
        }))
        .await;
    let access_token = get_access_token(&login_response.json::<Value>());

    // Try to demote self - should fail
    let response = server
        .put(&format!("/api/admin/users/{}/role", sysop_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "role": "member"
        }))
        .await;

    // This should either be forbidden or bad request
    assert!(
        response.status_code() == axum::http::StatusCode::FORBIDDEN
            || response.status_code() == axum::http::StatusCode::BAD_REQUEST
    );
}
