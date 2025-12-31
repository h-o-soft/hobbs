//! Authentication session management for HOBBS.
//!
//! This module provides session tokens, login/logout functionality,
//! and login attempt rate limiting.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::User;

/// Session-related errors.
#[derive(Error, Debug)]
pub enum SessionError {
    /// Invalid credentials (wrong username or password).
    #[error("invalid credentials")]
    InvalidCredentials,

    /// Account is locked due to too many failed attempts.
    #[error("account locked for {0} seconds")]
    AccountLocked(u64),

    /// Session has expired.
    #[error("session expired")]
    SessionExpired,

    /// Session not found.
    #[error("session not found")]
    SessionNotFound,

    /// Account is inactive/suspended.
    #[error("account is inactive")]
    AccountInactive,
}

/// Default session duration (24 hours).
pub const DEFAULT_SESSION_DURATION_SECS: u64 = 24 * 60 * 60;

/// Default idle timeout (5 minutes).
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 5 * 60;

/// Maximum login attempts before lockout.
pub const MAX_LOGIN_ATTEMPTS: u32 = 3;

/// Lockout duration (5 minutes).
pub const LOCKOUT_DURATION_SECS: u64 = 5 * 60;

/// Authentication session representing a logged-in user.
#[derive(Debug, Clone)]
pub struct AuthSession {
    /// Unique session token (UUID v4).
    pub token: String,
    /// User ID associated with this session.
    pub user_id: i64,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session expires (absolute timeout).
    pub expires_at: DateTime<Utc>,
    /// Last activity timestamp (for idle timeout).
    last_activity: Instant,
}

impl AuthSession {
    /// Create a new authentication session for a user.
    pub fn new(user_id: i64) -> Self {
        Self::with_duration(user_id, Duration::from_secs(DEFAULT_SESSION_DURATION_SECS))
    }

    /// Create a new session with a custom duration.
    pub fn with_duration(user_id: i64, duration: Duration) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::from_std(duration).unwrap_or_default();

        Self {
            token: Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            expires_at,
            last_activity: Instant::now(),
        }
    }

    /// Check if the session has expired (absolute timeout).
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Check if the session has been idle too long.
    pub fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_activity.elapsed() >= idle_timeout
    }

    /// Check if the session is still valid (not expired and not idle).
    pub fn is_valid(&self, idle_timeout: Duration) -> bool {
        !self.is_expired() && !self.is_idle(idle_timeout)
    }

    /// Update the last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get the remaining time until expiration.
    pub fn remaining_time(&self) -> Option<chrono::Duration> {
        let remaining = self.expires_at - Utc::now();
        if remaining.num_seconds() > 0 {
            Some(remaining)
        } else {
            None
        }
    }

    /// Get the time since last activity.
    pub fn idle_time(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

/// Result of a login attempt rate limit check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitResult {
    /// Login attempt is allowed.
    Allowed,
    /// Account is locked for the specified duration.
    Locked(Duration),
}

/// Login attempt rate limiter.
///
/// Tracks failed login attempts per username and enforces lockout
/// after too many failures.
#[derive(Debug)]
pub struct LoginLimiter {
    /// Failed attempts per username: (username -> list of attempt times).
    attempts: HashMap<String, Vec<Instant>>,
    /// Maximum attempts before lockout.
    max_attempts: u32,
    /// Time window for counting attempts.
    window: Duration,
    /// Lockout duration after exceeding max attempts.
    lockout: Duration,
}

impl Default for LoginLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginLimiter {
    /// Create a new limiter with default settings.
    pub fn new() -> Self {
        Self {
            attempts: HashMap::new(),
            max_attempts: MAX_LOGIN_ATTEMPTS,
            window: Duration::from_secs(LOCKOUT_DURATION_SECS),
            lockout: Duration::from_secs(LOCKOUT_DURATION_SECS),
        }
    }

    /// Create a limiter with custom settings.
    pub fn with_config(max_attempts: u32, window_secs: u64, lockout_secs: u64) -> Self {
        Self {
            attempts: HashMap::new(),
            max_attempts,
            window: Duration::from_secs(window_secs),
            lockout: Duration::from_secs(lockout_secs),
        }
    }

