//! Request DTOs for Web API.

use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

use super::validation::{no_control_chars, not_empty_trimmed};

// ============================================================================
// Auth DTOs
// ============================================================================

/// Login request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct LoginRequest {
    /// Username.
    #[validate(length(min = 1, max = 16, message = "Username must be 1-16 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    pub username: String,
    /// Password.
    #[validate(length(min = 1, max = 128, message = "Password must be 1-128 characters"))]
    pub password: String,
}

/// Logout request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct LogoutRequest {
    /// Refresh token to invalidate.
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

/// Token refresh request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RefreshRequest {
    /// Refresh token.
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

/// One-time token request.
///
/// Used to obtain a short-lived token for WebSocket connections or file downloads.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct OneTimeTokenRequest {
    /// Token purpose: "websocket" or "download".
    #[validate(custom(function = "validate_token_purpose"))]
    pub purpose: String,
    /// Optional target ID (e.g., file_id for downloads).
    #[serde(default)]
    pub target_id: Option<i64>,
}

/// Validate token purpose.
fn validate_token_purpose(purpose: &str) -> Result<(), validator::ValidationError> {
    match purpose {
        "websocket" | "download" => Ok(()),
        _ => Err(validator::ValidationError::new("invalid_purpose")),
    }
}

/// User registration request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RegisterRequest {
    /// Username.
    #[validate(length(min = 1, max = 16, message = "Username must be 1-16 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub username: String,
    /// Password.
    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: String,
    /// Nickname.
    #[validate(length(min = 1, max = 20, message = "Nickname must be 1-20 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub nickname: String,
    /// Email (optional).
    #[serde(default)]
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
}

// ============================================================================
// Pagination DTOs
// ============================================================================

/// Pagination query parameters.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct PaginationQuery {
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    #[validate(range(min = 1, message = "Page must be 1 or greater"))]
    pub page: u32,
    /// Items per page.
    #[serde(default = "default_per_page")]
    #[validate(range(min = 1, max = 100, message = "Items per page must be 1-100"))]
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
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateThreadRequest {
    /// Thread title.
    #[validate(length(min = 1, max = 50, message = "Title must be 1-50 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub title: String,
    /// First post body.
    #[validate(length(min = 1, max = 10000, message = "Body must be 1-10000 characters"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub body: String,
}

/// Create post request (for thread-based boards).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreatePostRequest {
    /// Post body.
    #[validate(length(min = 1, max = 10000, message = "Body must be 1-10000 characters"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub body: String,
}

/// Create flat post request (for flat boards).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateFlatPostRequest {
    /// Post title.
    #[validate(length(min = 1, max = 50, message = "Title must be 1-50 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub title: String,
    /// Post body.
    #[validate(length(min = 1, max = 10000, message = "Body must be 1-10000 characters"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub body: String,
}

/// Update post request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdatePostRequest {
    /// Post title (optional, for flat board posts).
    #[validate(length(max = 50, message = "Title must be at most 50 characters"))]
    #[serde(default)]
    pub title: Option<String>,
    /// Post body.
    #[validate(length(min = 1, max = 10000, message = "Body must be 1-10000 characters"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub body: String,
}

/// Update thread request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateThreadRequest {
    /// Thread title.
    #[validate(length(min = 1, max = 50, message = "Title must be 1-50 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub title: String,
}

// ============================================================================
// Mail DTOs
// ============================================================================

/// Send mail request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct SendMailRequest {
    /// Recipient username or user ID.
    #[validate(length(min = 1, max = 16, message = "Recipient must be 1-16 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    pub recipient: String,
    /// Mail subject.
    #[validate(length(min = 1, max = 50, message = "Subject must be 1-50 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub subject: String,
    /// Mail body.
    #[validate(length(min = 1, max = 10000, message = "Body must be 1-10000 characters"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub body: String,
}

// ============================================================================
// User DTOs
// ============================================================================

/// Update profile request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateProfileRequest {
    /// New nickname.
    #[serde(default)]
    #[validate(length(min = 1, max = 20, message = "Nickname must be 1-20 characters"))]
    pub nickname: Option<String>,
    /// New email.
    #[serde(default)]
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    /// New profile text.
    #[serde(default)]
    #[validate(length(max = 1000, message = "Profile must be 1000 characters or less"))]
    pub profile: Option<String>,
}

/// Change password request.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct ChangePasswordRequest {
    /// Current password.
    #[validate(length(
        min = 1,
        max = 128,
        message = "Current password must be 1-128 characters"
    ))]
    pub current_password: String,
    /// New password.
    #[validate(length(min = 8, max = 128, message = "New password must be 8-128 characters"))]
    pub new_password: String,
}

// ============================================================================
// Admin DTOs
// ============================================================================

/// Update user request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminUpdateUserRequest {
    /// New nickname.
    #[serde(default)]
    #[validate(length(min = 1, max = 20, message = "Nickname must be 1-20 characters"))]
    pub nickname: Option<String>,
    /// New email.
    #[serde(default)]
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    /// New profile text.
    #[serde(default)]
    #[validate(length(max = 1000, message = "Profile must be 1000 characters or less"))]
    pub profile: Option<String>,
}

