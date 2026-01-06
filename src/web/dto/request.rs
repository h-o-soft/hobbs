//! Request DTOs for Web API.

use serde::Deserialize;
use utoipa::ToSchema;

// ============================================================================
// Auth DTOs
// ============================================================================

/// Login request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Username.
    pub username: String,
    /// Password.
    pub password: String,
}

/// Logout request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutRequest {
    /// Refresh token to invalidate.
    pub refresh_token: String,
}

/// Token refresh request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    /// Refresh token.
    pub refresh_token: String,
}

/// User registration request.
#[derive(Debug, Deserialize, ToSchema)]
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
#[derive(Debug, Deserialize, ToSchema)]
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
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateThreadRequest {
    /// Thread title.
    pub title: String,
    /// First post body.
    pub body: String,
}

/// Create post request (for thread-based boards).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePostRequest {
    /// Post body.
    pub body: String,
}

/// Create flat post request (for flat boards).
#[derive(Debug, Deserialize, ToSchema)]
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
#[derive(Debug, Deserialize, ToSchema)]
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
#[derive(Debug, Deserialize, ToSchema)]
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
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    /// Current password.
    pub current_password: String,
    /// New password.
    pub new_password: String,
}

// ============================================================================
// Admin DTOs
// ============================================================================

/// Update user request (admin).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUpdateUserRequest {
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

/// Update user role request (admin).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUpdateRoleRequest {
    /// New role (guest, member, subop, sysop).
    pub role: String,
}

/// Update user status request (admin).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUpdateStatusRequest {
    /// Whether the user is active.
    pub is_active: bool,
}

/// Reset password request (admin).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminResetPasswordRequest {
    /// New password.
    pub new_password: String,
}

/// Create board request (admin).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminCreateBoardRequest {
    /// Board name.
    pub name: String,
    /// Board description.
    #[serde(default)]
    pub description: Option<String>,
    /// Board type (thread or flat).
    #[serde(default = "default_board_type")]
    pub board_type: String,
    /// Minimum role required to read.
    #[serde(default = "default_guest_role")]
    pub min_read_role: String,
    /// Minimum role required to write.
    #[serde(default = "default_member_role")]
    pub min_write_role: String,
    /// Sort order.
    #[serde(default)]
    pub sort_order: i32,
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
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUpdateBoardRequest {
    /// Board name.
    #[serde(default)]
    pub name: Option<String>,
    /// Board description.
    #[serde(default)]
    pub description: Option<Option<String>>,
    /// Board type (thread or flat).
    #[serde(default)]
    pub board_type: Option<String>,
    /// Minimum role required to read.
    #[serde(default)]
    pub min_read_role: Option<String>,
    /// Minimum role required to write.
    #[serde(default)]
    pub min_write_role: Option<String>,
    /// Sort order.
    #[serde(default)]
    pub sort_order: Option<i32>,
    /// Whether the board is active.
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Create folder request (admin).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminCreateFolderRequest {
    /// Folder name.
    pub name: String,
    /// Folder description.
    #[serde(default)]
    pub description: Option<String>,
    /// Parent folder ID.
    #[serde(default)]
    pub parent_id: Option<i64>,
    /// Minimum role required to view.
    #[serde(default = "default_member_role")]
    pub permission: String,
    /// Minimum role required to upload.
    #[serde(default = "default_subop_role")]
    pub upload_perm: String,
    /// Sort order.
    #[serde(default)]
    pub order_num: i32,
}

fn default_subop_role() -> String {
    "subop".to_string()
}

/// Update folder request (admin).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUpdateFolderRequest {
    /// Folder name.
    #[serde(default)]
    pub name: Option<String>,
    /// Folder description.
    #[serde(default)]
    pub description: Option<Option<String>>,
    /// Parent folder ID.
    #[serde(default)]
    pub parent_id: Option<Option<i64>>,
    /// Minimum role required to view.
    #[serde(default)]
    pub permission: Option<String>,
    /// Minimum role required to upload.
    #[serde(default)]
    pub upload_perm: Option<String>,
    /// Sort order.
    #[serde(default)]
    pub order_num: Option<i32>,
}

/// Add RSS feed request (admin).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminAddFeedRequest {
    /// Feed URL.
    pub url: String,
    /// Feed title (optional, will be fetched from feed if not provided).
    #[serde(default)]
    pub title: Option<String>,
}