    /// Check if a login attempt is allowed for the given username.
    pub fn check(&mut self, username: &str) -> LimitResult {
        let now = Instant::now();
        let key = username.to_lowercase();

        // Get or create the attempts list
        let attempts = self.attempts.entry(key).or_default();

        // Remove expired attempts
        attempts.retain(|t| now.duration_since(*t) < self.window);

        // Check if locked out
        if attempts.len() >= self.max_attempts as usize {
            if let Some(oldest) = attempts.first() {
                let elapsed = now.duration_since(*oldest);
                if elapsed < self.lockout {
                    let remaining = self.lockout - elapsed;
                    return LimitResult::Locked(remaining);
                }
                // Lockout expired, clear attempts
                attempts.clear();
            }
        }

        LimitResult::Allowed
    }

    /// Record a failed login attempt.
    pub fn record_failure(&mut self, username: &str) {
        let key = username.to_lowercase();
        let now = Instant::now();

        let attempts = self.attempts.entry(key.clone()).or_default();

        // Clean old attempts first
        attempts.retain(|t| now.duration_since(*t) < self.window);

        // Record this failure
        attempts.push(now);

        debug!(
            username = %username,
            attempt_count = attempts.len(),
            "Recorded failed login attempt"
        );
    }

    /// Clear all attempts for a username (call on successful login).
    pub fn clear(&mut self, username: &str) {
        let key = username.to_lowercase();
        self.attempts.remove(&key);
    }

    /// Get the number of failed attempts for a username.
    pub fn attempt_count(&mut self, username: &str) -> usize {
        let now = Instant::now();
        let key = username.to_lowercase();

        if let Some(attempts) = self.attempts.get_mut(&key) {
            attempts.retain(|t| now.duration_since(*t) < self.window);
            attempts.len()
        } else {
            0
        }
    }

    /// Clean up expired entries to prevent memory growth.
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        self.attempts.retain(|_, attempts| {
            attempts.retain(|t| now.duration_since(*t) < self.window);
            !attempts.is_empty()
        });
    }
}

