//! Session management for the Telnet server.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use super::encoding::CharacterEncoding;

/// Session state representing the current phase of the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Initial connection, showing welcome screen.
    Welcome,
    /// User is at the login prompt.
    Login,
    /// User is registering a new account.
    Registration,
    /// User is at the main menu.
    MainMenu,
    /// User is browsing boards.
    Board,
    /// User is in a chat room.
    Chat,
    /// User is reading/writing mail.
    Mail,
    /// User is in file management.
    Files,
    /// User is in admin menu.
    Admin,
    /// Session is being closed.
    Closing,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Welcome
    }
}

/// A Telnet session representing a connected client.
pub struct TelnetSession {
    /// Unique session identifier.
    id: Uuid,
    /// The TCP stream for this connection.
    stream: TcpStream,
    /// Remote peer address.
    peer_addr: SocketAddr,
    /// Current session state.
    state: SessionState,
    /// Timestamp of last activity.
    last_activity: Instant,
    /// User ID if logged in (None for guest/anonymous).
    user_id: Option<i64>,
    /// Username if logged in.
    username: Option<String>,
    /// Character encoding for this session.
    encoding: CharacterEncoding,
}

impl TelnetSession {
    /// Create a new session from a TCP stream.
    pub fn new(stream: TcpStream, peer_addr: SocketAddr) -> Self {
        let id = Uuid::new_v4();
        debug!("Created new session {} for {}", id, peer_addr);

        Self {
            id,
            stream,
            peer_addr,
            state: SessionState::Welcome,
            last_activity: Instant::now(),
            user_id: None,
            username: None,
            encoding: CharacterEncoding::default(),
        }
    }

    /// Create a new session with a specific encoding.
    pub fn with_encoding(
        stream: TcpStream,
        peer_addr: SocketAddr,
        encoding: CharacterEncoding,
    ) -> Self {
        let id = Uuid::new_v4();
        debug!(
            "Created new session {} for {} with encoding {:?}",
            id, peer_addr, encoding
        );

        Self {
            id,
            stream,
            peer_addr,
            state: SessionState::Welcome,
            last_activity: Instant::now(),
            user_id: None,
            username: None,
            encoding,
        }
    }

    /// Get the session ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get a reference to the TCP stream.
    pub fn stream(&self) -> &TcpStream {
        &self.stream
    }

    /// Get a mutable reference to the TCP stream.
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Get the peer address.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Get the current session state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Set the session state.
    pub fn set_state(&mut self, state: SessionState) {
        debug!(
            "Session {} state changed: {:?} -> {:?}",
            self.id, self.state, state
        );
        self.state = state;
        self.touch();
    }

    /// Get the last activity timestamp.
    pub fn last_activity(&self) -> Instant {
        self.last_activity
    }

    /// Update the last activity timestamp to now.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if the session has been idle for longer than the given duration.
    pub fn is_idle(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    /// Get the user ID if logged in.
    pub fn user_id(&self) -> Option<i64> {
        self.user_id
    }

    /// Get the username if logged in.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Check if the session is logged in.
    pub fn is_logged_in(&self) -> bool {
        self.user_id.is_some()
    }

    /// Set the logged-in user.
    pub fn set_user(&mut self, user_id: i64, username: String) {
        info!(
            "Session {} logged in as {} (user_id={})",
            self.id, username, user_id
        );
        self.user_id = Some(user_id);
        self.username = Some(username);
        self.touch();
    }

    /// Clear the logged-in user (logout).
    pub fn clear_user(&mut self) {
        if let Some(username) = &self.username {
            info!("Session {} logged out (was {})", self.id, username);
        }
        self.user_id = None;
        self.username = None;
        self.touch();
    }

    /// Get the character encoding for this session.
    pub fn encoding(&self) -> CharacterEncoding {
        self.encoding
    }

    /// Set the character encoding for this session.
    pub fn set_encoding(&mut self, encoding: CharacterEncoding) {
        debug!(
            "Session {} encoding changed: {:?} -> {:?}",
            self.id, self.encoding, encoding
        );
        self.encoding = encoding;
        self.touch();
    }

    /// Consume the session and return the TCP stream.
    pub fn into_stream(self) -> TcpStream {
        self.stream
    }

    /// Swap the TCP stream with another stream.
    ///
    /// This is useful for operations like XMODEM file transfer that need
    /// temporary ownership of the stream. The original stream is returned
    /// and can be restored later by swapping again.
    ///
    /// # Arguments
    ///
    /// * `new_stream` - The new stream to put in place
    ///
    /// # Returns
    ///
    /// The old stream that was in the session
    pub fn swap_stream(&mut self, new_stream: TcpStream) -> TcpStream {
        std::mem::replace(&mut self.stream, new_stream)
    }
}

/// Information about a session for external queries.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Session ID.
    pub id: Uuid,
    /// Peer address.
    pub peer_addr: SocketAddr,
    /// Current state.
    pub state: SessionState,
    /// Duration since last activity.
    pub idle_duration: Duration,
    /// Username if logged in.
    pub username: Option<String>,
    /// User ID if logged in.
    pub user_id: Option<i64>,
    /// Character encoding.
    pub encoding: CharacterEncoding,
    /// When the session was connected.
    pub connected_at: std::time::Instant,
    /// Whether this session should be forcefully disconnected.
    pub force_disconnect: bool,
}

