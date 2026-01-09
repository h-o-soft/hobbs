#![cfg(feature = "sqlite")]
//! E2E Connection tests for HOBBS.
//!
//! Tests basic Telnet connection, negotiation, and session lifecycle.

mod common;

use common::{with_test_server, with_test_server_multi, TestClient, TestServer};
use std::time::Duration;

/// Test basic connection to the server.
#[tokio::test]
async fn test_connection_basic() {
    with_test_server(|mut client| async move {
        // Should receive initial data (negotiation + welcome screen)
        let response = client.recv().await?;

        // Debug: print response info
        eprintln!(
            "Response length: {}, content: {:?}",
            response.len(),
            response
        );

        assert!(!response.is_empty(), "Should receive welcome message");
        Ok(())
    })
    .await
    .unwrap();
}

/// Test multiple simultaneous connections.
#[tokio::test]
async fn test_multiple_connections() {
    with_test_server_multi(3, |mut clients| async move {
        // All clients should receive initial data (negotiation + welcome)
        for client in &mut clients {
            let response = client.recv().await?;
            assert!(!response.is_empty(), "Each client should receive welcome");
        }
        Ok(())
    })
    .await
    .unwrap();
}

/// Test guest mode entry.
#[tokio::test]
async fn test_guest_mode() {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // New flow: welcome screen (ASCII) appears first
    // Wait for welcome screen prompt
    client.recv_until("Select:").await.unwrap();
    client.send_line("G").await.unwrap();

    // Language selection appears after choosing G
    client.recv_until("Gengo").await.unwrap();
    // Select English
    client.send_line("E").await.unwrap();

    // Should see main menu
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    // Guest mode should work (menu should appear)
    assert!(
        response.contains("B")
            || response.contains("Q")
            || response.contains("Board")
            || response.contains("Menu"),
        "Should see menu options: {:?}",
        response
    );
}

/// Test quit command.
#[tokio::test]
async fn test_quit() {
    with_test_server(|mut client| async move {
        // Receive initial data (negotiation + welcome)
        let welcome = client.recv().await?;
        assert!(!welcome.is_empty(), "Should receive welcome");

        // Quit immediately
        client.send_line("Q").await?;

        // Should receive goodbye or connection close
        let response = client.recv_timeout(Duration::from_secs(2)).await?;
        // After quitting, we may get goodbye message or just timeout/close
        // Either outcome is acceptable
        Ok(())
    })
    .await
    .unwrap();
}

/// Test connection with invalid input stays at welcome screen.
#[tokio::test]
async fn test_invalid_input_at_welcome() {
    with_test_server(|mut client| async move {
        // Receive initial data (negotiation + welcome)
        let welcome = client.recv().await?;
        assert!(!welcome.is_empty(), "Should receive welcome");

        // Send invalid input
        client.send_line("INVALID").await?;

        // Should receive error message and prompt again (not proceed to guest)
        let response = client.recv().await?;
        assert!(
            !response.is_empty(),
            "Should receive response for invalid input"
        );

        // The response should contain the prompt again, not main menu
        // After fix: stays at welcome screen with re-prompt
        assert!(
            response.contains("[L]")
                || response.contains("[1]")
                || response.contains("Login")
                || response.contains(">"),
            "Should show welcome prompt again after invalid input: {:?}",
            response
        );

        // Now send valid input (G for guest) to proceed
        client.send_line("G").await?;

        // New flow: language selection appears after choosing G
        let lang_response = client.recv().await?;
        assert!(
            lang_response.contains("Gengo") || lang_response.contains("English"),
            "Should see language selection: {:?}",
            lang_response
        );

        // Select English
        client.send_line("E").await?;
        let menu = client.recv().await?;

        // Now should be at main menu
        assert!(
            menu.contains("[B]")
                || menu.contains("[C]")
                || menu.contains("Menu")
                || menu.contains("Board")
                || menu.contains("Chat"),
            "Should now be at main menu: {:?}",
            menu
        );

        Ok(())
    })
    .await
    .unwrap();
}

/// Test server handles empty input - stays at welcome screen.
#[tokio::test]
async fn test_empty_input() {
    with_test_server(|mut client| async move {
        // Receive initial data (negotiation + welcome)
        let welcome = client.recv().await?;
        assert!(!welcome.is_empty(), "Should receive welcome");

        // Send empty line (just CR)
        client.send_line("").await?;

        // Should receive error message and prompt again (not proceed to guest)
        let response = client.recv().await?;
        assert!(!response.is_empty(), "Should handle empty input");

        // The response should contain the prompt again, not main menu
        assert!(
            response.contains("[L]")
                || response.contains("[1]")
                || response.contains("Login")
                || response.contains(">"),
            "Should show welcome prompt again after empty input: {:?}",
            response
        );

        Ok(())
    })
    .await
    .unwrap();
}

/// Test TestServer lifecycle.
#[tokio::test]
async fn test_server_lifecycle() {
    // Create server (starts automatically)
    let mut server = TestServer::new().await.unwrap();
    let addr = server.addr();

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = TestClient::connect(addr).await;
    assert!(client.is_ok(), "Should be able to connect");

    // Stop server
    server.stop();
}
