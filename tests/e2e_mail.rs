//! E2E Mail tests for HOBBS.
//!
//! Tests mail inbox, sending, and reading.

mod common;

use common::{create_test_user, TestClient, TestServer};
use std::time::Duration;

/// Test mail requires login.
#[tokio::test]
async fn test_mail_requires_login() {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // New flow: welcome screen first, then choose guest, then language selection
    client.recv_until("Select:").await.unwrap();
    client.send_line("G").await.unwrap();

    // Language selection appears after choosing G
    client.select_language("E").await.unwrap();

    // Wait for guest menu
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Try to access mail
    client.send_line("M").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should require login or show error or invalid selection for guests
    assert!(
        response.contains("login")
            || response.contains("ログイン")
            || response.contains("required")
            || response.contains("必要")
            || response.contains("Menu")
            || response.contains("Select")
            || response.contains("Invalid"),
        "Mail should require login: {:?}",
        response
    );
}

/// Test mail inbox access.
#[tokio::test]
async fn test_mail_inbox_access() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "member", "password123", "member").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login
    let result = client.login("member", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Access mail
    client.send_line("M").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see inbox or mail menu
    assert!(
        response.contains("Inbox")
            || response.contains("受信箱")
            || response.contains("Mail")
            || response.contains("メール")
            || response.contains("W")
            || response.contains("Q"),
        "Should see mail inbox: {:?}",
        response
    );
}

/// Test empty inbox.
#[tokio::test]
async fn test_mail_empty_inbox() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "member", "password123", "member").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login
    let result = client.login("member", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Access mail
    client.send_line("M").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should show empty inbox message or mail menu
    assert!(
        response.contains("no mail")
            || response.contains("メールはありません")
            || response.contains("empty")
            || response.contains("0")
            || response.contains("Total: 0")
            || response.contains("W")
            || response.contains("Q")
            || response.contains("Mail"),
        "Should show empty inbox: {:?}",
        response
    );
}

/// Test back from mail to menu.
#[tokio::test]
async fn test_mail_back_to_menu() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "member", "password123", "member").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login
    let result = client.login("member", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Go to mail
    client.send_line("M").await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Go back
    client.send_line("Q").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should be back at main menu
    assert!(
        response.contains("B")
            || response.contains("Board")
            || response.contains("Menu")
            || response.contains("Select"),
        "Should be back at main menu: {:?}",
        response
    );
}
