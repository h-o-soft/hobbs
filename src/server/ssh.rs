//! SSH tunnel server module.
//!
//! Provides an SSH server that accepts `direct-tcpip` (port forwarding) connections
//! and relays them to the internal Telnet port. This allows encrypted access to the
//! BBS via SSH tunneling (e.g., `ssh -L 12323:localhost:2323 bbs@server -p 2222 -N`).
//!
//! Shell sessions are not supported because the BBS uses Telnet IAC negotiation
//! which SSH terminals cannot process.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};

use russh::keys::PrivateKey;
use russh::server::{Auth, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId};
use tokio::net::TcpStream;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{error, info, warn};

use crate::{Config, HobbsError, Result};

/// Load an existing SSH host key or generate a new Ed25519 key.
fn load_or_generate_host_key(path: &str) -> Result<PrivateKey> {
    let key_path = Path::new(path);

    // Ensure parent directory exists
    if let Some(parent) = key_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                HobbsError::Config(format!(
                    "Failed to create directory for SSH host key '{}': {}",
                    path, e
                ))
            })?;
        }
    }

    if key_path.exists() {
        // Load existing key
        let pem = std::fs::read_to_string(path).map_err(|e| {
            HobbsError::Config(format!("Failed to read SSH host key '{}': {}", path, e))
        })?;
        PrivateKey::from_openssh(&pem).map_err(|e| {
            HobbsError::Config(format!(
                "SSH host key '{}' is invalid: {}. Delete the file to regenerate.",
                path, e
            ))
        })
    } else {
        // Generate new Ed25519 key
        let key = PrivateKey::random(
            &mut rand_core::OsRng,
            russh::keys::Algorithm::Ed25519,
        )
        .map_err(|e| HobbsError::Config(format!("Failed to generate SSH host key: {}", e)))?;

        // Save to file in OpenSSH format
        let pem = key
            .to_openssh(russh::keys::ssh_key::LineEnding::LF)
            .map_err(|e| {
                HobbsError::Config(format!("Failed to encode SSH host key: {}", e))
            })?;
        std::fs::write(path, pem.as_bytes()).map_err(|e| {
            HobbsError::Config(format!("Failed to write SSH host key '{}': {}", path, e))
        })?;

        // Set file permissions to 0600 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(
                |e| {
                    HobbsError::Config(format!(
                        "Failed to set permissions on SSH host key '{}': {}",
                        path, e
                    ))
                },
            )?;
        }

        info!("SSH host key generated: {}", path);
        Ok(key)
    }
}

/// SSH server factory that creates a handler for each new connection.
struct BbsServer {
    config: Arc<Config>,
    semaphore: Arc<Semaphore>,
}

impl Server for BbsServer {
    type Handler = BbsHandler;

    fn new_client(&mut self, peer_addr: Option<SocketAddr>) -> Self::Handler {
        // Try to acquire a connection permit (non-blocking)
        let permit = match self.semaphore.clone().try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => {
                warn!(
                    "SSH connection limit reached, rejecting connection from {:?}",
                    peer_addr
                );
                None
            }
        };

        BbsHandler {
            telnet_addr: format!("127.0.0.1:{}", self.config.server.port),
            config: Arc::clone(&self.config),
            peer_addr,
            active_channels: Arc::new(Mutex::new(HashSet::new())),
            _permit: permit,
        }
    }
}

/// SSH connection handler for a single client.
struct BbsHandler {
    /// Internal Telnet address to relay to.
    telnet_addr: String,
    /// Shared configuration.
    config: Arc<Config>,
    /// Peer address of the SSH client.
    peer_addr: Option<SocketAddr>,
    /// Set of active channel IDs (shared with relay tasks for cleanup).
    active_channels: Arc<Mutex<HashSet<ChannelId>>>,
    /// Connection permit (None = connection was rejected at limit).
    _permit: Option<OwnedSemaphorePermit>,
}

impl Handler for BbsHandler {
    type Error = russh::Error;

    /// Reject unauthenticated access.
    async fn auth_none(&mut self, _user: &str) -> std::result::Result<Auth, Self::Error> {
        Ok(Auth::Reject {
            proceed_with_methods: None,
            partial_success: false,
        })
    }

    /// Authenticate with shared username/password.
    async fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> std::result::Result<Auth, Self::Error> {
        // Reject if connection was not permitted (limit reached)
        if self._permit.is_none() {
            return Ok(Auth::Reject {
                proceed_with_methods: None,
                partial_success: false,
            });
        }

        if user == self.config.ssh.username && password == self.config.ssh.password {
            info!("SSH authentication successful from {:?}", self.peer_addr);
            Ok(Auth::Accept)
        } else {
            warn!(
                "SSH authentication failed from {:?}: invalid credentials",
                self.peer_addr
            );
            Ok(Auth::Reject {
                proceed_with_methods: None,
                partial_success: false,
            })
        }
    }

    /// Reject public key authentication.
    async fn auth_publickey(
        &mut self,
        _user: &str,
        _public_key: &russh::keys::PublicKey,
    ) -> std::result::Result<Auth, Self::Error> {
        Ok(Auth::Reject {
            proceed_with_methods: None,
            partial_success: false,
        })
    }

