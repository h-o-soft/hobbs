//! Session management for administrators.
//!
//! This module provides administrative functions for managing connected sessions:
//! - List connected sessions (SubOp and above)
//! - Get session details (SubOp and above)
//! - Force disconnect (SysOp only)

use std::time::Duration;

use uuid::Uuid;

use crate::auth::require_sysop;
use crate::db::User;
use crate::server::{SessionInfo, SessionManager, SessionState};

use super::{require_admin, AdminError};

/// Admin service for session management.
///
/// This service provides administrative functions for managing connected sessions.
/// It wraps the `SessionManager` and adds permission checks.
pub struct SessionAdminService {
    session_manager: SessionManager,
}

impl SessionAdminService {
    /// Create a new SessionAdminService.
    pub fn new(session_manager: SessionManager) -> Self {
        Self { session_manager }
    }

    /// Get the underlying session manager.
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// List all connected sessions.
    ///
    /// Requires SubOp or higher permission.
    pub async fn list_sessions(&self, admin: &User) -> Result<Vec<SessionInfo>, AdminError> {
        require_admin(Some(admin))?;
        Ok(self.session_manager.list().await)
    }

    /// Get the number of connected sessions.
    ///
    /// Requires SubOp or higher permission.
    pub async fn connection_count(&self, admin: &User) -> Result<usize, AdminError> {
        require_admin(Some(admin))?;
        Ok(self.session_manager.count().await)
    }

    /// Get a specific session by ID.
    ///
    /// Requires SubOp or higher permission.
    pub async fn get_session(
        &self,
        session_id: Uuid,
        admin: &User,
    ) -> Result<SessionInfo, AdminError> {
        require_admin(Some(admin))?;

        self.session_manager
            .get(session_id)
            .await
            .ok_or_else(|| AdminError::NotFound("セッション".to_string()))
    }

    /// Find sessions by username.
    ///
    /// Requires SubOp or higher permission.
    pub async fn find_sessions_by_username(
        &self,
        username: &str,
        admin: &User,
    ) -> Result<Vec<SessionInfo>, AdminError> {
        require_admin(Some(admin))?;
        Ok(self.session_manager.find_by_username(username).await)
    }

    /// Find sessions by user ID.
    ///
    /// Requires SubOp or higher permission.
    pub async fn find_sessions_by_user_id(
        &self,
        user_id: i64,
        admin: &User,
    ) -> Result<Vec<SessionInfo>, AdminError> {
        require_admin(Some(admin))?;
        Ok(self.session_manager.find_by_user_id(user_id).await)
    }

    /// Check if a user is currently connected.
    ///
    /// Requires SubOp or higher permission.
    pub async fn is_user_connected(
        &self,
        username: &str,
        admin: &User,
    ) -> Result<bool, AdminError> {
        require_admin(Some(admin))?;
        Ok(self.session_manager.is_user_connected(username).await)
    }

    /// Force disconnect a session.
    ///
    /// Requires SysOp permission.
    /// Returns true if the session was found and marked for disconnect.
    ///
    /// Note: This only marks the session for disconnect. The actual disconnection
    /// happens when the session handler checks the flag and terminates.
    pub async fn force_disconnect(
        &self,
        session_id: Uuid,
        admin: &User,
    ) -> Result<bool, AdminError> {
        require_sysop(Some(admin))?;

        // Get session info first to check if trying to disconnect self
        if let Some(info) = self.session_manager.get(session_id).await {
            // Don't allow disconnecting own session
            if info.user_id == Some(admin.id) {
                return Err(AdminError::CannotModifySelf);
            }
        }

        let found = self.session_manager.request_disconnect(session_id).await;
        if found {
            Ok(true)
        } else {
            Err(AdminError::NotFound("セッション".to_string()))
        }
    }

