//! Authentication module for HOBBS.
//!
//! This module provides password hashing, session management,
//! and authentication utilities.

mod password;
mod session;

pub use password::{hash_password, validate_password, verify_password, PasswordError};
pub use session::{
    AuthSession, LimitResult, LoginLimiter, SessionError, SessionManager,
    DEFAULT_IDLE_TIMEOUT_SECS, DEFAULT_SESSION_DURATION_SECS, LOCKOUT_DURATION_SECS,
    MAX_LOGIN_ATTEMPTS,
};
