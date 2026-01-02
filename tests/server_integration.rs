//! Integration tests for the Telnet server.

use std::time::Duration;

use hobbs::config::ServerConfig;
use hobbs::TelnetServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn test_config(port: u16, max_connections: usize) -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port,
        max_connections,
        idle_timeout_secs: 300,
        read_timeout_secs: 30,
        timezone: "Asia/Tokyo".to_string(),
    }
}

#[tokio::test]
async fn test_server_accepts_multiple_clients() {
    let config = test_config(0, 5);
    let server = TelnetServer::bind(&config).await.unwrap();
    let addr = server.local_addr().unwrap();

    // Connect multiple clients
    let mut clients = Vec::new();
    let mut permits = Vec::new();

    for _ in 0..3 {
        let client = tokio::net::TcpStream::connect(addr).await.unwrap();
        clients.push(client);

        let (_, _, permit) = server.accept().await.unwrap();
        permits.push(permit);
    }

    assert_eq!(server.active_connections(), 3);
    assert_eq!(server.available_connections(), 2);

    // Drop all permits and clients
    drop(permits);
    drop(clients);

    // Give time for cleanup
    tokio::time::sleep(Duration::from_millis(10)).await;

    assert_eq!(server.active_connections(), 0);
    assert_eq!(server.available_connections(), 5);
}

#[tokio::test]
async fn test_server_client_communication() {
    let config = test_config(0, 10);
    let server = TelnetServer::bind(&config).await.unwrap();
    let addr = server.local_addr().unwrap();

    // Connect a client
    let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (mut stream, _, _permit) = server.accept().await.unwrap();

    // Server sends welcome message
    let welcome = b"Welcome to HOBBS!\r\n";
    stream.write_all(welcome).await.unwrap();

    // Client receives welcome message
    let mut buf = vec![0u8; welcome.len()];
    client.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, welcome);

    // Client sends response
    let response = b"Hello!\r\n";
    client.write_all(response).await.unwrap();

    // Server receives response
    let mut buf = vec![0u8; response.len()];
    stream.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, response);
}

#[tokio::test]
async fn test_server_handles_client_disconnect() {
    let config = test_config(0, 10);
    let server = TelnetServer::bind(&config).await.unwrap();
    let addr = server.local_addr().unwrap();

    // Connect and immediately disconnect
    let client = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (mut stream, _, permit) = server.accept().await.unwrap();

    assert_eq!(server.active_connections(), 1);

    // Drop client
    drop(client);

    // Try to write to the stream - should eventually fail
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Reading from disconnected client should return 0 bytes (EOF)
    let mut buf = [0u8; 1];
    let result = stream.read(&mut buf).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0); // EOF

    // Drop permit to release connection slot
    drop(permit);

    assert_eq!(server.active_connections(), 0);
}

#[tokio::test]
async fn test_max_connections_releases_on_drop() {
    let config = test_config(0, 2);
    let server = TelnetServer::bind(&config).await.unwrap();
    let addr = server.local_addr().unwrap();

    // Fill up all connection slots
    let _client1 = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (_, _, permit1) = server.accept().await.unwrap();

    let _client2 = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (_, _, permit2) = server.accept().await.unwrap();

    assert_eq!(server.active_connections(), 2);
    assert_eq!(server.available_connections(), 0);

    // Release one slot by dropping the permit
    drop(permit1);

    assert_eq!(server.active_connections(), 1);
    assert_eq!(server.available_connections(), 1);

    // Now we can accept another connection
    let _client3 = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (_, _, _permit3) = server.accept().await.unwrap();

    assert_eq!(server.active_connections(), 2);

    drop(permit2);
}
