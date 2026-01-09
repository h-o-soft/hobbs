//! Web API File/Folder Tests
//!
//! Integration tests for file and folder endpoints.

use axum::http::header::AUTHORIZATION;
use axum_test::TestServer;
use hobbs::config::WebConfig;
use hobbs::db::{Role, UserRepository, UserUpdate};
use hobbs::file::{FolderRepository, NewFolder};
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
async fn set_user_role(db: &Arc<Database>, user_id: i64, role: Role) {
    let repo = UserRepository::new(db.pool());
    let update = UserUpdate {
        role: Some(role),
        ..Default::default()
    };
    repo.update(user_id, &update)
        .await
        .expect("Failed to update user role");
}

/// Create a test folder in the database.
async fn create_test_folder(
    db: &Arc<Database>,
    name: &str,
    permission: Role,
    upload_perm: Role,
) -> i64 {
    let folder_repo = FolderRepository::new(db.pool());
    let new_folder = NewFolder::new(name)
        .with_description(format!("{} folder", name))
        .with_permission(permission)
        .with_upload_perm(upload_perm);
    folder_repo
        .create(&new_folder)
        .await
        .expect("Failed to create test folder")
        .id
}

// ============================================================================
// Folder List Tests
// ============================================================================

#[tokio::test]
async fn test_list_folders_empty() {
    let (server, _db) = create_test_server().await;

    // Register user
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // List folders
    let response = server
        .get("/api/folders")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_folders_with_folders() {
    let (server, db) = create_test_server().await;

    // Create folders accessible by member role
    create_test_folder(&db, "Public Files", Role::Member, Role::SubOp).await;
    create_test_folder(&db, "Downloads", Role::Member, Role::SubOp).await;

    // Register user
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // List folders
    let response = server
        .get("/api/folders")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    let folders = body["data"].as_array().unwrap();
    assert_eq!(folders.len(), 2);
}

#[tokio::test]
async fn test_list_folders_member_cannot_see_subop_only() {
    let (server, db) = create_test_server().await;

    // Create folders with different permissions
    create_test_folder(&db, "Member Folder", Role::Member, Role::Member).await;
    create_test_folder(&db, "SubOp Only", Role::SubOp, Role::SubOp).await;

    // Register user (member role)
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // List folders - member should only see member-accessible folder
    let response = server
        .get("/api/folders")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    let folders = body["data"].as_array().unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0]["name"], "Member Folder");
}

#[tokio::test]
async fn test_list_folders_subop_can_see_all() {
    let (server, db) = create_test_server().await;

    // Create folders with different permissions
    create_test_folder(&db, "Member Folder", Role::Member, Role::Member).await;
    create_test_folder(&db, "SubOp Only", Role::SubOp, Role::SubOp).await;

    // Register user and make them subop
    let user_response = register_test_user(&server, "subop", "password123", "SubOp").await;
    let user_id = get_user_id(&user_response);
    set_user_role(&db, user_id, Role::SubOp).await;

    // Re-login
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "subop",
            "password": "password123"
        }))
        .await;
    let user_token = get_access_token(&login_response.json::<Value>());

    // List folders - subop should see both
    let response = server
        .get("/api/folders")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    let folders = body["data"].as_array().unwrap();
    assert_eq!(folders.len(), 2);
}

#[tokio::test]
async fn test_list_folders_unauthorized() {
    let (server, _db) = create_test_server().await;

    let response = server.get("/api/folders").await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Get Folder Tests
// ============================================================================

#[tokio::test]
async fn test_get_folder_success() {
    let (server, db) = create_test_server().await;

    let folder_id = create_test_folder(&db, "Test Folder", Role::Member, Role::SubOp).await;

    // Register user
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // Get folder
    let response = server
        .get(&format!("/api/folders/{}", folder_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["id"], folder_id);
    assert_eq!(body["data"]["name"], "Test Folder");
    assert_eq!(body["data"]["can_read"], true);
    assert_eq!(body["data"]["can_upload"], false); // member cannot upload to subop folder
}

#[tokio::test]
async fn test_get_folder_not_found() {
    let (server, _db) = create_test_server().await;

    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    let response = server
        .get("/api/folders/99999")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_folder_access_denied() {
    let (server, db) = create_test_server().await;

    let folder_id = create_test_folder(&db, "SubOp Only", Role::SubOp, Role::SubOp).await;

    // Register regular member
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // Try to access SubOp-only folder as member
    let response = server
        .get(&format!("/api/folders/{}", folder_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// File List Tests
// ============================================================================

#[tokio::test]
async fn test_list_files_empty_folder() {
    let (server, db) = create_test_server().await;

    let folder_id = create_test_folder(&db, "Empty Folder", Role::Member, Role::SubOp).await;

    // Register user
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // List files
    let response = server
        .get(&format!("/api/folders/{}/files", folder_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_files_folder_not_found() {
    let (server, _db) = create_test_server().await;

    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    let response = server
        .get("/api/folders/99999/files")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_files_access_denied() {
    let (server, db) = create_test_server().await;

    let folder_id = create_test_folder(&db, "SubOp Only", Role::SubOp, Role::SubOp).await;

    // Register regular member
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    let response = server
        .get(&format!("/api/folders/{}/files", folder_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// Upload Endpoint Tests (Without actual file upload - checking auth/permissions)
// ============================================================================

#[tokio::test]
async fn test_upload_endpoint_unauthorized() {
    let (server, db) = create_test_server().await;

    let folder_id = create_test_folder(&db, "Upload Folder", Role::Member, Role::Member).await;

    // Try to access upload endpoint without auth
    let response = server
        .post(&format!("/api/folders/{}/files", folder_id))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Download Tests
// ============================================================================

#[tokio::test]
async fn test_download_file_no_storage() {
    let (server, _db) = create_test_server().await;

    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // Without storage configured, download should fail with internal error
    let response = server
        .get("/api/files/99999/download")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_download_file_unauthorized() {
    let (server, _db) = create_test_server().await;

    let response = server.get("/api/files/1/download").await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_file_info_not_found() {
    let (server, _db) = create_test_server().await;

    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    let response = server
        .get("/api/files/99999")
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

// ============================================================================
// Upload Permission Tests
// ============================================================================

#[tokio::test]
async fn test_upload_permission_check() {
    let (server, db) = create_test_server().await;

    // Create folder that requires SubOp to upload
    let folder_id = create_test_folder(&db, "SubOp Upload", Role::Member, Role::SubOp).await;

    // Register regular member
    let user_response = register_test_user(&server, "user", "password123", "User").await;
    let user_token = get_access_token(&user_response);

    // Check folder details - should show can_upload = false
    let response = server
        .get(&format!("/api/folders/{}", folder_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["can_upload"], false);
}

#[tokio::test]
async fn test_subop_can_upload_to_subop_folder() {
    let (server, db) = create_test_server().await;

    // Create folder that requires SubOp to upload
    let folder_id = create_test_folder(&db, "SubOp Upload", Role::Member, Role::SubOp).await;

    // Register user and make them subop
    let user_response = register_test_user(&server, "subop", "password123", "SubOp").await;
    let user_id = get_user_id(&user_response);
    set_user_role(&db, user_id, Role::SubOp).await;

    // Re-login
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "subop",
            "password": "password123"
        }))
        .await;
    let user_token = get_access_token(&login_response.json::<Value>());

    // Check folder details - should show can_upload = true
    let response = server
        .get(&format!("/api/folders/{}", folder_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", user_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["can_upload"], true);
}
