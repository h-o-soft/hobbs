//! TCP listener for the Telnet server.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use crate::config::ServerConfig;
use crate::Result;

/// Telnet server that accepts TCP connections.
pub struct TelnetServer {
    listener: TcpListener,
    semaphore: Arc<Semaphore>,
    max_connections: usize,
}

impl TelnetServer {
    /// Create a new TelnetServer bound to the specified address.
    pub async fn bind(config: &ServerConfig) -> Result<Self> {
        let addr = format!("{}:{}", config.host, config.port);
        let listener = TcpListener::bind(&addr).await?;
        let local_addr = listener.local_addr()?;

        info!("Telnet server listening on {}", local_addr);

        Ok(Self {
            listener,
            semaphore: Arc::new(Semaphore::new(config.max_connections)),
            max_connections: config.max_connections,
        })
    }

    /// Get the local address the server is bound to.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Get the maximum number of connections allowed.
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }

    /// Get the number of available connection slots.
    pub fn available_connections(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Get the number of active connections.
    pub fn active_connections(&self) -> usize {
        self.max_connections - self.semaphore.available_permits()
    }

    /// Accept a new connection.
    ///
    /// This method will wait until a connection slot is available (if max
    /// connections is reached) and then accept the next incoming connection.
    ///
    /// Returns the TCP stream and the peer address.
    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr, ConnectionPermit)> {
        // Acquire a permit before accepting the connection
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| crate::HobbsError::Io(std::io::Error::other("semaphore closed")))?;

        let (stream, addr) = self.listener.accept().await?;
        debug!("Accepted connection from {}", addr);

        Ok((stream, addr, ConnectionPermit { _permit: permit }))
    }

    /// Run the server, accepting connections and spawning handlers.
    ///
    /// The `handler` function is called for each new connection.
    pub async fn run<F, Fut>(self, handler: F) -> Result<()>
    where
        F: Fn(TcpStream, SocketAddr) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(handler);

        loop {
            match self.accept().await {
                Ok((stream, addr, permit)) => {
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        handler(stream, addr).await;
                        // Permit is dropped here, releasing the connection slot
                        drop(permit);
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

/// A permit that represents an active connection slot.
///
/// When this permit is dropped, the connection slot is released.
pub struct ConnectionPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn test_config(port: u16, max_connections: usize) -> ServerConfig {
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port,
            max_connections,
            idle_timeout_secs: 300,
        }
    }

    #[tokio::test]
    async fn test_server_bind() {
        let config = test_config(0, 10); // Port 0 = OS assigns random port
        let server = TelnetServer::bind(&config).await.unwrap();

        assert!(server.local_addr().is_ok());
        assert_eq!(server.max_connections(), 10);
        assert_eq!(server.available_connections(), 10);
        assert_eq!(server.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_accept_connection() {
        let config = test_config(0, 10);
        let server = TelnetServer::bind(&config).await.unwrap();
        let addr = server.local_addr().unwrap();

        // Connect a client
        let client = tokio::net::TcpStream::connect(addr).await.unwrap();

        // Accept the connection
        let (stream, peer_addr, _permit) = server.accept().await.unwrap();

        assert_eq!(peer_addr, client.local_addr().unwrap());
        assert_eq!(server.active_connections(), 1);
        assert_eq!(server.available_connections(), 9);

        drop(stream);
        drop(client);
    }

    #[tokio::test]
    async fn test_max_connections_limit() {
        let config = test_config(0, 2);
        let server = Arc::new(TelnetServer::bind(&config).await.unwrap());
        let addr = server.local_addr().unwrap();

        // Connect two clients (max)
        let _client1 = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (_stream1, _, permit1) = server.accept().await.unwrap();

        let _client2 = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (_stream2, _, permit2) = server.accept().await.unwrap();

        assert_eq!(server.active_connections(), 2);
        assert_eq!(server.available_connections(), 0);

        // Try to connect a third client - should succeed connecting but
        // server.accept() would block until a slot is available
        let _client3 = tokio::net::TcpStream::connect(addr).await.unwrap();

        // Drop one permit to free a slot
        drop(permit1);

        // Now we should be able to accept the third connection
        let (_stream3, _, _permit3) = server.accept().await.unwrap();
        assert_eq!(server.active_connections(), 2);

        drop(permit2);
    }

    #[tokio::test]
    async fn test_connection_read_write() {
        let config = test_config(0, 10);
        let server = TelnetServer::bind(&config).await.unwrap();
        let addr = server.local_addr().unwrap();

        // Connect a client
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();

        // Accept the connection
        let (mut stream, _, _permit) = server.accept().await.unwrap();

        // Write from server to client
        stream.write_all(b"Hello, client!").await.unwrap();

        // Read on client
        let mut buf = [0u8; 14];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"Hello, client!");

        // Write from client to server
        client.write_all(b"Hello, server!").await.unwrap();

        // Read on server
        let mut buf = [0u8; 14];
        stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"Hello, server!");
    }
}
