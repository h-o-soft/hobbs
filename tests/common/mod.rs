//! Test helpers for E2E tests.
//!
//! Provides TestClient, TestServer, and helper functions for E2E testing.
//!
//! Note: TestServer uses file-based SQLite databases for isolation,
//! so E2E tests are only available when compiled with the `sqlite` feature.

use std::net::SocketAddr;
#[cfg(feature = "sqlite")]
use std::path::PathBuf;
#[cfg(feature = "sqlite")]
use std::sync::Arc;
#[cfg(feature = "sqlite")]
use std::thread;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
#[cfg(feature = "sqlite")]
use tokio::sync::oneshot;
use tokio::time::timeout;

#[cfg(feature = "sqlite")]
use hobbs::chat::ChatRoomManager;
use hobbs::config::{BbsConfig, Config, DatabaseConfig, LocaleConfig, LoggingConfig, ServerConfig};
use hobbs::server::{encode_for_client, CharacterEncoding, SessionManager};
#[cfg(feature = "sqlite")]
use hobbs::{Application, Database, I18nManager, TelnetServer, TelnetSession, TemplateLoader};

/// Default timeout for test operations.
#[cfg(feature = "sqlite")]
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Test client for connecting to the BBS server.
#[cfg(feature = "sqlite")]
pub struct TestClient {
    stream: TcpStream,
    encoding: CharacterEncoding,
    buffer: Vec<u8>,
}

