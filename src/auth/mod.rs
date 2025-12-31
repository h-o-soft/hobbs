//! Authentication module for HOBBS.
//!
//! This module provides password hashing, session management,
//! user registration, permission checking, profile management,
//! and authentication utilities.

mod password;
pub mod permission;
mod profile;
mod registration;
mod session;
pub mod validation;

pub use password::{hash_password, validate_password, verify_password, PasswordError};
pub use permission::{
    can_modify_resource, check_permission, require_member, require_subop, require_sysop,
    PermissionError,
};
pub use profile::{
    change_password, get_profile, get_profile_by_username, reset_password, update_profile,
    ProfileError, ProfileUpdateRequest, UserProfile, MAX_PROFILE_LENGTH,
};
pub use registration::{register, register_with_role, RegistrationError, RegistrationRequest};
pub use session::{
    AuthSession, LimitResult, LoginLimiter, SessionError, SessionManager,
    DEFAULT_IDLE_TIMEOUT_SECS, DEFAULT_SESSION_DURATION_SECS, LOCKOUT_DURATION_SECS,
    MAX_LOGIN_ATTEMPTS,
};
pub use validation::ValidationError;