/// Manager for all active sessions.
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, SessionInfo>>>,
    idle_timeout: Duration,
}

impl SessionManager {
    /// Create a new session manager with the given idle timeout.
    pub fn new(idle_timeout_secs: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            idle_timeout: Duration::from_secs(idle_timeout_secs),
        }
    }

    /// Get the idle timeout duration.
    pub fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    /// Register a new session.
    pub async fn register(&self, session: &TelnetSession) {
        let info = SessionInfo {
            id: session.id(),
            peer_addr: session.peer_addr(),
            state: session.state(),
            idle_duration: Duration::ZERO,
            username: session.username().map(String::from),
            user_id: session.user_id(),
            encoding: session.encoding(),
            connected_at: Instant::now(),
            force_disconnect: false,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id(), info);
        debug!(
            "Registered session {} (total: {})",
            session.id(),
            sessions.len()
        );
    }

    /// Update session information.
    pub async fn update(&self, session: &TelnetSession) {
        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(&session.id()) {
            info.state = session.state();
            info.idle_duration = session.last_activity().elapsed();
            info.username = session.username().map(String::from);
            info.user_id = session.user_id();
            info.encoding = session.encoding();
        }
    }

    /// Unregister a session.
    pub async fn unregister(&self, session_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        if sessions.remove(&session_id).is_some() {
            debug!(
                "Unregistered session {} (total: {})",
                session_id,
                sessions.len()
            );
        }
    }

    /// Get the number of active sessions.
    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Get information about all sessions.
    pub async fn list(&self) -> Vec<SessionInfo> {
        self.sessions.read().await.values().cloned().collect()
    }

    /// Get information about a specific session.
    pub async fn get(&self, session_id: Uuid) -> Option<SessionInfo> {
        self.sessions.read().await.get(&session_id).cloned()
    }

    /// Find sessions that have exceeded the idle timeout.
    pub async fn find_idle_sessions(&self) -> Vec<Uuid> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .filter(|(_, info)| info.idle_duration > self.idle_timeout)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get sessions by username.
    pub async fn find_by_username(&self, username: &str) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|info| info.username.as_deref() == Some(username))
            .cloned()
            .collect()
    }

    /// Check if a user is currently connected.
    pub async fn is_user_connected(&self, username: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .any(|info| info.username.as_deref() == Some(username))
    }

    /// Request a session to be forcefully disconnected.
    ///
    /// Returns true if the session was found and marked for disconnect.
    pub async fn request_disconnect(&self, session_id: Uuid) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(&session_id) {
            info.force_disconnect = true;
            info!(
                "Session {} marked for force disconnect (user: {:?})",
                session_id, info.username
            );
            true
        } else {
            false
        }
    }

    /// Check if a session should be forcefully disconnected.
    ///
    /// This is called by the session handler to check if it should terminate.
    pub async fn should_disconnect(&self, session_id: Uuid) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id)
            .is_some_and(|info| info.force_disconnect)
    }

    /// Clear the force disconnect flag for a session.
    ///
    /// This is called after the disconnect has been processed.
    pub async fn clear_disconnect_flag(&self, session_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(&session_id) {
            info.force_disconnect = false;
        }
    }

    /// Get sessions by user ID.
    pub async fn find_by_user_id(&self, user_id: i64) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|info| info.user_id == Some(user_id))
            .cloned()
            .collect()
    }
}