    /// Force disconnect all sessions for a user.
    ///
    /// Requires SysOp permission.
    /// Returns the number of sessions marked for disconnect.
    pub async fn force_disconnect_user(
        &self,
        user_id: i64,
        admin: &User,
    ) -> Result<usize, AdminError> {
        require_sysop(Some(admin))?;

        // Don't allow disconnecting own sessions
        if user_id == admin.id {
            return Err(AdminError::CannotModifySelf);
        }

        let sessions = self.session_manager.find_by_user_id(user_id).await;
        let mut count = 0;

        for session in sessions {
            if self.session_manager.request_disconnect(session.id).await {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get sessions that have exceeded the idle timeout.
    ///
    /// Requires SubOp or higher permission.
    pub async fn find_idle_sessions(&self, admin: &User) -> Result<Vec<Uuid>, AdminError> {
        require_admin(Some(admin))?;
        Ok(self.session_manager.find_idle_sessions().await)
    }

    /// Get sessions in a specific state.
    ///
    /// Requires SubOp or higher permission.
    pub async fn find_sessions_by_state(
        &self,
        state: SessionState,
        admin: &User,
    ) -> Result<Vec<SessionInfo>, AdminError> {
        require_admin(Some(admin))?;

        let sessions = self.session_manager.list().await;
        Ok(sessions.into_iter().filter(|s| s.state == state).collect())
    }

    /// Get session statistics.
    ///
    /// Requires SubOp or higher permission.
    pub async fn get_statistics(&self, admin: &User) -> Result<SessionStatistics, AdminError> {
        require_admin(Some(admin))?;

        let sessions = self.session_manager.list().await;

        let total = sessions.len();
        let logged_in = sessions.iter().filter(|s| s.user_id.is_some()).count();
        let guests = total - logged_in;

        let in_chat = sessions
            .iter()
            .filter(|s| s.state == SessionState::Chat)
            .count();
        let in_board = sessions
            .iter()
            .filter(|s| s.state == SessionState::Board)
            .count();
        let in_mail = sessions
            .iter()
            .filter(|s| s.state == SessionState::Mail)
            .count();
        let in_files = sessions
            .iter()
            .filter(|s| s.state == SessionState::Files)
            .count();
        let in_admin = sessions
            .iter()
            .filter(|s| s.state == SessionState::Admin)
            .count();

        let idle_timeout = self.session_manager.idle_timeout();
        let idle = sessions
            .iter()
            .filter(|s| s.idle_duration > idle_timeout)
            .count();

        Ok(SessionStatistics {
            total,
            logged_in,
            guests,
            in_chat,
            in_board,
            in_mail,
            in_files,
            in_admin,
            idle,
        })
    }
}

/// Statistics about connected sessions.
#[derive(Debug, Clone)]
pub struct SessionStatistics {
    /// Total number of connected sessions.
    pub total: usize,
    /// Number of logged-in users.
    pub logged_in: usize,
    /// Number of guest (not logged in) sessions.
    pub guests: usize,
    /// Number of users in chat rooms.
    pub in_chat: usize,
    /// Number of users in board view.
    pub in_board: usize,
    /// Number of users in mail.
    pub in_mail: usize,
    /// Number of users in file management.
    pub in_files: usize,
    /// Number of users in admin menu.
    pub in_admin: usize,
    /// Number of idle sessions.
    pub idle: usize,
}

/// Format session state for display.
pub fn format_session_state(state: SessionState) -> &'static str {
    match state {
        SessionState::Welcome => "接続中",
        SessionState::Login => "ログイン画面",
        SessionState::Registration => "新規登録",
        SessionState::MainMenu => "メインメニュー",
        SessionState::Board => "掲示板",
        SessionState::Chat => "チャット",
        SessionState::Mail => "メール",
        SessionState::Files => "ファイル",
        SessionState::Admin => "管理メニュー",
        SessionState::Script => "スクリプト",
        SessionState::News => "ニュース",
        SessionState::Closing => "切断中",
    }
}

/// Format duration for display (e.g., "5分23秒").
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs < 60 {
        format!("{total_secs}秒")
    } else if total_secs < 3600 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if secs == 0 {
            format!("{mins}分")
        } else {
            format!("{mins}分{secs}秒")
        }
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if mins == 0 {
            format!("{hours}時間")
        } else {
            format!("{hours}時間{mins}分")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Role;
    use crate::server::CharacterEncoding;
    use std::net::SocketAddr;
    use std::time::Instant;

    fn create_test_user(id: i64, role: Role) -> User {
        User {
            id,
            username: format!("user{id}"),
            password: "hash".to_string(),
            nickname: format!("User {id}"),
            email: None,
            role,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            language: "en".to_string(),
            auto_paging: true,
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        }
    }

    fn create_test_session_info(
        id: Uuid,
        username: Option<&str>,
        user_id: Option<i64>,
    ) -> SessionInfo {
        SessionInfo {
            id,
            peer_addr: "127.0.0.1:12345".parse::<SocketAddr>().unwrap(),
            state: SessionState::MainMenu,
            idle_duration: Duration::from_secs(10),
            username: username.map(String::from),
            user_id,
            encoding: CharacterEncoding::default(),
            connected_at: Instant::now(),
            force_disconnect: false,
        }
    }

    // format_session_state tests
    #[test]
    fn test_format_session_state() {
        assert_eq!(format_session_state(SessionState::Welcome), "接続中");
        assert_eq!(format_session_state(SessionState::Login), "ログイン画面");
        assert_eq!(
            format_session_state(SessionState::MainMenu),
            "メインメニュー"
        );
        assert_eq!(format_session_state(SessionState::Board), "掲示板");
        assert_eq!(format_session_state(SessionState::Chat), "チャット");
        assert_eq!(format_session_state(SessionState::Mail), "メール");
        assert_eq!(format_session_state(SessionState::Files), "ファイル");
        assert_eq!(format_session_state(SessionState::Admin), "管理メニュー");
        assert_eq!(format_session_state(SessionState::Closing), "切断中");
    }

    // format_duration tests
    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0秒");
        assert_eq!(format_duration(Duration::from_secs(30)), "30秒");
        assert_eq!(format_duration(Duration::from_secs(59)), "59秒");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1分");
        assert_eq!(format_duration(Duration::from_secs(90)), "1分30秒");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59分59秒");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1時間");
        assert_eq!(format_duration(Duration::from_secs(3660)), "1時間1分");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2時間");
        assert_eq!(format_duration(Duration::from_secs(7320)), "2時間2分");
    }

