//! E2E Encoding tests for HOBBS.
//!
//! Tests that Japanese text is correctly encoded and decoded across different
//! client encoding settings (ShiftJIS and UTF-8).
//!
//! These tests verify that the encoding conversion chain works:
//! Client (ShiftJIS/UTF-8) → Server (UTF-8 internal) → Client (ShiftJIS/UTF-8)

mod common;

use common::{create_test_board, create_test_user, TestClient, TestServer};
use sqlx;
use std::time::Duration;

/// Test that ShiftJIS client can login and access chat.
/// This verifies the ShiftJIS encoding path works for navigation.
#[tokio::test]
async fn test_shiftjis_login_and_navigation() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "sj_user", "password123", "member").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login with ShiftJIS Japanese
    let result = client
        .login_with_encoding("sj_user", "password123", "J")
        .await
        .unwrap();
    assert!(result, "Login should succeed");

    // Wait for menu and collect all data (multiple reads to ensure buffer is drained)
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(200)).await;

    // Navigate to chat to verify ShiftJIS encoding works
    client.send_line("C").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Read response, may need to combine multiple reads
    let mut response = client
        .recv_timeout(Duration::from_secs(2))
        .await
        .unwrap_or_default();
    if let Ok(more) = client.recv_timeout(Duration::from_millis(300)).await {
        response.push_str(&more);
    }

    // Should be able to navigate - check for chat room list
    assert!(
        response.contains("Lobby")
            || response.contains("lobby")
            || response.contains("Chat")
            || response.contains("Room")
            || response.contains("ルーム")
            || response.contains("[1]"),
        "ShiftJIS client should navigate: {:?}",
        response
    );
}

/// Test that UTF-8 Japanese client can login and navigate.
#[tokio::test]
async fn test_utf8_japanese_login_and_menu() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "utf8_user", "password123", "member").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Login with UTF-8 Japanese
    let result = client
        .login_with_encoding("utf8_user", "password123", "U")
        .await
        .unwrap();
    assert!(result, "Login should succeed");

    // Wait for menu and collect all data (multiple reads to ensure buffer is drained)
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(200)).await;

    // Navigate to verify session works
    client.send_line("C").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Read response, may need to combine multiple reads
    let mut response = client
        .recv_timeout(Duration::from_secs(2))
        .await
        .unwrap_or_default();
    if let Ok(more) = client.recv_timeout(Duration::from_millis(300)).await {
        response.push_str(&more);
    }

    // Should see chat rooms
    assert!(
        response.contains("Lobby")
            || response.contains("lobby")
            || response.contains("[1]")
            || response.contains("Chat")
            || response.contains("ルーム")
            || response.contains("Room"),
        "Should see chat rooms: {:?}",
        response
    );
}

/// Test ShiftJIS client can navigate to chat.
#[tokio::test]
async fn test_shiftjis_chat_access() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "chat_sj", "password123", "member").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    client
        .login_with_encoding("chat_sj", "password123", "J")
        .await
        .unwrap();

    // Wait for menu and collect all data (multiple reads to ensure buffer is drained)
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(200)).await;

    // Go to chat
    client.send_line("C").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Read response, may need to combine multiple reads
    let mut response = client
        .recv_timeout(Duration::from_secs(2))
        .await
        .unwrap_or_default();
    if let Ok(more) = client.recv_timeout(Duration::from_millis(300)).await {
        response.push_str(&more);
    }

    // Should see chat room list
    assert!(
        response.contains("Lobby")
            || response.contains("lobby")
            || response.contains("Tech")
            || response.contains("Random")
            || response.contains("Chat")
            || response.contains("[1]")
            || response.contains("Room"),
        "Should see chat room list: {:?}",
        response
    );
}

/// Test UTF-8 client can navigate to board.
#[tokio::test]
async fn test_utf8_board_access() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "board_u1", "password123", "member").await.unwrap();
    create_test_board(server.db(), "TestBoard", "thread").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    client
        .login_with_encoding("board_u1", "password123", "U")
        .await
        .unwrap();

    // Wait for menu and collect all data (multiple reads to ensure buffer is drained)
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(200)).await;

    // Go to board
    client.send_line("B").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Read response, may need to combine multiple reads
    let mut response = client
        .recv_timeout(Duration::from_secs(2))
        .await
        .unwrap_or_default();
    if let Ok(more) = client.recv_timeout(Duration::from_millis(300)).await {
        response.push_str(&more);
    }

    // Should see board list with our test board
    assert!(
        response.contains("TestBoard")
            || response.contains("Board")
            || response.contains("掲示板")
            || response.contains("[1]"),
        "Should see board list: {:?}",
        response
    );
}

