//! Response DTOs for Web API.

use serde::Serialize;

/// Generic API response wrapper.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// Response data.
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a new API response.
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    /// Response data.
    pub data: Vec<T>,
    /// Pagination metadata.
    pub meta: PaginationMeta,
}

/// Pagination metadata.
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    /// Current page number.
    pub page: u32,
    /// Items per page.
    pub per_page: u32,
    /// Total number of items.
    pub total: u64,
}

/// Login response.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// Access token (JWT).
    pub access_token: String,
    /// Refresh token.
    pub refresh_token: String,
    /// Access token expiry in seconds.
    pub expires_in: u64,
    /// User information.
    pub user: UserInfo,
}

/// User information in responses.
#[derive(Debug, Serialize)]
pub struct UserInfo {
    /// User ID.
    pub id: i64,
    /// Username.
    pub username: String,
    /// Nickname.
    pub nickname: String,
    /// User role.
    pub role: String,
}

/// Token refresh response.
#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    /// New access token.
    pub access_token: String,
    /// New refresh token.
    pub refresh_token: String,
    /// Expiry in seconds.
    pub expires_in: u64,
}

/// Current user response (for /api/auth/me).
#[derive(Debug, Serialize)]
pub struct MeResponse {
    /// User ID.
    pub id: i64,
    /// Username.
    pub username: String,
    /// Nickname.
    pub nickname: String,
    /// User role.
    pub role: String,
    /// Email address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Unread mail count.
    pub unread_mail_count: u64,
    /// Account creation timestamp.
    pub created_at: String,
    /// Last login timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
}
