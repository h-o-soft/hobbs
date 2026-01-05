//! Request DTOs for Web API.

use serde::Deserialize;

/// Login request.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Username.
    pub username: String,
    /// Password.
    pub password: String,
}

/// Logout request.
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    /// Refresh token to invalidate.
    pub refresh_token: String,
}

/// Token refresh request.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    /// Refresh token.
    pub refresh_token: String,
}

/// User registration request.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Username.
    pub username: String,
    /// Password.
    pub password: String,
    /// Nickname.
    pub nickname: String,
    /// Email (optional).
    #[serde(default)]
    pub email: Option<String>,
}