#[cfg(feature = "sqlite")]
impl TestClient {
    /// Connect to the server at the given address.
    pub async fn connect(addr: SocketAddr) -> Result<Self, std::io::Error> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            stream,
            encoding: CharacterEncoding::Utf8,
            buffer: Vec::with_capacity(4096),
        })
    }

    /// Set the character encoding for this client.
    pub fn set_encoding(&mut self, encoding: CharacterEncoding) {
        self.encoding = encoding;
    }

    /// Send raw bytes to the server.
    pub async fn send_raw(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.stream.write_all(data).await?;
        self.stream.flush().await
    }

    /// Send a string to the server (encoded).
    pub async fn send(&mut self, data: &str) -> Result<(), std::io::Error> {
        let encoded = encode_for_client(data, self.encoding);
        self.send_raw(&encoded).await
    }

    /// Send a line (with CR) to the server.
    pub async fn send_line(&mut self, line: &str) -> Result<(), std::io::Error> {
        self.send(line).await?;
        self.send_raw(b"\r").await
    }

    /// Receive data from the server with timeout.
    pub async fn recv(&mut self) -> Result<String, std::io::Error> {
        self.recv_timeout(DEFAULT_TIMEOUT).await
    }

    /// Receive data from the server with custom timeout.
    /// Waits for data with a small delay between reads to allow server to send complete response.
    pub async fn recv_timeout(&mut self, duration: Duration) -> Result<String, std::io::Error> {
        self.buffer.clear();
        let mut buf = [0u8; 1024];

        let deadline = tokio::time::Instant::now() + duration;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            // Wait a bit for data to arrive
            tokio::time::sleep(Duration::from_millis(50)).await;

            match timeout(Duration::from_millis(100), self.stream.read(&mut buf)).await {
                Ok(Ok(0)) => break, // EOF
                Ok(Ok(n)) => {
                    self.buffer.extend_from_slice(&buf[..n]);
                    // Continue reading if there might be more
                    if n == buf.len() {
                        continue;
                    }
                    // If we have substantial data (not just whitespace), we can return
                    let decoded = self.decode_buffer();
                    if decoded.trim().len() > 5 {
                        return Ok(decoded);
                    }
                    // Otherwise keep waiting for more
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    // Read timeout - check if we have enough data
                    if !self.buffer.is_empty() {
                        let decoded = self.decode_buffer();
                        if decoded.trim().len() > 2 {
                            return Ok(decoded);
                        }
                    }
                }
            }
        }

        // Return whatever we have
        Ok(self.decode_buffer())
    }

    /// Receive data until a pattern is found.
    pub async fn recv_until(&mut self, pattern: &str) -> Result<String, std::io::Error> {
        self.recv_until_timeout(pattern, DEFAULT_TIMEOUT).await
    }

    /// Receive data until a pattern is found with custom timeout.
    pub async fn recv_until_timeout(
        &mut self,
        pattern: &str,
        duration: Duration,
    ) -> Result<String, std::io::Error> {
        self.buffer.clear();
        let mut buf = [0u8; 1];

        let result = timeout(duration, async {
            loop {
                match self.stream.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        self.buffer.push(buf[0]);
                        let decoded = self.decode_buffer();
                        if decoded.contains(pattern) {
                            return Ok(decoded);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(self.decode_buffer())
        })
        .await;

        match result {
            Ok(r) => r,
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Timeout waiting for pattern: {}", pattern),
            )),
        }
    }

    /// Expect a pattern in the received data.
    pub async fn expect(&mut self, pattern: &str) -> Result<String, std::io::Error> {
        let data = self.recv_until(pattern).await?;
        if data.contains(pattern) {
            Ok(data)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Pattern not found: {}", pattern),
            ))
        }
    }

    /// Check if the received data contains a pattern.
    pub async fn contains(&mut self, pattern: &str) -> Result<bool, std::io::Error> {
        let data = self.recv().await?;
        Ok(data.contains(pattern))
    }

    /// Decode the internal buffer to a string.
    fn decode_buffer(&self) -> String {
        // Filter out Telnet control sequences (IAC commands)
        let filtered: Vec<u8> = self
            .buffer
            .iter()
            .copied()
            .filter(|&b| b < 0xF0 || b > 0xFF)
            .collect();

        match self.encoding {
            CharacterEncoding::Utf8 => String::from_utf8_lossy(&filtered).to_string(),
            CharacterEncoding::ShiftJIS => {
                let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&filtered);
                decoded.to_string()
            }
            CharacterEncoding::Cp437 | CharacterEncoding::Petscii => {
                // For tests, just treat as ASCII-compatible for now
                String::from_utf8_lossy(&filtered).to_string()
            }
        }
    }

    /// Skip the initial Telnet negotiation bytes.
    pub async fn skip_negotiation(&mut self) -> Result<(), std::io::Error> {
        // Give the server time to send negotiation
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = self.recv_timeout(Duration::from_millis(500)).await;
        Ok(())
    }

    /// Select language/encoding.
    /// Handles the language selection screen that appears before the welcome screen.
    pub async fn select_language(&mut self, choice: &str) -> Result<(), std::io::Error> {
        // Wait for language selection screen
        self.recv_until("Gengo").await?;
        self.send_line(choice).await?;
        Ok(())
    }

    /// Select language/encoding and set client encoding accordingly.
    /// Options:
    /// - "E" or "1": English (UTF-8)
    /// - "J" or "2": Japanese (ShiftJIS)
    /// - "U" or "3": Japanese (UTF-8)
    pub async fn select_language_with_encoding(
        &mut self,
        choice: &str,
    ) -> Result<(), std::io::Error> {
        // Wait for language selection screen
        self.recv_until("Gengo").await?;

        // Set client encoding based on choice
        match choice.to_uppercase().as_str() {
            "E" | "1" => self.encoding = CharacterEncoding::Utf8,
            "J" | "2" => self.encoding = CharacterEncoding::ShiftJIS,
            "U" | "3" => self.encoding = CharacterEncoding::Utf8,
            _ => self.encoding = CharacterEncoding::Utf8,
        }

        self.send_line(choice).await?;
        Ok(())
    }

    /// Perform login with specific encoding.
    /// Note: The new flow doesn't require language selection before login.
    /// The user's saved encoding/language will be applied after successful login.
    pub async fn login_with_encoding(
        &mut self,
        username: &str,
        password: &str,
        _language_choice: &str,
    ) -> Result<bool, std::io::Error> {
        // Wait for welcome screen (ASCII) - choose login
        self.recv_until_timeout("Select:", Duration::from_secs(3))
            .await?;
        self.send_line("L").await?;

        // Wait for username prompt
        self.recv_until_timeout(":", Duration::from_secs(3)).await?;
        self.send_line(username).await?;

        // Wait for password prompt
        self.recv_until_timeout(":", Duration::from_secs(3)).await?;
        self.send_line(password).await?;

        // Wait for login result
        let response = self.recv_timeout(Duration::from_secs(3)).await?;
        Ok(response.contains("success")
            || response.contains("ようこそ")
            || response.contains("Welcome"))
    }

    /// Perform login sequence.
    /// New flow: welcome screen (ASCII) -> choose L -> login.
    /// User's encoding/language will be applied after successful login.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<bool, std::io::Error> {
        // Wait for welcome screen (ASCII) - choose login
        self.recv_until("Select:").await?;
        self.send_line("L").await?;

        // Wait for username prompt
        self.recv_until("Username:").await?;
        self.send_line(username).await?;

        // Wait for password prompt
        self.recv_until("Password:").await?;
        self.send_line(password).await?;

        // Wait for login result
        let response = self.recv_timeout(Duration::from_secs(3)).await?;
        Ok(response.contains("success")
            || response.contains("ようこそ")
            || response.contains("Welcome"))
    }

    /// Perform registration sequence.
    /// New flow: welcome screen (ASCII) -> choose R -> language selection -> register.
    pub async fn register(
        &mut self,
        username: &str,
        password: &str,
        nickname: &str,
    ) -> Result<bool, std::io::Error> {
        // Wait for welcome screen (ASCII) - choose register
        self.recv_until("Select:").await?;
        self.send_line("R").await?;

        // Handle language selection (appears after choosing R)
        self.select_language("E").await?;

        // Wait for username prompt
        self.recv_until("Username:").await?;
        self.send_line(username).await?;

        // Wait for password prompt
        self.recv_until("Password:").await?;
        self.send_line(password).await?;

        // Wait for confirm password prompt
        self.recv_until(":").await?;
        self.send_line(password).await?;

        // Wait for nickname prompt
        self.recv_until(":").await?;
        self.send_line(nickname).await?;

        // Wait for registration result - wait for menu to appear after success
        let response = self
            .recv_until_timeout("Select:", Duration::from_secs(5))
            .await?;
        Ok(response.contains("success")
            || response.contains("登録完了")
            || response.contains("Welcome")
            || response.contains("Main Menu"))
    }

    /// Enter guest mode.
    /// New flow: welcome screen (ASCII) -> choose G -> language selection -> menu.
    pub async fn enter_guest(&mut self) -> Result<(), std::io::Error> {
        // Wait for welcome screen
        self.recv_until("Select:").await?;
        self.send_line("G").await?;
        // Handle language selection (appears after choosing G)
        self.select_language("E").await?;
        // Wait for menu to appear
        let _ = self.recv_timeout(Duration::from_secs(2)).await;
        Ok(())
    }

    /// Quit the session.
    pub async fn quit(&mut self) -> Result<(), std::io::Error> {
        self.send_line("Q").await?;
        Ok(())
    }
}