/// Session manager for tracking active sessions.
#[derive(Debug)]
pub struct SessionManager {
    /// Active sessions by token.
    sessions: HashMap<String, AuthSession>,
    /// Login attempt limiter.
    limiter: LoginLimiter,
    /// Idle timeout duration.
    idle_timeout: Duration,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Create a new session manager with default settings.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            limiter: LoginLimiter::new(),
            idle_timeout: Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
        }
    }

    /// Create a session manager with a custom idle timeout.
    pub fn with_idle_timeout(idle_timeout_secs: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            limiter: LoginLimiter::new(),
            idle_timeout: Duration::from_secs(idle_timeout_secs),
        }
    }

    /// Attempt to log in a user.
    ///
    /// Returns an `AuthSession` on success, or an error on failure.
    pub fn login(
        &mut self,
        username: &str,
        password: &str,
        user: Option<&User>,
    ) -> Result<AuthSession, SessionError> {
        // Check rate limit
        match self.limiter.check(username) {
            LimitResult::Locked(remaining) => {
                warn!(
                    username = %username,
                    remaining_secs = remaining.as_secs(),
                    "Login attempt blocked: account locked"
                );
                return Err(SessionError::AccountLocked(remaining.as_secs()));
            }
            LimitResult::Allowed => {}
        }

        // Check if user exists
        let user = match user {
            Some(u) => u,
            None => {
                self.limiter.record_failure(username);
                warn!(username = %username, "Login failed: user not found");
                return Err(SessionError::InvalidCredentials);
            }
        };

        // Check if account is active
        if !user.is_active {
            warn!(username = %username, "Login failed: account inactive");
            return Err(SessionError::AccountInactive);
        }

        // Verify password
        match crate::auth::verify_password(password, &user.password) {
            Ok(()) => {
                // Password is correct
                self.limiter.clear(username);

                let session = AuthSession::new(user.id);
                let token = session.token.clone();
                self.sessions.insert(token.clone(), session.clone());

                info!(
                    username = %username,
                    user_id = user.id,
                    token = %token,
                    "Login successful"
                );

                Ok(session)
            }
            Err(_) => {
                self.limiter.record_failure(username);
                warn!(username = %username, "Login failed: wrong password");
                Err(SessionError::InvalidCredentials)
            }
        }
    }

    /// Log out a session by token.
    pub fn logout(&mut self, token: &str) -> bool {
        if let Some(session) = self.sessions.remove(token) {
            info!(
                token = %token,
                user_id = session.user_id,
                "Session logged out"
            );
            true
        } else {
            debug!(token = %token, "Logout: session not found");
            false
        }
    }

    /// Log out all sessions for a user.
    pub fn logout_user(&mut self, user_id: i64) -> usize {
        let tokens_to_remove: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.user_id == user_id)
            .map(|(t, _)| t.clone())
            .collect();

        let count = tokens_to_remove.len();
        for token in tokens_to_remove {
            self.sessions.remove(&token);
        }

        if count > 0 {
            info!(
                user_id = user_id,
                count = count,
                "All user sessions logged out"
            );
        }

        count
    }

    /// Get a session by token, validating it is still active.
    pub fn get_session(&mut self, token: &str) -> Result<&AuthSession, SessionError> {
        // First check if session exists
        if !self.sessions.contains_key(token) {
            return Err(SessionError::SessionNotFound);
        }

        // Check validity
        let session = self.sessions.get(token).unwrap();
        if !session.is_valid(self.idle_timeout) {
            // Remove expired session
            self.sessions.remove(token);
            return Err(SessionError::SessionExpired);
        }

        Ok(self.sessions.get(token).unwrap())
    }

    /// Get a mutable session by token, updating the last activity.
    pub fn touch_session(&mut self, token: &str) -> Result<&AuthSession, SessionError> {
        // First check if session exists and is valid
        if !self.sessions.contains_key(token) {
            return Err(SessionError::SessionNotFound);
        }

        {
            let session = self.sessions.get(token).unwrap();
            if !session.is_valid(self.idle_timeout) {
                self.sessions.remove(token);
                return Err(SessionError::SessionExpired);
            }
        }

        // Update last activity
        if let Some(session) = self.sessions.get_mut(token) {
            session.touch();
        }

        Ok(self.sessions.get(token).unwrap())
    }

    /// Clean up expired sessions.
    pub fn cleanup(&mut self) -> usize {
        let before = self.sessions.len();

        self.sessions.retain(|_, s| s.is_valid(self.idle_timeout));

        self.limiter.cleanup();

        let removed = before - self.sessions.len();
        if removed > 0 {
            debug!(removed = removed, "Cleaned up expired sessions");
        }
        removed
    }

    /// Get the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the number of sessions for a specific user.
    pub fn user_session_count(&self, user_id: i64) -> usize {
        self.sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .count()
    }

    /// Check if a user has any active sessions.
    pub fn user_has_session(&self, user_id: i64) -> bool {
        self.sessions.values().any(|s| s.user_id == user_id)
    }

    /// Get access to the login limiter for manual operations.
    pub fn limiter(&mut self) -> &mut LoginLimiter {
        &mut self.limiter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_auth_session_new() {
        let session = AuthSession::new(1);

        assert!(!session.token.is_empty());
        assert_eq!(session.user_id, 1);
        assert!(!session.is_expired());
        assert!(!session.is_idle(Duration::from_secs(300)));
    }

    #[test]
    fn test_auth_session_with_duration() {
        let session = AuthSession::with_duration(1, Duration::from_secs(3600));

        assert!(!session.is_expired());
        let remaining = session.remaining_time().unwrap();
        assert!(remaining.num_seconds() > 3500);
        assert!(remaining.num_seconds() <= 3600);
    }

    #[test]
    fn test_auth_session_token_uniqueness() {
        let session1 = AuthSession::new(1);
        let session2 = AuthSession::new(1);

        assert_ne!(session1.token, session2.token);
    }

    #[test]
    fn test_auth_session_touch() {
        let mut session = AuthSession::new(1);

        // Wait a bit to ensure idle_time is measurable
        sleep(Duration::from_millis(50));
        let idle_before_touch = session.idle_time();
        assert!(idle_before_touch >= Duration::from_millis(50));

        // Touch should reset idle time
        session.touch();
        let idle_after_touch = session.idle_time();

        assert!(idle_after_touch < idle_before_touch);
    }

    #[test]
    fn test_auth_session_idle_check() {
        let session = AuthSession::new(1);

        // With a very short timeout, should be idle after sleeping
        assert!(!session.is_idle(Duration::from_secs(10)));

        // With an extremely short timeout
        sleep(Duration::from_millis(10));
        assert!(session.is_idle(Duration::from_millis(5)));
    }

    #[test]
    fn test_login_limiter_allows_initial_attempts() {
        let mut limiter = LoginLimiter::new();

        assert_eq!(limiter.check("testuser"), LimitResult::Allowed);
        assert_eq!(limiter.check("testuser"), LimitResult::Allowed);
    }

    #[test]
    fn test_login_limiter_locks_after_max_attempts() {
        let mut limiter = LoginLimiter::with_config(3, 60, 60);

        // Record 3 failures
        limiter.record_failure("testuser");
        limiter.record_failure("testuser");
        limiter.record_failure("testuser");

        // Should be locked
        match limiter.check("testuser") {
            LimitResult::Locked(duration) => {
                assert!(duration.as_secs() > 0);
            }
            LimitResult::Allowed => panic!("Expected account to be locked"),
        }
    }

    #[test]
    fn test_login_limiter_case_insensitive() {
        let mut limiter = LoginLimiter::with_config(3, 60, 60);

        limiter.record_failure("TestUser");
        limiter.record_failure("TESTUSER");
        limiter.record_failure("testuser");

        match limiter.check("TeStUsEr") {
            LimitResult::Locked(_) => {}
            LimitResult::Allowed => panic!("Expected account to be locked"),
        }
    }

    #[test]
    fn test_login_limiter_clear() {
        let mut limiter = LoginLimiter::with_config(3, 60, 60);

        limiter.record_failure("testuser");
        limiter.record_failure("testuser");
        assert_eq!(limiter.attempt_count("testuser"), 2);

        limiter.clear("testuser");
        assert_eq!(limiter.attempt_count("testuser"), 0);
    }

    #[test]
    fn test_login_limiter_cleanup() {
        let mut limiter = LoginLimiter::with_config(3, 1, 1); // 1 second window

        limiter.record_failure("user1");
        limiter.record_failure("user2");

        // Wait for expiry
        sleep(Duration::from_millis(1100));

        limiter.cleanup();

        assert_eq!(limiter.attempt_count("user1"), 0);
        assert_eq!(limiter.attempt_count("user2"), 0);
    }

    #[test]
    fn test_session_manager_logout() {
        let mut manager = SessionManager::new();
        let session = AuthSession::new(1);
        let token = session.token.clone();
        manager.sessions.insert(token.clone(), session);

        assert!(manager.logout(&token));
        assert!(!manager.logout(&token)); // Already logged out
    }

    #[test]
    fn test_session_manager_logout_user() {
        let mut manager = SessionManager::new();

        // Add multiple sessions for same user
        let session1 = AuthSession::new(1);
        let session2 = AuthSession::new(1);
        let session3 = AuthSession::new(2);

        manager.sessions.insert(session1.token.clone(), session1);
        manager.sessions.insert(session2.token.clone(), session2);
        manager.sessions.insert(session3.token.clone(), session3);

        assert_eq!(manager.session_count(), 3);
        assert_eq!(manager.logout_user(1), 2);
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_session_manager_get_session() {
        let mut manager = SessionManager::new();
        let session = AuthSession::new(1);
        let token = session.token.clone();
        manager.sessions.insert(token.clone(), session);

        assert!(manager.get_session(&token).is_ok());
        assert!(manager.get_session("invalid").is_err());
    }

    #[test]
    fn test_session_manager_touch_session() {
        let mut manager = SessionManager::new();
        let session = AuthSession::new(1);
        let token = session.token.clone();
        manager.sessions.insert(token.clone(), session);

        sleep(Duration::from_millis(10));

        // Touch should update last activity
        let session = manager.touch_session(&token).unwrap();
        assert!(session.idle_time() < Duration::from_millis(10));
    }

    #[test]
    fn test_session_manager_user_session_count() {
        let mut manager = SessionManager::new();

        let session1 = AuthSession::new(1);
        let session2 = AuthSession::new(1);
        let session3 = AuthSession::new(2);

        manager.sessions.insert(session1.token.clone(), session1);
        manager.sessions.insert(session2.token.clone(), session2);
        manager.sessions.insert(session3.token.clone(), session3);

        assert_eq!(manager.user_session_count(1), 2);
        assert_eq!(manager.user_session_count(2), 1);
        assert_eq!(manager.user_session_count(3), 0);
    }

    #[test]
    fn test_session_error_display() {
        assert_eq!(
            SessionError::InvalidCredentials.to_string(),
            "invalid credentials"
        );
        assert_eq!(
            SessionError::AccountLocked(300).to_string(),
            "account locked for 300 seconds"
        );
        assert_eq!(SessionError::SessionExpired.to_string(), "session expired");
        assert_eq!(
            SessionError::SessionNotFound.to_string(),
            "session not found"
        );
        assert_eq!(
            SessionError::AccountInactive.to_string(),
            "account is inactive"
        );
    }
}