/// Update user role request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminUpdateRoleRequest {
    /// New role (guest, member, subop, sysop).
    #[validate(length(min = 1, message = "Role is required"))]
    #[validate(custom(function = "no_control_chars"))]
    pub role: String,
}

/// Update user status request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminUpdateStatusRequest {
    /// Whether the user is active.
    pub is_active: bool,
}

/// Reset password request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminResetPasswordRequest {
    /// New password.
    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub new_password: String,
}

/// Create board request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminCreateBoardRequest {
    /// Board name.
    #[validate(length(min = 1, max = 50, message = "Board name must be 1-50 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub name: String,
    /// Board description.
    #[serde(default)]
    #[validate(length(max = 500, message = "Description must be 500 characters or less"))]
    pub description: Option<String>,
    /// Board type (thread or flat).
    #[serde(default = "default_board_type")]
    #[validate(length(min = 1, message = "Board type is required"))]
    pub board_type: String,
    /// Minimum role required to read.
    #[serde(default = "default_guest_role")]
    #[validate(length(min = 1, message = "Minimum read role is required"))]
    pub min_read_role: String,
    /// Minimum role required to write.
    #[serde(default = "default_member_role")]
    #[validate(length(min = 1, message = "Minimum write role is required"))]
    pub min_write_role: String,
    /// Sort order.
    #[serde(default)]
    pub sort_order: i32,
    /// Whether auto-paging is disabled for this board.
    #[serde(default)]
    pub disable_paging: bool,
}

fn default_board_type() -> String {
    "thread".to_string()
}

fn default_guest_role() -> String {
    "guest".to_string()
}

fn default_member_role() -> String {
    "member".to_string()
}

/// Update board request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminUpdateBoardRequest {
    /// Board name.
    #[serde(default)]
    #[validate(length(min = 1, max = 50, message = "Board name must be 1-50 characters"))]
    pub name: Option<String>,
    /// Board description.
    #[serde(default)]
    pub description: Option<Option<String>>,
    /// Board type (thread or flat).
    #[serde(default)]
    #[validate(length(min = 1, message = "Board type must not be empty"))]
    pub board_type: Option<String>,
    /// Minimum role required to read.
    #[serde(default)]
    #[validate(length(min = 1, message = "Minimum read role must not be empty"))]
    pub min_read_role: Option<String>,
    /// Minimum role required to write.
    #[serde(default)]
    #[validate(length(min = 1, message = "Minimum write role must not be empty"))]
    pub min_write_role: Option<String>,
    /// Sort order.
    #[serde(default)]
    pub sort_order: Option<i32>,
    /// Whether the board is active.
    #[serde(default)]
    pub is_active: Option<bool>,
    /// Whether auto-paging is disabled for this board.
    #[serde(default)]
    pub disable_paging: Option<bool>,
}

/// Create folder request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminCreateFolderRequest {
    /// Folder name.
    #[validate(length(min = 1, max = 50, message = "Folder name must be 1-50 characters"))]
    #[validate(custom(function = "no_control_chars"))]
    #[validate(custom(function = "not_empty_trimmed"))]
    pub name: String,
    /// Folder description.
    #[serde(default)]
    #[validate(length(max = 500, message = "Description must be 500 characters or less"))]
    pub description: Option<String>,
    /// Parent folder ID.
    #[serde(default)]
    pub parent_id: Option<i64>,
    /// Minimum role required to view.
    #[serde(default = "default_member_role")]
    #[validate(length(min = 1, message = "Permission is required"))]
    pub permission: String,
    /// Minimum role required to upload.
    #[serde(default = "default_subop_role")]
    #[validate(length(min = 1, message = "Upload permission is required"))]
    pub upload_perm: String,
    /// Sort order.
    #[serde(default)]
    pub order_num: i32,
}

fn default_subop_role() -> String {
    "subop".to_string()
}

/// Update folder request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminUpdateFolderRequest {
    /// Folder name.
    #[serde(default)]
    #[validate(length(min = 1, max = 50, message = "Folder name must be 1-50 characters"))]
    pub name: Option<String>,
    /// Folder description.
    #[serde(default)]
    pub description: Option<Option<String>>,
    /// Parent folder ID.
    #[serde(default)]
    pub parent_id: Option<Option<i64>>,
    /// Minimum role required to view.
    #[serde(default)]
    #[validate(length(min = 1, message = "Permission must not be empty"))]
    pub permission: Option<String>,
    /// Minimum role required to upload.
    #[serde(default)]
    #[validate(length(min = 1, message = "Upload permission must not be empty"))]
    pub upload_perm: Option<String>,
    /// Sort order.
    #[serde(default)]
    pub order_num: Option<i32>,
}

/// Add RSS feed request (admin).
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminAddFeedRequest {
    /// Feed URL.
    #[validate(url(message = "Invalid URL format"))]
    #[validate(length(min = 1, max = 2048, message = "URL must be 1-2048 characters"))]
    pub url: String,
    /// Feed title (optional, will be fetched from feed if not provided).
    #[serde(default)]
    #[validate(length(max = 100, message = "Title must be 100 characters or less"))]
    pub title: Option<String>,
}
