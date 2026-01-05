//! Request DTOs for Web API.

use serde::Deserialize;

// ============================================================================
// Auth DTOs
// ============================================================================

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

// ============================================================================
// Pagination DTOs
// ============================================================================

/// Pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page.
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

impl PaginationQuery {
    /// Convert to offset and limit.
    pub fn to_offset_limit(&self) -> (i64, i64) {
        let page = self.page.max(1) as i64;
        let per_page = self.per_page.clamp(1, 100) as i64;
        let offset = (page - 1) * per_page;
        (offset, per_page)
    }
}

// ============================================================================
// Board DTOs
// ============================================================================

/// Create thread request.
#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    /// Thread title.
    pub title: String,
    /// First post body.
    pub body: String,
}

/// Create post request (for thread-based boards).
#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    /// Post body.
    pub body: String,
}

/// Create flat post request (for flat boards).
#[derive(Debug, Deserialize)]
pub struct CreateFlatPostRequest {
    /// Post title.
    pub title: String,
    /// Post body.
    pub body: String,
}

// ============================================================================
// Mail DTOs
// ============================================================================

/// Send mail request.
#[derive(Debug, Deserialize)]
pub struct SendMailRequest {
    /// Recipient username or user ID.
    pub recipient: String,
    /// Mail subject.
    pub subject: String,
    /// Mail body.
    pub body: String,
}

// ============================================================================
// User DTOs
// ============================================================================

/// Update profile request.
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    /// New nickname.
    #[serde(default)]
    pub nickname: Option<String>,
    /// New email.
    #[serde(default)]
    pub email: Option<String>,
    /// New profile text.
    #[serde(default)]
    pub profile: Option<String>,
}

/// Change password request.
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    /// Current password.
    pub current_password: String,
    /// New password.
    pub new_password: String,
}