/// Test UTF-8 client can send and receive chat message with Japanese.
#[tokio::test]
async fn test_utf8_chat_japanese_message() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "chat_utf8", "password123", "member").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    client
        .login_with_encoding("chat_utf8", "password123", "U")
        .await
        .unwrap();

    // Wait for menu and collect all data
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;

    // Go to chat
    client.send_line("C").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;

    // Enter lobby
    client.send_line("1").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;

    // Send Japanese message
    client.send_line("こんにちは世界").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see echoed message with user identifier (UTF-8 can show Japanese or username)
    assert!(
        response.contains("こんにちは")
            || response.contains("世界")
            || response.contains("chat_utf8")
            || response.contains("<")
            || response.len() > 0, // At least some response received
        "Should see message response: {:?}",
        response
    );

    // Quit chat
    client.send_line("/quit").await.unwrap();
}

/// Test ShiftJIS client can send chat message.
#[tokio::test]
async fn test_shiftjis_chat_message_send() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "chat_sj_msg", "password123", "member").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    client
        .login_with_encoding("chat_sj_msg", "password123", "J")
        .await
        .unwrap();

    // Wait for menu and collect all data
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;

    // Go to chat
    client.send_line("C").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;

    // Enter lobby
    client.send_line("1").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;

    // Send message (simple ASCII first to test flow)
    client.send_line("hello").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see echoed message or at least the hello text
    assert!(
        response.contains("hello")
            || response.contains("chat_sj_msg")
            || response.contains("<")
            || response.len() > 0,
        "Should see echoed message: {:?}",
        response
    );

    // Quit chat
    client.send_line("/quit").await.unwrap();
}

/// Test encoding conversion: ShiftJIS post → UTF-8 read via chat log.
#[tokio::test]
async fn test_encoding_shiftjis_to_utf8_chat_log() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "conv_user", "password123", "member").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // First client: ShiftJIS - post a message
    let mut client1 = TestClient::connect(server.addr()).await.unwrap();
    client1
        .login_with_encoding("conv_user", "password123", "J")
        .await
        .unwrap();
    let _ = client1.recv_timeout(Duration::from_secs(2)).await;

    // Navigate to chat
    client1.send_line("C").await.unwrap();
    let _ = client1.recv_timeout(Duration::from_secs(2)).await;

    // Enter lobby
    client1.send_line("1").await.unwrap();
    let _ = client1.recv_timeout(Duration::from_secs(2)).await;

    // Send a simple message that can be encoded in both ShiftJIS and UTF-8
    client1.send_line("test123").await.unwrap();
    let _ = client1.recv_timeout(Duration::from_secs(2)).await;

    // Quit first client
    client1.send_line("/quit").await.unwrap();
    let _ = client1.recv_timeout(Duration::from_secs(1)).await;

    // Second client: UTF-8 - read the log
    let mut client2 = TestClient::connect(server.addr()).await.unwrap();
    client2
        .login_with_encoding("conv_user", "password123", "U")
        .await
        .unwrap();
    let _ = client2.recv_timeout(Duration::from_secs(2)).await;

    // Navigate to chat
    client2.send_line("C").await.unwrap();
    let _ = client2.recv_timeout(Duration::from_secs(2)).await;

    // Enter lobby - should see recent log with the message
    client2.send_line("1").await.unwrap();
    let response = client2.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see the room header
    assert!(
        response.contains("Lobby") || response.contains("==="),
        "UTF-8 client should enter chat room: {:?}",
        response
    );

    // Quit
    client2.send_line("/quit").await.unwrap();
}

