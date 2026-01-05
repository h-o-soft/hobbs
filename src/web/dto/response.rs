//! Response DTOs for Web API.

use serde::Serialize;

// ============================================================================
// Generic Response Wrappers
// ============================================================================

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

impl<T: Serialize> PaginatedResponse<T> {
    /// Create a new paginated response.
    pub fn new(data: Vec<T>, page: u32, per_page: u32, total: u64) -> Self {
        Self {
            data,
            meta: PaginationMeta {
                page,
                per_page,
                total,
            },
        }
    }
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

// ============================================================================
// Board DTOs
// ============================================================================

/// Board response.
#[derive(Debug, Serialize)]
pub struct BoardResponse {
    /// Board ID.
    pub id: i64,
    /// Board name.
    pub name: String,
    /// Board description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Board type (thread or flat).
    pub board_type: String,
    /// Whether user can read this board.
    pub can_read: bool,
    /// Whether user can write to this board.
    pub can_write: bool,
    /// Creation timestamp.
    pub created_at: String,
}

/// Thread response.
#[derive(Debug, Serialize)]
pub struct ThreadResponse {
    /// Thread ID.
    pub id: i64,
    /// Board ID.
    pub board_id: i64,
    /// Thread title.
    pub title: String,
    /// Author info.
    pub author: AuthorInfo,
    /// Number of posts.
    pub post_count: i32,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Post response.
#[derive(Debug, Serialize)]
pub struct PostResponse {
    /// Post ID.
    pub id: i64,
    /// Board ID.
    pub board_id: i64,
    /// Thread ID (None for flat boards).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<i64>,
    /// Author info.
    pub author: AuthorInfo,
    /// Post title (for flat boards).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Post body.
    pub body: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Author information.
#[derive(Debug, Serialize)]
pub struct AuthorInfo {
    /// User ID.
    pub id: i64,
    /// Username.
    pub username: String,
    /// Nickname.
    pub nickname: String,
}

// ============================================================================
// Mail DTOs
// ============================================================================

/// Mail list item response.
#[derive(Debug, Serialize)]
pub struct MailListResponse {
    /// Mail ID.
    pub id: i64,
    /// Sender info.
    pub sender: AuthorInfo,
    /// Recipient info.
    pub recipient: AuthorInfo,
    /// Mail subject.
    pub subject: String,
    /// Whether the mail has been read.
    pub is_read: bool,
    /// Creation timestamp.
    pub created_at: String,
}

/// Mail detail response.
#[derive(Debug, Serialize)]
pub struct MailDetailResponse {
    /// Mail ID.
    pub id: i64,
    /// Sender info.
    pub sender: AuthorInfo,
    /// Recipient info.
    pub recipient: AuthorInfo,
    /// Mail subject.
    pub subject: String,
    /// Mail body.
    pub body: String,
    /// Whether the mail has been read.
    pub is_read: bool,
    /// Creation timestamp.
    pub created_at: String,
}

/// Unread count response.
#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    /// Unread mail count.
    pub count: u64,
}

// ============================================================================
// User DTOs
// ============================================================================

/// User list response.
#[derive(Debug, Serialize)]
pub struct UserListResponse {
    /// User ID.
    pub id: i64,
    /// Username.
    pub username: String,
    /// Nickname.
    pub nickname: String,
    /// User role.
    pub role: String,
    /// Last login timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
}

/// User detail response.
#[derive(Debug, Serialize)]
pub struct UserDetailResponse {
    /// User ID.
    pub id: i64,
    /// Username.
    pub username: String,
    /// Nickname.
    pub nickname: String,
    /// User role.
    pub role: String,
    /// Profile text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Account creation timestamp.
    pub created_at: String,
    /// Last login timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
}

// ============================================================================
// RSS DTOs
// ============================================================================

/// RSS feed response.
#[derive(Debug, Serialize)]
pub struct RssFeedResponse {
    /// Feed ID.
    pub id: i64,
    /// Feed URL.
    pub url: String,
    /// Feed title.
    pub title: String,
    /// Feed description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Site URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_url: Option<String>,
    /// Last fetched timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_fetched_at: Option<String>,
    /// Whether the feed is active.
    pub is_active: bool,
}

/// RSS item response.
#[derive(Debug, Serialize)]
pub struct RssItemResponse {
    /// Item ID.
    pub id: i64,
    /// Feed ID.
    pub feed_id: i64,
    /// Item title.
    pub title: String,
    /// Item link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Item description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Item author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Published timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}
