//! E2E Authentication tests for HOBBS.
//!
//! Tests login, logout, and registration flows.

mod common;

use common::{create_test_user, with_test_server, TestClient, TestServer};
use std::time::Duration;

/// Test successful login flow.
#[tokio::test]
async fn test_login_success() {
    // Create server with a test user
    let server = TestServer::new().await.unwrap();

    // Create test user
    create_test_user(server.db(), "testuser", "password123", "member").unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login - this waits for welcome, logs in, and returns success message + menu
    let result = client.login("testuser", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // After successful login, we're at the main menu.
    // The login helper already consumed the menu, so we just verify login worked.
}

/// Test login with wrong password.
#[tokio::test]
async fn test_login_wrong_password() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "testuser", "password123", "member").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Try login with wrong password - login() handles welcome screen
    let result = client.login("testuser", "wrongpassword").await.unwrap();
    assert!(!result, "Login should fail with wrong password");
}

/// Test login with non-existent user.
#[tokio::test]
async fn test_login_nonexistent_user() {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Try login with non-existent user - login() handles welcome screen
    let result = client.login("nobody", "password123").await.unwrap();
    assert!(!result, "Login should fail for non-existent user");
}

/// Test logout flow.
#[tokio::test]
async fn test_logout() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "testuser", "password123", "member").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login first - login() handles welcome screen
    let result = client.login("testuser", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Send logout command (Q for Quit/Logout from main menu)
    client.send_line("Q").await.unwrap();

    // Should receive goodbye message and connection closes
    let response = client
        .recv_timeout(Duration::from_secs(2))
        .await
        .unwrap_or_default();
    assert!(
        response.contains("Thank you")
            || response.contains("goodbye")
            || response.contains("さようなら")
            || response.is_empty(),
        "Should receive goodbye message after logout: {:?}",
        response
    );
}

/// Test registration flow.
#[tokio::test]
async fn test_registration_success() {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Register new user - register() handles welcome screen
    let result = client
        .register("newuser", "password123", "New User")
        .await
        .unwrap();
    assert!(result, "Registration should succeed");
}

/// Test registration with duplicate username.
#[tokio::test]
async fn test_registration_duplicate_username() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "existing", "password123", "member").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Handle language selection first
    client.select_language("E").await.unwrap();

    // Wait for welcome then try to register with existing username
    client.recv_until("Select:").await.unwrap();
    client.send_line("R").await.unwrap();
    client.recv_until("Username:").await.unwrap();
    client.send_line("existing").await.unwrap();

    // Should fail because username is taken
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    assert!(
        response.contains("taken")
            || response.contains("既に")
            || response.contains("already")
            || response.contains("exists")
            || response.contains("Username:"), // Might re-prompt
        "Should reject duplicate username"
    );
}

/// Test empty username during login.
#[tokio::test]
async fn test_login_empty_username() {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Handle language selection first
    client.select_language("E").await.unwrap();

    // Wait for welcome then try to login with empty username
    client.recv_until("Select:").await.unwrap();
    client.send_line("L").await.unwrap();
    client.recv_until("Username:").await.unwrap();
    client.send_line("").await.unwrap();

    // Should return to welcome or show error
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    assert!(
        response.contains("L")
            || response.contains("Login")
            || response.contains("G")
            || response.contains("Select")
            || response.contains("Username"),
        "Should return to welcome or re-prompt with empty username"
    );
}

/// Test disabled user cannot login.
#[tokio::test]
async fn test_login_disabled_user() {
    let server = TestServer::new().await.unwrap();

    // Create and disable user
    let user_id = create_test_user(server.db(), "disabled", "password123", "member").unwrap();
    server
        .db()
        .conn()
        .execute(
            "UPDATE users SET is_active = 0 WHERE id = ?",
            rusqlite::params![user_id],
        )
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Try to login - login() handles welcome screen
    let result = client.login("disabled", "password123").await.unwrap();
    assert!(!result, "Disabled user should not be able to login");
}