/// Test server configuration and lifecycle management.
/// Runs the server in a separate thread with its own tokio runtime.
///
/// Note: This struct is only available with the `sqlite` feature because
/// it uses file-based SQLite databases for test isolation.
#[cfg(feature = "sqlite")]
pub struct TestServer {
    addr: SocketAddr,
    db: Database,
    db_path: PathBuf,
    shutdown_tx: Option<oneshot::Sender<()>>,
    _thread_handle: Option<thread::JoinHandle<()>>,
}

#[cfg(feature = "sqlite")]
impl TestServer {
    /// Create a new test server with a temporary file-based database.
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_config(test_config()).await
    }

    /// Create a new test server with custom configuration.
    pub async fn with_config(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Create a unique temp file path for the database
        let db_path = std::env::temp_dir().join(format!("hobbs_test_{}.db", uuid::Uuid::new_v4()));

        // Create database for test setup (in this thread)
        let db = Database::open(&db_path).await?;

        // Bind server to a random port first to get the address
        let server_config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Let OS assign a port
            max_connections: 10,
            idle_timeout_secs: 300,
            read_timeout_secs: 30,
            guest_timeout_secs: 120,
            timezone: "Asia/Tokyo".to_string(),
        };

        let server = TelnetServer::bind(&server_config).await?;
        let addr = server.local_addr()?;

        // Create channel for shutdown signal
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Clone the path for the server thread
        let db_path_for_server = db_path.clone();

        // Spawn server in a separate thread with its own runtime
        let thread_handle =
            thread::spawn(move || {
                // Create a new single-threaded runtime for this thread
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create runtime");

                rt.block_on(async move {
                    // Create database connection for this thread
                    let server_db = Arc::new(
                        Database::open(&db_path_for_server)
                            .await
                            .expect("Failed to open database in server thread"),
                    );

                    // Create I18n manager
                    let i18n_manager =
                        Arc::new(I18nManager::load_all("locales").expect("Failed to load i18n"));

                    // Create template loader
                    let template_loader = Arc::new(TemplateLoader::new("templates"));

                    // Create session manager
                    let session_manager = Arc::new(SessionManager::new(300));

                    // Create chat room manager
                    let chat_manager = Arc::new(ChatRoomManager::with_defaults().await);

                    // Create application
                    let app = Application::new(
                        server_db,
                        Arc::new(config),
                        i18n_manager,
                        template_loader,
                        session_manager,
                        chat_manager,
                    );

                    // Create LocalSet for non-Send futures
                    let local = tokio::task::LocalSet::new();

                    local.run_until(async move {
                    let mut shutdown_rx = shutdown_rx;

                    loop {
                        tokio::select! {
                            _ = &mut shutdown_rx => {
                                break;
                            }
                            result = server.accept() => {
                                match result {
                                    Ok((stream, addr, permit)) => {
                                        let app = app.clone();
                                        tokio::task::spawn_local(async move {
                                            let mut session = TelnetSession::new(stream, addr);
                                            let _ = app.run_session(&mut session).await;
                                            drop(permit);
                                        });
                                    }
                                    Err(_) => {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }).await;
                });
            });

        Ok(Self {
            addr,
            db,
            db_path,
            shutdown_tx: Some(shutdown_tx),
            _thread_handle: Some(thread_handle),
        })
    }

    /// Get the local address of the server.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get a reference to the database (for test setup).
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Stop the server.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(feature = "sqlite")]
impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop();
        // Give the server thread time to shutdown
        thread::sleep(Duration::from_millis(50));
        // Clean up the temp database file
        let _ = std::fs::remove_file(&self.db_path);
        // Also remove WAL and SHM files if they exist
        let _ = std::fs::remove_file(format!("{}-wal", self.db_path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.db_path.display()));
    }
}

/// Create a test configuration.
pub fn test_config() -> Config {
    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            max_connections: 10,
            idle_timeout_secs: 300,
            read_timeout_secs: 30,
            guest_timeout_secs: 120,
            timezone: "Asia/Tokyo".to_string(),
        },
        database: DatabaseConfig {
            path: ":memory:".to_string(),
            ..Default::default()
        },
        bbs: BbsConfig {
            name: "Test BBS".to_string(),
            description: "A test BBS for E2E testing".to_string(),
            sysop_name: "TestSysOp".to_string(),
        },
        locale: LocaleConfig {
            language: "en".to_string(),
        },
        logging: LoggingConfig {
            level: "warn".to_string(),
            file: String::new(), // No file logging for tests
        },
        files: Default::default(),
        templates: Default::default(),
        terminal: Default::default(),
        rss: Default::default(),
        web: Default::default(),
        rate_limits: Default::default(),
    }
}

