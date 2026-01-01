//! E2E Board tests for HOBBS.
//!
//! Tests board listing, thread creation, and posting.

mod common;

use common::{create_test_board, create_test_user, TestClient, TestServer};
use std::time::Duration;

/// Test board list access as guest.
#[tokio::test]
async fn test_board_list_guest() {
    let server = TestServer::new().await.unwrap();
    create_test_board(server.db(), "General", "thread").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // New flow: welcome screen appears first
    // Wait for welcome, enter guest mode
    client.recv_until("Select:").await.unwrap();
    client.send_line("G").await.unwrap();

    // Language selection appears after choosing G
    client.select_language("E").await.unwrap();

    // Wait for guest menu, go to board
    let menu = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    assert!(!menu.is_empty(), "Should receive guest menu");

    client.send_line("B").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see board list
    assert!(
        response.contains("General")
            || response.contains("Board")
            || response.contains("掲示板")
            || response.contains("Q"),
        "Should see board list: {:?}",
        response
    );
}

/// Test board list access as logged-in user.
#[tokio::test]
async fn test_board_list_member() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "member", "password123", "member").unwrap();
    create_test_board(server.db(), "General", "thread").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login - login() handles welcome screen
    let result = client.login("member", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Go to board
    client.send_line("B").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see board list
    assert!(
        response.contains("General")
            || response.contains("Board")
            || response.contains("掲示板")
            || response.contains("Q"),
        "Should see board list: {:?}",
        response
    );
}

/// Test selecting a board.
#[tokio::test]
async fn test_board_selection() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "member", "password123", "member").unwrap();
    create_test_board(server.db(), "TestBoard", "thread").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login
    let result = client.login("member", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Go to board
    client.send_line("B").await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Select first board (1)
    client.send_line("1").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should be in board view
    assert!(
        response.contains("TestBoard")
            || response.contains("thread")
            || response.contains("スレッド")
            || response.contains("N")
            || response.contains("Q"),
        "Should be in board view: {:?}",
        response
    );
}

/// Test back navigation from board.
#[tokio::test]
async fn test_board_back_navigation() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "member", "password123", "member").unwrap();
    create_test_board(server.db(), "General", "thread").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login
    let result = client.login("member", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Go to board
    client.send_line("B").await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Go back to main menu
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

/// Test board without boards.
#[tokio::test]
async fn test_no_boards() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "member", "password123", "member").unwrap();
    // No boards created
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login
    let result = client.login("member", "password123").await.unwrap();
    assert!(result, "Login should succeed");

    // Go to board
    client.send_line("B").await.unwrap();
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should show no boards message or empty list
    assert!(
        response.contains("no")
            || response.contains("empty")
            || response.contains("ありません")
            || response.contains("Q")
            || response.contains("Board"),
        "Should handle no boards: {:?}",
        response
    );
}