    /// Reject shell sessions (SSH terminals can't handle Telnet IAC).
    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        _session: &mut Session,
    ) -> std::result::Result<bool, Self::Error> {
        info!(
            "SSH shell session rejected from {:?} (not supported)",
            self.peer_addr
        );
        Ok(false)
    }

    /// Handle direct-tcpip (port forwarding) requests.
    /// Only allows forwarding to the internal Telnet port.
    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        originator_address: &str,
        originator_port: u32,
        _session: &mut Session,
    ) -> std::result::Result<bool, Self::Error> {
        let telnet_port = self.config.server.port as u32;

        // Only allow forwarding to the internal Telnet port on localhost
        let allowed_host =
            host_to_connect == "127.0.0.1" || host_to_connect == "localhost";

        if !allowed_host || port_to_connect != telnet_port {
            warn!(
                "SSH direct-tcpip rejected from {:?}: {}:{} (only 127.0.0.1/localhost:{} allowed)",
                self.peer_addr, host_to_connect, port_to_connect, telnet_port
            );
            return Ok(false);
        }

        // Check channel limit
        let channel_count = {
            let channels = self.active_channels.lock().unwrap();
            channels.len()
        };
        if channel_count >= self.config.ssh.max_channels_per_connection {
            warn!(
                "SSH channel limit reached from {:?}: {}/{}",
                self.peer_addr, channel_count, self.config.ssh.max_channels_per_connection
            );
            return Ok(false);
        }

        // Connect to internal Telnet port
        let tcp_stream = match TcpStream::connect(&self.telnet_addr).await {
            Ok(stream) => stream,
            Err(e) => {
                error!(
                    "Failed to connect to internal Telnet port {}: {}",
                    self.telnet_addr, e
                );
                return Ok(false);
            }
        };

        let channel_id = channel.id();

        // Register channel
        {
            let mut channels = self.active_channels.lock().unwrap();
            channels.insert(channel_id);
        }

        info!(
            "SSH port forwarding from {:?} ({}:{}) -> {}",
            self.peer_addr, originator_address, originator_port, self.telnet_addr
        );

        // Start bidirectional relay in a background task
        let active_channels = Arc::clone(&self.active_channels);
        let peer_addr = self.peer_addr;
        tokio::spawn(async move {
            let mut channel_stream = channel.into_stream();
            let mut tcp_stream = tcp_stream;

            match tokio::io::copy_bidirectional(&mut channel_stream, &mut tcp_stream).await {
                Ok((to_telnet, from_telnet)) => {
                    info!(
                        "SSH relay completed for {:?}: {} bytes to Telnet, {} bytes from Telnet",
                        peer_addr, to_telnet, from_telnet
                    );
                }
                Err(e) => {
                    // Connection closed is normal (client disconnect)
                    info!("SSH relay ended for {:?}: {}", peer_addr, e);
                }
            }

            // Cleanup
            {
                let mut channels = active_channels.lock().unwrap();
                channels.remove(&channel_id);
            }
        });

        Ok(true)
    }

    /// Handle channel close.
    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> std::result::Result<(), Self::Error> {
        let mut channels = self.active_channels.lock().unwrap();
        channels.remove(&channel);
        Ok(())
    }

    /// Handle channel EOF.
    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> std::result::Result<(), Self::Error> {
        let mut channels = self.active_channels.lock().unwrap();
        channels.remove(&channel);
        Ok(())
    }
}

/// Run the SSH tunnel server.
pub async fn run(config: Arc<Config>) -> Result<()> {
    let host_key = load_or_generate_host_key(&config.ssh.host_key_path)?;

    // Warn if Telnet is publicly accessible while SSH is enabled
    if config.server.host != "127.0.0.1" && config.server.host != "localhost" {
        warn!(
            "SSH is enabled but Telnet is bound to {}. \
             Recommend setting server.host = \"127.0.0.1\" to prevent plaintext access.",
            config.server.host
        );
    }

    let russh_config = russh::server::Config {
        keys: vec![host_key],
        ..Default::default()
    };

    let semaphore = Arc::new(Semaphore::new(config.ssh.max_connections));
    let addr = format!("{}:{}", config.ssh.host, config.ssh.port);

    info!("SSH server listening on {}", addr);

    let mut server = BbsServer {
        config,
        semaphore,
    };

    server
        .run_on_address(Arc::new(russh_config), &addr)
        .await
        .map_err(|e| HobbsError::Config(format!("SSH server error: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_or_generate_host_key() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("test_host_key");
        let key_path_str = key_path.to_str().unwrap();

        // Generate new key
        let key1 = load_or_generate_host_key(key_path_str).unwrap();
        assert!(key_path.exists());

        // Verify permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&key_path).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o600);
        }

        // Reload same key
        let key2 = load_or_generate_host_key(key_path_str).unwrap();
        assert_eq!(
            key1.public_key().to_string(),
            key2.public_key().to_string()
        );
    }

    #[test]
    fn test_load_invalid_host_key() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("invalid_key");
        std::fs::write(&key_path, "this is not a valid key").unwrap();

        let result = load_or_generate_host_key(key_path.to_str().unwrap());
        assert!(result.is_err());
        if let Err(HobbsError::Config(msg)) = result {
            assert!(msg.contains("invalid"));
            assert!(msg.contains("Delete the file"));
        }
    }

    #[test]
    fn test_generate_key_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("sub").join("dir").join("key");

        let result = load_or_generate_host_key(key_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(key_path.exists());
    }
}