/// Run a test with a fresh test server.
///
/// This helper function creates a new test server, runs the provided
/// async closure with a connected client, and cleans up afterward.
/// Note: The client receives raw connection data; tests should handle
/// Telnet negotiation and welcome screen as needed.
#[cfg(feature = "sqlite")]
pub async fn with_test_server<F, Fut>(f: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(TestClient) -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    // Create server (starts automatically)
    let mut server = TestServer::new().await?;

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = TestClient::connect(server.addr()).await?;

    // Run the test
    let result = f(client).await;

    // Stop server
    server.stop();

    result
}

/// Run a test with a server and multiple clients.
#[cfg(feature = "sqlite")]
pub async fn with_test_server_multi<F, Fut>(
    num_clients: usize,
    f: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(Vec<TestClient>) -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    // Create server (starts automatically)
    let mut server = TestServer::new().await?;

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect clients
    let mut clients = Vec::new();
    for _ in 0..num_clients {
        let client = TestClient::connect(server.addr()).await?;
        clients.push(client);
    }

    // Run the test
    let result = f(clients).await;

    // Stop server
    server.stop();

    result
}

/// Create a test user in the database.
#[cfg(feature = "sqlite")]
pub async fn create_test_user(
    db: &hobbs::Database,
    username: &str,
    password: &str,
    role: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    use hobbs::db::{NewUser, Role, UserRepository};

    let password_hash = hobbs::hash_password(password)?;
    let user_role: Role = role.parse().unwrap_or(Role::Member);

    let user_repo = UserRepository::new(db.pool());
    let new_user = NewUser::new(username, &password_hash, username).with_role(user_role);
    let user = user_repo.create(&new_user).await?;

    Ok(user.id)
}

/// Create a test user with specific language and encoding settings.
#[cfg(feature = "sqlite")]
pub async fn create_test_user_with_settings(
    db: &hobbs::Database,
    username: &str,
    password: &str,
    role: &str,
    language: &str,
    encoding: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    use hobbs::db::{NewUser, Role, UserRepository};
    use hobbs::server::CharacterEncoding;

    let password_hash = hobbs::hash_password(password)?;
    let user_role: Role = role.parse().unwrap_or(Role::Member);
    let char_encoding: CharacterEncoding = encoding.parse().unwrap_or(CharacterEncoding::ShiftJIS);

    let user_repo = UserRepository::new(db.pool());
    let new_user = NewUser::new(username, &password_hash, username)
        .with_role(user_role)
        .with_language(language)
        .with_encoding(char_encoding);
    let user = user_repo.create(&new_user).await?;

    Ok(user.id)
}

/// Create a test board in the database.
#[cfg(feature = "sqlite")]
pub async fn create_test_board(
    db: &hobbs::Database,
    name: &str,
    board_type: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    use hobbs::board::{BoardRepository, BoardType, NewBoard};

    let bt: BoardType = board_type.parse().unwrap_or(BoardType::Thread);
    let board_repo = BoardRepository::new(db.pool());
    let new_board = NewBoard::new(name)
        .with_description(&format!("Test board: {}", name))
        .with_board_type(bt);
    let board = board_repo.create(&new_board).await?;

    Ok(board.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_config() {
        let config = test_config();
        assert_eq!(config.bbs.name, "Test BBS");
        assert_eq!(config.locale.language, "en");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_test_server() {
        let server = TestServer::new().await;
        assert!(server.is_ok());
    }
}
