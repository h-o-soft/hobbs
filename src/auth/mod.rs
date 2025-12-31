//! Authentication module for HOBBS.
//!
//! This module provides password hashing and authentication utilities.

mod password;

pub use password::{hash_password, validate_password, verify_password, PasswordError};