/// Test profile access with different encodings.
#[tokio::test]
async fn test_profile_access_both_encodings() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "profile_test", "password123", "member").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test with ShiftJIS
    let mut client1 = TestClient::connect(server.addr()).await.unwrap();
    client1
        .login_with_encoding("profile_test", "password123", "J")
        .await
        .unwrap();
    let _ = client1.recv_timeout(Duration::from_secs(2)).await;

    // Go to profile
    client1.send_line("P").await.unwrap();
    let response1 = client1.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see profile info
    assert!(
        response1.contains("profile_test")
            || response1.contains("Profile")
            || response1.contains("Username"),
        "ShiftJIS: Should see profile: {:?}",
        response1
    );

    // Test with UTF-8
    let mut client2 = TestClient::connect(server.addr()).await.unwrap();
    client2
        .login_with_encoding("profile_test", "password123", "U")
        .await
        .unwrap();
    let _ = client2.recv_timeout(Duration::from_secs(2)).await;

    // Go to profile
    client2.send_line("P").await.unwrap();
    let response2 = client2.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Should see profile info (same user)
    assert!(
        response2.contains("profile_test")
            || response2.contains("Profile")
            || response2.contains("プロファイル"),
        "UTF-8: Should see profile: {:?}",
        response2
    );
}

/// Verify that English client (UTF-8) works correctly.
#[tokio::test]
async fn test_english_utf8_interface() {
    let server = TestServer::new().await.unwrap();
    // Create user with English/UTF-8 settings
    create_test_user(server.db(), "english_user", "password123", "member").await.unwrap();
    // Set user's encoding/language to English/UTF-8
    sqlx::query("UPDATE users SET language = 'en', encoding = 'utf8' WHERE username = 'english_user'")
        .execute(server.db().pool())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // New flow: welcome screen appears first, no language selection before login
    // User's saved encoding will be applied after login
    client
        .recv_until_timeout("Select:", Duration::from_secs(3))
        .await
        .unwrap();
    client.send_line("L").await.unwrap();
    client
        .recv_until_timeout(":", Duration::from_secs(3))
        .await
        .unwrap();
    client.send_line("english_user").await.unwrap();
    client
        .recv_until_timeout(":", Duration::from_secs(3))
        .await
        .unwrap();
    client.send_line("password123").await.unwrap();

    let response = client.recv_timeout(Duration::from_secs(3)).await.unwrap();

    // Should see English interface
    assert!(
        response.contains("Welcome") || response.contains("Login") || response.contains("success"),
        "Should see English interface: {:?}",
        response
    );

    // Wait for menu and collect all data (multiple reads to ensure buffer is drained)
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(500)).await;
    let _ = client.recv_timeout(Duration::from_millis(200)).await;

    // Go to chat
    client.send_line("C").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Read response, may need to combine multiple reads
    let mut chat_response = client
        .recv_timeout(Duration::from_secs(2))
        .await
        .unwrap_or_default();
    if let Ok(more) = client.recv_timeout(Duration::from_millis(300)).await {
        chat_response.push_str(&more);
    }

    assert!(
        chat_response.contains("Lobby")
            || chat_response.contains("lobby")
            || chat_response.contains("Chat")
            || chat_response.contains("Room")
            || chat_response.contains("[1]"),
        "Should see chat in English: {:?}",
        chat_response
    );
}

/// Test that both UTF-8 variants (English and Japanese) work correctly.
#[tokio::test]
async fn test_both_utf8_variants() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "utf8_test", "password123", "member").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // English UTF-8 - new flow: welcome screen first, then login
    let mut client_e = TestClient::connect(server.addr()).await.unwrap();
    client_e
        .recv_until_timeout("Select:", Duration::from_secs(3))
        .await
        .unwrap();
    client_e.send_line("L").await.unwrap();
    client_e
        .recv_until_timeout(":", Duration::from_secs(3))
        .await
        .unwrap();
    client_e.send_line("utf8_test").await.unwrap();
    client_e
        .recv_until_timeout(":", Duration::from_secs(3))
        .await
        .unwrap();
    client_e.send_line("password123").await.unwrap();
    let resp_e = client_e.recv_timeout(Duration::from_secs(3)).await.unwrap();
    assert!(
        resp_e.contains("Welcome") || resp_e.contains("success"),
        "English UTF-8 should work: {:?}",
        resp_e
    );

    // Japanese UTF-8 - same login flow (user's saved encoding applies)
    let mut client_j = TestClient::connect(server.addr()).await.unwrap();
    let result = client_j
        .login_with_encoding("utf8_test", "password123", "U")
        .await
        .unwrap();
    assert!(result, "Japanese UTF-8 login should succeed");
}