    // SessionAdminService tests (using tokio)
    #[tokio::test]
    async fn test_list_sessions_as_subop() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let subop = create_test_user(1, Role::SubOp);

        let result = service.list_sessions(&subop).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_list_sessions_as_member_fails() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let member = create_test_user(1, Role::Member);

        let result = service.list_sessions(&member).await;
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[tokio::test]
    async fn test_connection_count() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let subop = create_test_user(1, Role::SubOp);

        let count = service.connection_count(&subop).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let subop = create_test_user(1, Role::SubOp);

        let result = service.get_session(Uuid::new_v4(), &subop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_force_disconnect_as_subop_fails() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let subop = create_test_user(1, Role::SubOp);

        let result = service.force_disconnect(Uuid::new_v4(), &subop).await;
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[tokio::test]
    async fn test_force_disconnect_not_found() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let sysop = create_test_user(1, Role::SysOp);

        let result = service.force_disconnect(Uuid::new_v4(), &sysop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_force_disconnect_user_as_subop_fails() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let subop = create_test_user(1, Role::SubOp);

        let result = service.force_disconnect_user(2, &subop).await;
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[tokio::test]
    async fn test_force_disconnect_user_self_fails() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let sysop = create_test_user(1, Role::SysOp);

        let result = service.force_disconnect_user(sysop.id, &sysop).await;
        assert!(matches!(result, Err(AdminError::CannotModifySelf)));
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let subop = create_test_user(1, Role::SubOp);

        let stats = service.get_statistics(&subop).await.unwrap();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.logged_in, 0);
        assert_eq!(stats.guests, 0);
    }

    #[tokio::test]
    async fn test_find_idle_sessions() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let subop = create_test_user(1, Role::SubOp);

        let idle = service.find_idle_sessions(&subop).await.unwrap();
        assert_eq!(idle.len(), 0);
    }

    #[tokio::test]
    async fn test_is_user_connected() {
        let manager = SessionManager::new(300);
        let service = SessionAdminService::new(manager);
        let subop = create_test_user(1, Role::SubOp);

        let connected = service.is_user_connected("testuser", &subop).await.unwrap();
        assert!(!connected);
    }

    // SessionStatistics tests
    #[test]
    fn test_session_statistics_fields() {
        let stats = SessionStatistics {
            total: 10,
            logged_in: 8,
            guests: 2,
            in_chat: 3,
            in_board: 2,
            in_mail: 1,
            in_files: 1,
            in_admin: 1,
            idle: 2,
        };

        assert_eq!(stats.total, 10);
        assert_eq!(stats.logged_in, 8);
        assert_eq!(stats.guests, 2);
        assert_eq!(stats.in_chat, 3);
    }
}
