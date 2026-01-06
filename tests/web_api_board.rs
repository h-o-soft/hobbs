//! Web API Board Tests
//!
//! Integration tests for board endpoints.

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

/// Create a test board in the database.
async fn create_test_board(db: &Arc<Mutex<Database>>, name: &str, board_type: BoardType) -> i64 {
    let db = db.lock().await;
    let repo = BoardRepository::new(&*db);
    let new_board = NewBoard::new(name)
        .with_description(format!("{} board", name))
        .with_board_type(board_type)
        .with_min_read_role(Role::Guest)
        .with_min_write_role(Role::Member);
    repo.create(&new_board)
        .expect("Failed to create test board")
        .id
}

/// Create a test board that requires member role to read.
async fn create_members_only_board(db: &Arc<Mutex<Database>>, name: &str) -> i64 {
    let db = db.lock().await;
    let repo = BoardRepository::new(&*db);
    let new_board = NewBoard::new(name)
        .with_description(format!("{} - members only", name))
        .with_board_type(BoardType::Thread)
        .with_min_read_role(Role::Member)
        .with_min_write_role(Role::Member);
    repo.create(&new_board)
        .expect("Failed to create test board")
        .id
}

/// Make a user a sysop.
async fn make_user_sysop(db: &Arc<Mutex<Database>>, user_id: i64) {
    let db = db.lock().await;
    let repo = UserRepository::new(&*db);
    let update = UserUpdate {
        role: Some(Role::SysOp),
        ..Default::default()
    };
    repo.update(user_id, &update)
        .expect("Failed to update user role");
}

// ============================================================================
// List Boards Tests
// ============================================================================

#[tokio::test]
async fn test_list_boards_empty() {
    let (server, _db) = create_test_server().await;

    let response = server.get("/api/boards").await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_boards_with_boards() {
    let (server, db) = create_test_server().await;

    // Create some test boards
    create_test_board(&db, "General Discussion", BoardType::Thread).await;
    create_test_board(&db, "News", BoardType::Flat).await;

    let response = server.get("/api/boards").await;

    response.assert_status_ok();

    let body: Value = response.json();
    let boards = body["data"].as_array().unwrap();
    assert_eq!(boards.len(), 2);
}

#[tokio::test]
async fn test_list_boards_guest_cannot_see_members_only() {
    let (server, db) = create_test_server().await;

    // Create a public board and a members-only board
    create_test_board(&db, "Public", BoardType::Thread).await;
    create_members_only_board(&db, "Members Only").await;

    // Guest user (no auth) should only see the public board
    let response = server.get("/api/boards").await;

    response.assert_status_ok();

    let body: Value = response.json();
    let boards = body["data"].as_array().unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0]["name"], "Public");
}

#[tokio::test]
async fn test_list_boards_member_can_see_all() {
    let (server, db) = create_test_server().await;

    // Create boards
    create_test_board(&db, "Public", BoardType::Thread).await;
    create_members_only_board(&db, "Members Only").await;

    // Register a user
    let login_response = register_test_user(&server, "memberuser", "password123", "Member").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    // Member should see both boards
    let response = server
        .get("/api/boards")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    let boards = body["data"].as_array().unwrap();
    assert_eq!(boards.len(), 2);
}

// ============================================================================
// Get Board Tests
// ============================================================================

#[tokio::test]
async fn test_get_board_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    let response = server.get(&format!("/api/boards/{}", board_id)).await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["id"], board_id);
    assert_eq!(body["data"]["name"], "Test Board");
    assert_eq!(body["data"]["board_type"], "thread");
}

#[tokio::test]
async fn test_get_board_not_found() {
    let (server, _db) = create_test_server().await;

    let response = server.get("/api/boards/99999").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_board_access_denied_for_guest() {
    let (server, db) = create_test_server().await;

    let board_id = create_members_only_board(&db, "Members Only").await;

    let response = server.get(&format!("/api/boards/{}", board_id)).await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// Thread Operations Tests
// ============================================================================

#[tokio::test]
async fn test_create_thread_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // Register a user
    let login_response =
        register_test_user(&server, "threaduser", "password123", "Thread User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    // Create a thread
    let response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "title": "My First Thread",
            "body": "This is the first post content."
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["title"], "My First Thread");
    assert!(body["data"]["id"].is_number());
}

#[tokio::test]
async fn test_create_thread_unauthorized() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // Try to create thread without auth
    let response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .json(&json!({
            "title": "My Thread",
            "body": "Content"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_thread_board_not_found() {
    let (server, _db) = create_test_server().await;

    let login_response =
        register_test_user(&server, "threaduser", "password123", "Thread User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    let response = server
        .post("/api/boards/99999/threads")
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "title": "My Thread",
            "body": "Content"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_threads_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // Register and create a thread
    let login_response = register_test_user(&server, "listuser", "password123", "List User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    // Create a thread
    server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "title": "Thread 1",
            "body": "Content 1"
        }))
        .await
        .assert_status_ok();

    // List threads
    let response = server
        .get(&format!("/api/boards/{}/threads", board_id))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    let threads = body["data"].as_array().unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0]["title"], "Thread 1");
}

#[tokio::test]
async fn test_get_thread_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // Register and create a thread
    let login_response = register_test_user(&server, "getuser", "password123", "Get User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    let create_response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "title": "Test Thread",
            "body": "Test Content"
        }))
        .await;

    create_response.assert_status_ok();
    let thread_id = create_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Get thread
    let response = server.get(&format!("/api/threads/{}", thread_id)).await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["id"], thread_id);
    assert_eq!(body["data"]["title"], "Test Thread");
}

