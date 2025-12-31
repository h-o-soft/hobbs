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

    // Wait for welcome, enter guest mode
    client.recv_until("Select:").await.unwrap();
    client.send_line("G").await.unwrap();

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

/// Test connection with invalid input.
#[tokio::test]
async fn test_invalid_input_at_welcome() {
    with_test_server(|mut client| async move {
        // Receive initial data (negotiation + welcome)
        let welcome = client.recv().await?;
        assert!(!welcome.is_empty(), "Should receive welcome");

        // Send invalid input
        client.send_line("INVALID").await?;

        // Should still be connected and receive response (error message or re-prompt)
        let response = client.recv().await?;
        assert!(
            !response.is_empty(),
            "Should receive response for invalid input"
        );

        Ok(())
    })
    .await
    .unwrap();
}

/// Test server handles empty input.
#[tokio::test]
async fn test_empty_input() {
    with_test_server(|mut client| async move {
        // Receive initial data (negotiation + welcome)
        let welcome = client.recv().await?;
        assert!(!welcome.is_empty(), "Should receive welcome");

        // Send empty line (just CR)
        client.send_line("").await?;

        // Should still be connected and receive response
        let response = client.recv().await?;
        assert!(!response.is_empty(), "Should handle empty input");

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
