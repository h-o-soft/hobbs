//! Authentication module for HOBBS.
//!
//! This module provides password hashing, session management,
//! user registration, and authentication utilities.

mod password;
mod registration;
mod session;
pub mod validation;

pub use password::{hash_password, validate_password, verify_password, PasswordError};
pub use registration::{register, register_with_role, RegistrationError, RegistrationRequest};
pub use session::{
    AuthSession, LimitResult, LoginLimiter, SessionError, SessionManager,
    DEFAULT_IDLE_TIMEOUT_SECS, DEFAULT_SESSION_DURATION_SECS, LOCKOUT_DURATION_SECS,
    MAX_LOGIN_ATTEMPTS,
};
pub use validation::ValidationError;