// ============================================================================
// Post Operations Tests
// ============================================================================

#[tokio::test]
async fn test_create_thread_post_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // Register and create a thread
    let login_response = register_test_user(&server, "postuser", "password123", "Post User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    let create_response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "title": "Test Thread",
            "body": "First Post"
        }))
        .await;

    let thread_id = create_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Create a reply
    let response = server
        .post(&format!("/api/threads/{}/posts", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "body": "This is a reply!"
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["body"], "This is a reply!");
}

#[tokio::test]
async fn test_list_thread_posts_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // Register and create a thread with posts
    let login_response =
        register_test_user(&server, "listpostuser", "password123", "List Post User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    let create_response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "title": "Test Thread",
            "body": "First Post"
        }))
        .await;

    let thread_id = create_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Create a reply
    server
        .post(&format!("/api/threads/{}/posts", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "body": "Second Post"
        }))
        .await
        .assert_status_ok();

    // List posts
    let response = server
        .get(&format!("/api/threads/{}/posts", thread_id))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    let posts = body["data"].as_array().unwrap();
    assert_eq!(posts.len(), 2); // First post + reply
}

// ============================================================================
// Flat Board Operations Tests
// ============================================================================

#[tokio::test]
async fn test_create_flat_post_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Flat Board", BoardType::Flat).await;

    // Register a user
    let login_response = register_test_user(&server, "flatuser", "password123", "Flat User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    // Create a flat post
    let response = server
        .post(&format!("/api/boards/{}/posts", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "title": "Flat Post Title",
            "body": "Flat post content here."
        }))
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["data"]["title"], "Flat Post Title");
    assert_eq!(body["data"]["body"], "Flat post content here.");
}

#[tokio::test]
async fn test_list_flat_posts_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Flat Board", BoardType::Flat).await;

    // Register and create flat posts
    let login_response =
        register_test_user(&server, "flatlistuser", "password123", "Flat List User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    // Create posts
    for i in 1..=3 {
        server
            .post(&format!("/api/boards/{}/posts", board_id))
            .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
            .json(&json!({
                "title": format!("Post {}", i),
                "body": format!("Content {}", i)
            }))
            .await
            .assert_status_ok();
    }

    // List posts
    let response = server.get(&format!("/api/boards/{}/posts", board_id)).await;

    response.assert_status_ok();

    let body: Value = response.json();
    let posts = body["data"].as_array().unwrap();
    assert_eq!(posts.len(), 3);
}

// ============================================================================
// Delete Post Tests
// ============================================================================

#[tokio::test]
async fn test_delete_own_post_success() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // Register and create a thread
    let login_response =
        register_test_user(&server, "deleteuser", "password123", "Delete User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    let create_response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "title": "Test Thread",
            "body": "First Post"
        }))
        .await;

    let thread_id = create_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Create a reply
    let post_response = server
        .post(&format!("/api/threads/{}/posts", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .json(&json!({
            "body": "To be deleted"
        }))
        .await;

    let post_id = post_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // Delete the post
    let response = server
        .delete(&format!("/api/posts/{}", post_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_delete_other_user_post_forbidden() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // User 1 creates a thread
    let login_response1 = register_test_user(&server, "user1", "password123", "User 1").await;
    let access_token1 = login_response1["data"]["access_token"].as_str().unwrap();

    let create_response = server
        .post(&format!("/api/boards/{}/threads", board_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token1))
        .json(&json!({
            "title": "User 1 Thread",
            "body": "User 1 Post"
        }))
        .await;

    let thread_id = create_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // User 1 creates a reply
    let post_response = server
        .post(&format!("/api/threads/{}/posts", thread_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token1))
        .json(&json!({
            "body": "User 1 Reply"
        }))
        .await;

    let post_id = post_response.json::<Value>()["data"]["id"]
        .as_i64()
        .unwrap();

    // User 2 tries to delete User 1's post
    let login_response2 = register_test_user(&server, "user2", "password123", "User 2").await;
    let access_token2 = login_response2["data"]["access_token"].as_str().unwrap();

    let response = server
        .delete(&format!("/api/posts/{}", post_id))
        .add_header(AUTHORIZATION, format!("Bearer {}", access_token2))
        .await;

    response.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// Pagination Tests
// ============================================================================

#[tokio::test]
async fn test_threads_pagination() {
    let (server, db) = create_test_server().await;

    let board_id = create_test_board(&db, "Test Board", BoardType::Thread).await;

    // Register and create multiple threads
    let login_response =
        register_test_user(&server, "paginationuser", "password123", "Pagination User").await;
    let access_token = login_response["data"]["access_token"].as_str().unwrap();

    // Create 25 threads
    for i in 1..=25 {
        server
            .post(&format!("/api/boards/{}/threads", board_id))
            .add_header(AUTHORIZATION, format!("Bearer {}", access_token))
            .json(&json!({
                "title": format!("Thread {}", i),
                "body": format!("Content {}", i)
            }))
            .await
            .assert_status_ok();
    }

    // First page (default per_page = 20)
    let response = server
        .get(&format!("/api/boards/{}/threads?page=1", board_id))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 20);
    assert_eq!(body["meta"]["total"], 25);
    assert_eq!(body["meta"]["page"], 1);

    // Second page
    let response = server
        .get(&format!("/api/boards/{}/threads?page=2", board_id))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 5);
    assert_eq!(body["meta"]["page"], 2);
}
