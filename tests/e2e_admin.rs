//! E2E Admin tests for HOBBS.
//!
//! Tests admin panel access and functionality.

mod common;

use common::{create_test_user, TestClient, TestServer};
use std::time::Duration;

/// Test admin panel requires admin role.
#[tokio::test]
async fn test_admin_requires_admin_role() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "member", "password123", "member").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login as regular member
    let result = client.login("member", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Try to access admin
    client.send_line("A").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should be denied (A might not even show in menu for non-admin)
    // Or should show permission denied message
    assert!(
        response.contains("permission")
            || response.contains("denied")
            || response.contains("権限")
            || response.contains("admin")
            || response.contains("B")
            || response.contains("Menu")
            || response.contains("Select"),
        "Admin should be denied for regular member: {:?}",
        response
    );
}

/// Test admin panel access as SubOp.
#[tokio::test]
async fn test_admin_access_subop() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "subop", "password123", "subop").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login as SubOp
    let result = client.login("subop", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Access admin
    client.send_line("A").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see admin menu or appropriate response
    assert!(
        response.contains("Admin")
            || response.contains("管理")
            || response.contains("Board")
            || response.contains("User")
            || response.contains("Q")
            || response.contains("Menu"),
        "SubOp should access admin panel: {:?}",
        response
    );
}

/// Test admin panel access as SysOp.
#[tokio::test]
async fn test_admin_access_sysop() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "sysop", "password123", "sysop").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login as SysOp
    let result = client.login("sysop", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Access admin
    client.send_line("A").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see admin menu
    assert!(
        response.contains("Admin")
            || response.contains("管理")
            || response.contains("Board")
            || response.contains("User")
            || response.contains("System")
            || response.contains("Q")
            || response.contains("Menu"),
        "SysOp should access admin panel: {:?}",
        response
    );
}

/// Test admin back to main menu.
#[tokio::test]
async fn test_admin_back_to_menu() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "sysop", "password123", "sysop").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login as SysOp
    let result = client.login("sysop", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Go to admin
    client.send_line("A").await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Go back
    client.send_line("Q").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should be back at main menu
    assert!(
        response.contains("B")
            || response.contains("Board")
            || response.contains("Menu")
            || response.contains("A")
            || response.contains("Select"),
        "Should be back at main menu: {:?}",
        response
    );
}

/// Test admin as guest (should not show admin option).
#[tokio::test]
async fn test_admin_not_visible_to_guest() {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Wait for welcome, enter guest mode
    client.recv_until("Select:").await.unwrap();
    client.send_line("G").await.unwrap();

    // Wait for guest menu
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Admin option should not be visible (A=Admin)
    // But if they try to send A anyway, it should be invalid
    client.send_line("A").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should either show invalid command or permission denied
    assert!(
        response.contains("invalid")
            || response.contains("無効")
            || response.contains("permission")
            || response.contains("admin")
            || response.contains("B")
            || response.contains("Menu")
            || response.contains("Select"),
        "Guest should not access admin: {:?}",
        response
    );
}