impl Clone for SessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
            idle_timeout: self.idle_timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    async fn create_test_session() -> TelnetSession {
        // Create a listener on a random port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Connect a client
        let client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (stream, peer_addr) = listener.accept().await.unwrap();

        drop(client); // We don't need the client side

        TelnetSession::new(stream, peer_addr)
    }

    #[tokio::test]
    async fn test_session_creation() {
        let session = create_test_session().await;

        assert_eq!(session.state(), SessionState::Welcome);
        assert!(!session.is_logged_in());
        assert!(session.user_id().is_none());
        assert!(session.username().is_none());
    }

    #[tokio::test]
    async fn test_session_state_change() {
        let mut session = create_test_session().await;

        assert_eq!(session.state(), SessionState::Welcome);

        session.set_state(SessionState::Login);
        assert_eq!(session.state(), SessionState::Login);

        session.set_state(SessionState::MainMenu);
        assert_eq!(session.state(), SessionState::MainMenu);
    }

    #[tokio::test]
    async fn test_session_login_logout() {
        let mut session = create_test_session().await;

        assert!(!session.is_logged_in());

        session.set_user(42, "testuser".to_string());
        assert!(session.is_logged_in());
        assert_eq!(session.user_id(), Some(42));
        assert_eq!(session.username(), Some("testuser"));

        session.clear_user();
        assert!(!session.is_logged_in());
        assert!(session.user_id().is_none());
        assert!(session.username().is_none());
    }

    #[tokio::test]
    async fn test_session_idle_detection() {
        let mut session = create_test_session().await;

        // Should not be idle immediately
        assert!(!session.is_idle(Duration::from_secs(1)));

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should be idle after 10ms timeout
        assert!(session.is_idle(Duration::from_millis(10)));

        // Touch the session
        session.touch();

        // Should not be idle anymore
        assert!(!session.is_idle(Duration::from_millis(100)));
    }

    #[tokio::test]
    async fn test_session_manager_register_unregister() {
        let manager = SessionManager::new(300);
        let session = create_test_session().await;
        let session_id = session.id();

        assert_eq!(manager.count().await, 0);

        manager.register(&session).await;
        assert_eq!(manager.count().await, 1);

        let info = manager.get(session_id).await;
        assert!(info.is_some());
        assert_eq!(info.unwrap().id, session_id);

        manager.unregister(session_id).await;
        assert_eq!(manager.count().await, 0);
        assert!(manager.get(session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_list() {
        let manager = SessionManager::new(300);

        let session1 = create_test_session().await;
        let session2 = create_test_session().await;

        manager.register(&session1).await;
        manager.register(&session2).await;

        let list = manager.list().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_session_manager_find_by_username() {
        let manager = SessionManager::new(300);

        let mut session1 = create_test_session().await;
        let mut session2 = create_test_session().await;

        session1.set_user(1, "user1".to_string());
        session2.set_user(2, "user2".to_string());

        manager.register(&session1).await;
        manager.register(&session2).await;

        // Update to reflect the username
        manager.update(&session1).await;
        manager.update(&session2).await;

        let found = manager.find_by_username("user1").await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].username, Some("user1".to_string()));

        assert!(manager.is_user_connected("user1").await);
        assert!(manager.is_user_connected("user2").await);
        assert!(!manager.is_user_connected("user3").await);
    }

    #[tokio::test]
    async fn test_session_state_default() {
        assert_eq!(SessionState::default(), SessionState::Welcome);
    }

    #[tokio::test]
    async fn test_session_encoding_default() {
        let session = create_test_session().await;
        // Default encoding is ShiftJIS
        assert_eq!(session.encoding(), CharacterEncoding::ShiftJIS);
    }

    #[tokio::test]
    async fn test_session_with_encoding() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (stream, peer_addr) = listener.accept().await.unwrap();
        drop(client);

        let session = TelnetSession::with_encoding(stream, peer_addr, CharacterEncoding::Utf8);
        assert_eq!(session.encoding(), CharacterEncoding::Utf8);
    }

    #[tokio::test]
    async fn test_session_set_encoding() {
        let mut session = create_test_session().await;

        assert_eq!(session.encoding(), CharacterEncoding::ShiftJIS);

        session.set_encoding(CharacterEncoding::Utf8);
        assert_eq!(session.encoding(), CharacterEncoding::Utf8);

        session.set_encoding(CharacterEncoding::ShiftJIS);
        assert_eq!(session.encoding(), CharacterEncoding::ShiftJIS);
    }

    #[tokio::test]
    async fn test_session_info_encoding() {
        let manager = SessionManager::new(300);
        let mut session = create_test_session().await;
        let session_id = session.id();

        // Register with default encoding
        manager.register(&session).await;
        let info = manager.get(session_id).await.unwrap();
        assert_eq!(info.encoding, CharacterEncoding::ShiftJIS);

        // Change encoding and update
        session.set_encoding(CharacterEncoding::Utf8);
        manager.update(&session).await;
        let info = manager.get(session_id).await.unwrap();
        assert_eq!(info.encoding, CharacterEncoding::Utf8);
    }
}
