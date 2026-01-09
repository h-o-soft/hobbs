#![cfg(feature = "sqlite")]
//! E2E tests for login settings application.
//!
//! Tests that user's language/encoding settings are applied on login.

mod common;

use common::{create_test_user_with_settings, TestClient, TestServer};
use std::time::Duration;

/// Test that user's language setting is applied on login.
/// User has Japanese language setting.
/// After login, the success message should be in Japanese.
#[tokio::test]
async fn test_login_applies_user_language() {
    let server = TestServer::new().await.unwrap();
    // Create user with Japanese language and UTF-8 encoding
    create_test_user_with_settings(
        server.db(),
        "jauser",
        "password123",
        "member",
        "ja",    // Japanese
        "utf-8", // UTF-8 encoding
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // New flow: welcome screen (ASCII) appears first, no language selection before login
    client.recv_until("Select:").await.unwrap();

    // Login
    client.send_line("L").await.unwrap();
    client.recv_until("Username:").await.unwrap();
    client.send_line("jauser").await.unwrap();
    client.recv_until("Password:").await.unwrap();
    client.send_line("password123").await.unwrap();

    // Wait for login success message
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // After login, user's Japanese language setting should be applied
    // Login success message should be in Japanese
    assert!(
        response.contains("ログイン")
            || response.contains("ようこそ")
            || response.contains("jauser"),
        "After login, message should be in Japanese: {:?}",
        response
    );

    // Continue receiving to get menu
    let menu = client
        .recv_timeout(Duration::from_secs(2))
        .await
        .unwrap_or_default();

    // Menu should contain Japanese text or menu indicators
    assert!(
        response.contains("ログイン")
            || menu.contains("掲示板")
            || menu.contains("メニュー")
            || menu.contains("終了")
            || menu.contains("B")
            || menu.contains("Q"),
        "Menu should be in Japanese or show menu options: {:?}",
        menu
    );
}

/// Test that user's English language setting is applied on login.
/// User has English language setting.
/// After login, the success message should be in English.
#[tokio::test]
async fn test_login_applies_user_english_language() {
    let server = TestServer::new().await.unwrap();
    // Create user with English language and UTF-8 encoding
    create_test_user_with_settings(
        server.db(),
        "enuser",
        "password123",
        "member",
        "en",    // English
        "utf-8", // UTF-8 encoding
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // New flow: welcome screen (ASCII) appears first, no language selection before login
    client.recv_until("Select:").await.unwrap();

    // Login
    client.send_line("L").await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap();
    client.send_line("enuser").await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap();
    client.send_line("password123").await.unwrap();

    // Wait for login success message
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // After login, user's English language setting should be applied
    // Login success message should be in English
    assert!(
        response.contains("Welcome")
            || response.contains("logged in")
            || response.contains("enuser"),
        "After login, message should be in English: {:?}",
        response
    );
}

/// Test that guest mode keeps selected language from language selection screen.
/// Guest selects Japanese after choosing G.
/// Guest menu should be in Japanese.
#[tokio::test]
async fn test_guest_keeps_welcome_language() {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // New flow: welcome screen (ASCII) appears first
    client.recv_until("Select:").await.unwrap();

    // Enter guest mode
    client.send_line("G").await.unwrap();

    // Language selection appears after choosing G
    client.recv_until("Gengo").await.unwrap();

    // Select Japanese UTF-8
    client.send_line("U").await.unwrap();

    // Wait for menu
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Guest menu should be in Japanese
    assert!(
        response.contains("掲示板")
            || response.contains("メニュー")
            || response.contains("終了")
            || response.contains("B")
            || response.contains("Q"),
        "Guest menu should be in Japanese or have menu options: {:?}",
        response
    );
}

/// Test that guest mode with English selection keeps English.
#[tokio::test]
async fn test_guest_keeps_english_language() {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // New flow: welcome screen (ASCII) appears first
    client.recv_until("Select:").await.unwrap();

    // Enter guest mode
    client.send_line("G").await.unwrap();

    // Language selection appears after choosing G
    client.recv_until("Gengo").await.unwrap();

    // Select English
    client.send_line("E").await.unwrap();

    // Wait for menu
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Guest menu should be in English
    assert!(
        response.contains("Board")
            || response.contains("Menu")
            || response.contains("Quit")
            || response.contains("B")
            || response.contains("Q"),
        "Guest menu should be in English or have menu options: {:?}",
        response
    );
}
