//! User profile management for HOBBS.
//!
//! This module provides functions for viewing and updating user profiles,
//! as well as changing passwords.

use thiserror::Error;
use tracing::info;

use crate::auth::validation::{validate_email, validate_nickname, ValidationError};
use crate::auth::{hash_password, verify_password, PasswordError};
use crate::db::{Role, User, UserRepository, UserUpdate};

/// Maximum length for profile text.
pub const MAX_PROFILE_LENGTH: usize = 1000;

/// Profile-related errors.
#[derive(Error, Debug)]
pub enum ProfileError {
    /// User not found.
    #[error("ユーザーが見つかりません")]
    UserNotFound,

    /// Validation failed.
    #[error("入力エラー: {0}")]
    Validation(#[from] ValidationError),

    /// Password error.
    #[error("パスワードエラー: {0}")]
    Password(#[from] PasswordError),

    /// Current password is incorrect.
    #[error("現在のパスワードが正しくありません")]
    WrongPassword,

    /// Profile text is too long.
    #[error("プロフィールは{MAX_PROFILE_LENGTH}文字以内で入力してください")]
    ProfileTooLong,

    /// Database error.
    #[error("データベースエラー: {0}")]
    Database(String),
}

/// User profile for public display.
///
/// This struct contains only the information that should be visible
/// to other users (no password hash, etc.).
#[derive(Debug, Clone)]
pub struct UserProfile {
    /// User ID.
    pub id: i64,
    /// Username (login name).
    pub username: String,
    /// Display nickname.
    pub nickname: String,
    /// User role.
    pub role: Role,
    /// Self-introduction text.
    pub profile: Option<String>,
    /// Terminal preference.
    pub terminal: String,
    /// Account creation date.
    pub created_at: String,
    /// Last login date.
    pub last_login: Option<String>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            nickname: user.nickname,
            role: user.role,
            profile: user.profile,
            terminal: user.terminal,
            created_at: user.created_at,
            last_login: user.last_login,
        }
    }
}

impl From<&User> for UserProfile {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            nickname: user.nickname.clone(),
            role: user.role,
            profile: user.profile.clone(),
            terminal: user.terminal.clone(),
            created_at: user.created_at.clone(),
            last_login: user.last_login.clone(),
        }
    }
}

/// Profile update request.
#[derive(Debug, Clone, Default)]
pub struct ProfileUpdateRequest {
    /// New nickname.
    pub nickname: Option<String>,
    /// New email address.
    pub email: Option<Option<String>>,
    /// New profile text.
    pub profile: Option<Option<String>>,
    /// New terminal preference.
    pub terminal: Option<String>,
}

impl ProfileUpdateRequest {
    /// Create a new empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set new nickname.
    pub fn nickname(mut self, nickname: impl Into<String>) -> Self {
        self.nickname = Some(nickname.into());
        self
    }

    /// Set new email (Some for update, None to clear).
    pub fn email(mut self, email: Option<String>) -> Self {
        self.email = Some(email);
        self
    }

    /// Set new profile text (Some for update, None to clear).
    pub fn profile(mut self, profile: Option<String>) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Set new terminal preference.
    pub fn terminal(mut self, terminal: impl Into<String>) -> Self {
        self.terminal = Some(terminal.into());
        self
    }

    /// Check if the request is empty.
    pub fn is_empty(&self) -> bool {
        self.nickname.is_none()
            && self.email.is_none()
            && self.profile.is_none()
            && self.terminal.is_none()
    }
}

/// Get a user's public profile by ID.
///
/// # Arguments
///
/// * `repo` - User repository
/// * `user_id` - The user ID to look up
///
/// # Returns
///
/// The user's public profile, or an error if not found.
pub fn get_profile(repo: &UserRepository, user_id: i64) -> Result<UserProfile, ProfileError> {
    let user = repo
        .get_by_id(user_id)
        .map_err(|e| ProfileError::Database(e.to_string()))?
        .ok_or(ProfileError::UserNotFound)?;

    Ok(UserProfile::from(user))
}

/// Get a user's public profile by username.
///
/// # Arguments
///
/// * `repo` - User repository
/// * `username` - The username to look up
///
/// # Returns
///
/// The user's public profile, or an error if not found.
pub fn get_profile_by_username(
    repo: &UserRepository,
    username: &str,
) -> Result<UserProfile, ProfileError> {
    let user = repo
        .get_by_username(username)
        .map_err(|e| ProfileError::Database(e.to_string()))?
        .ok_or(ProfileError::UserNotFound)?;

    Ok(UserProfile::from(user))
}

/// Validate profile text.
fn validate_profile_text(text: &str) -> Result<(), ProfileError> {
    if text.chars().count() > MAX_PROFILE_LENGTH {
        return Err(ProfileError::ProfileTooLong);
    }
    // Check for control characters (except newlines)
    if text
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r')
    {
        return Err(ProfileError::Validation(
            ValidationError::NicknameInvalidChars,
        ));
    }
    Ok(())
}

/// Update a user's profile.
///
/// This function validates all input and updates the specified fields.
/// Only the user themselves or an operator can update a profile.
///
/// # Arguments
///
/// * `repo` - User repository
/// * `user_id` - The user ID to update
/// * `request` - The profile update request
///
/// # Returns
///
/// The updated user profile, or an error.
pub fn update_profile(
    repo: &UserRepository,
    user_id: i64,
    request: ProfileUpdateRequest,
) -> Result<UserProfile, ProfileError> {
    // Check if user exists
    let _user = repo
        .get_by_id(user_id)
        .map_err(|e| ProfileError::Database(e.to_string()))?
        .ok_or(ProfileError::UserNotFound)?;

    // If empty request, just return current profile
    if request.is_empty() {
        return get_profile(repo, user_id);
    }

    // Validate fields
    if let Some(ref nickname) = request.nickname {
        validate_nickname(nickname)?;
    }

    if let Some(Some(ref email)) = request.email {
        validate_email(email)?;
    }

    if let Some(Some(ref profile)) = request.profile {
        validate_profile_text(profile)?;
    }

    // Build update
    let mut update = UserUpdate::new();

    if let Some(nickname) = request.nickname {
        update = update.nickname(nickname);
    }

    if let Some(email) = request.email {
        update = update.email(email);
    }

    if let Some(profile) = request.profile {
        update = update.profile(profile);
    }

    if let Some(terminal) = request.terminal {
        update = update.terminal(terminal);
    }

    // Apply update
    let updated = repo
        .update(user_id, &update)
        .map_err(|e| ProfileError::Database(e.to_string()))?
        .ok_or(ProfileError::UserNotFound)?;

    info!(
        user_id = user_id,
        username = %updated.username,
        "Profile updated"
    );

    Ok(UserProfile::from(updated))
}

/// Change a user's password.
///
/// This function verifies the current password before updating to the new one.
///
/// # Arguments
///
/// * `repo` - User repository
/// * `user_id` - The user ID to update
/// * `current_password` - The current password for verification
/// * `new_password` - The new password
///
/// # Returns
///
/// `Ok(())` on success, or an error.
pub fn change_password(
    repo: &UserRepository,
    user_id: i64,
    current_password: &str,
    new_password: &str,
) -> Result<(), ProfileError> {
    // Get user
    let user = repo
        .get_by_id(user_id)
        .map_err(|e| ProfileError::Database(e.to_string()))?
        .ok_or(ProfileError::UserNotFound)?;

    // Verify current password
    verify_password(current_password, &user.password).map_err(|e| match e {
        PasswordError::VerificationFailed => ProfileError::WrongPassword,
        other => ProfileError::Password(other),
    })?;

    // Hash new password (this also validates length)
    let new_hash = hash_password(new_password)?;

    // Update password
    let update = UserUpdate::new().password(new_hash);
    repo.update(user_id, &update)
        .map_err(|e| ProfileError::Database(e.to_string()))?
        .ok_or(ProfileError::UserNotFound)?;

    info!(
        user_id = user_id,
        username = %user.username,
        "Password changed"
    );

    Ok(())
}

/// Reset a user's password (admin operation).
///
/// This function does not require the current password.
/// Only SysOp should be able to call this.
///
/// # Arguments
///
/// * `repo` - User repository
/// * `user_id` - The user ID to update
/// * `new_password` - The new password
///
/// # Returns
///
/// `Ok(())` on success, or an error.
pub fn reset_password(
    repo: &UserRepository,
    user_id: i64,
    new_password: &str,
) -> Result<(), ProfileError> {
    // Get user to verify existence
    let user = repo
        .get_by_id(user_id)
        .map_err(|e| ProfileError::Database(e.to_string()))?
        .ok_or(ProfileError::UserNotFound)?;

    // Hash new password (this also validates length)
    let new_hash = hash_password(new_password)?;

    // Update password
    let update = UserUpdate::new().password(new_hash);
    repo.update(user_id, &update)
        .map_err(|e| ProfileError::Database(e.to_string()))?
        .ok_or(ProfileError::UserNotFound)?;

    info!(
        user_id = user_id,
        username = %user.username,
        "Password reset by admin"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_user(repo: &UserRepository) -> User {
        use crate::auth::register;
        use crate::RegistrationRequest;

        let request = RegistrationRequest::new("testuser", "password123", "Test User")
            .with_email("test@example.com");
        register(repo, request).unwrap()
    }

    #[test]
    fn test_get_profile() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let profile = get_profile(&repo, user.id).unwrap();

        assert_eq!(profile.id, user.id);
        assert_eq!(profile.username, "testuser");
        assert_eq!(profile.nickname, "Test User");
        assert_eq!(profile.role, Role::Member);
    }

    #[test]
    fn test_get_profile_not_found() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let result = get_profile(&repo, 999);
        assert!(matches!(result, Err(ProfileError::UserNotFound)));
    }

    #[test]
    fn test_get_profile_by_username() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let _user = setup_user(&repo);

        let profile = get_profile_by_username(&repo, "testuser").unwrap();
        assert_eq!(profile.username, "testuser");
    }

    #[test]
    fn test_get_profile_by_username_not_found() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let result = get_profile_by_username(&repo, "nonexistent");
        assert!(matches!(result, Err(ProfileError::UserNotFound)));
    }

    #[test]
    fn test_update_profile_nickname() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request = ProfileUpdateRequest::new().nickname("New Nickname");
        let updated = update_profile(&repo, user.id, request).unwrap();

        assert_eq!(updated.nickname, "New Nickname");
    }

    #[test]
    fn test_update_profile_email() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request = ProfileUpdateRequest::new().email(Some("new@example.com".to_string()));
        let _updated = update_profile(&repo, user.id, request).unwrap();

        // Verify via database
        let updated_user = repo.get_by_id(user.id).unwrap().unwrap();
        assert_eq!(updated_user.email, Some("new@example.com".to_string()));
    }

    #[test]
    fn test_update_profile_clear_email() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request = ProfileUpdateRequest::new().email(None);
        let _updated = update_profile(&repo, user.id, request).unwrap();

        // Verify via database
        let updated_user = repo.get_by_id(user.id).unwrap().unwrap();
        assert_eq!(updated_user.email, None);
    }

    #[test]
    fn test_update_profile_text() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request =
            ProfileUpdateRequest::new().profile(Some("Hello, I'm a BBS user!".to_string()));
        let updated = update_profile(&repo, user.id, request).unwrap();

        assert_eq!(updated.profile, Some("Hello, I'm a BBS user!".to_string()));
    }

    #[test]
    fn test_update_profile_terminal() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request = ProfileUpdateRequest::new().terminal("c64");
        let updated = update_profile(&repo, user.id, request).unwrap();

        assert_eq!(updated.terminal, "c64");
    }

    #[test]
    fn test_update_profile_multiple_fields() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request = ProfileUpdateRequest::new()
            .nickname("Updated Name")
            .profile(Some("My profile text".to_string()))
            .terminal("c64_ansi");

        let updated = update_profile(&repo, user.id, request).unwrap();

        assert_eq!(updated.nickname, "Updated Name");
        assert_eq!(updated.profile, Some("My profile text".to_string()));
        assert_eq!(updated.terminal, "c64_ansi");
    }

    #[test]
    fn test_update_profile_empty_request() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request = ProfileUpdateRequest::new();
        let updated = update_profile(&repo, user.id, request).unwrap();

        // Should return current profile unchanged
        assert_eq!(updated.nickname, "Test User");
    }

    #[test]
    fn test_update_profile_invalid_nickname() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request = ProfileUpdateRequest::new().nickname("");
        let result = update_profile(&repo, user.id, request);

        assert!(matches!(result, Err(ProfileError::Validation(_))));
    }

    #[test]
    fn test_update_profile_invalid_email() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let request = ProfileUpdateRequest::new().email(Some("invalid-email".to_string()));
        let result = update_profile(&repo, user.id, request);

        assert!(matches!(result, Err(ProfileError::Validation(_))));
    }

    #[test]
    fn test_update_profile_too_long() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let long_text = "a".repeat(MAX_PROFILE_LENGTH + 1);
        let request = ProfileUpdateRequest::new().profile(Some(long_text));
        let result = update_profile(&repo, user.id, request);

        assert!(matches!(result, Err(ProfileError::ProfileTooLong)));
    }

    #[test]
    fn test_update_profile_max_length() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let max_text = "a".repeat(MAX_PROFILE_LENGTH);
        let request = ProfileUpdateRequest::new().profile(Some(max_text.clone()));
        let updated = update_profile(&repo, user.id, request).unwrap();

        assert_eq!(updated.profile, Some(max_text));
    }

    #[test]
    fn test_update_profile_not_found() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let request = ProfileUpdateRequest::new().nickname("New Name");
        let result = update_profile(&repo, 999, request);

        assert!(matches!(result, Err(ProfileError::UserNotFound)));
    }

    #[test]
    fn test_change_password_success() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let result = change_password(&repo, user.id, "password123", "newpassword456");
        assert!(result.is_ok());

        // Verify new password works
        let updated_user = repo.get_by_id(user.id).unwrap().unwrap();
        assert!(verify_password("newpassword456", &updated_user.password).is_ok());
    }

    #[test]
    fn test_change_password_wrong_current() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let result = change_password(&repo, user.id, "wrongpassword", "newpassword456");
        assert!(matches!(result, Err(ProfileError::WrongPassword)));
    }

    #[test]
    fn test_change_password_invalid_new() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        // Too short
        let result = change_password(&repo, user.id, "password123", "short");
        assert!(matches!(result, Err(ProfileError::Password(_))));
    }

    #[test]
    fn test_change_password_not_found() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let result = change_password(&repo, 999, "oldpass", "newpass123");
        assert!(matches!(result, Err(ProfileError::UserNotFound)));
    }

    #[test]
    fn test_reset_password_success() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let result = reset_password(&repo, user.id, "resetpassword123");
        assert!(result.is_ok());

        // Verify new password works
        let updated_user = repo.get_by_id(user.id).unwrap().unwrap();
        assert!(verify_password("resetpassword123", &updated_user.password).is_ok());
    }

    #[test]
    fn test_reset_password_invalid() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        // Too short
        let result = reset_password(&repo, user.id, "short");
        assert!(matches!(result, Err(ProfileError::Password(_))));
    }

    #[test]
    fn test_reset_password_not_found() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let result = reset_password(&repo, 999, "newpass123");
        assert!(matches!(result, Err(ProfileError::UserNotFound)));
    }

    #[test]
    fn test_user_profile_from_user() {
        let user = User {
            id: 1,
            username: "testuser".to_string(),
            password: "hash".to_string(),
            nickname: "Test".to_string(),
            email: Some("test@example.com".to_string()),
            role: Role::Member,
            profile: Some("Hello!".to_string()),
            terminal: "standard".to_string(),
            created_at: "2024-01-01".to_string(),
            last_login: Some("2024-01-02".to_string()),
            is_active: true,
        };

        let profile = UserProfile::from(&user);

        assert_eq!(profile.id, 1);
        assert_eq!(profile.username, "testuser");
        assert_eq!(profile.nickname, "Test");
        assert_eq!(profile.profile, Some("Hello!".to_string()));
        // Password should not be in profile (it's not a field)
    }

    #[test]
    fn test_profile_update_request_builder() {
        let request = ProfileUpdateRequest::new()
            .nickname("Nick")
            .email(Some("a@b.com".to_string()))
            .profile(Some("Profile".to_string()))
            .terminal("c64");

        assert_eq!(request.nickname, Some("Nick".to_string()));
        assert_eq!(request.email, Some(Some("a@b.com".to_string())));
        assert_eq!(request.profile, Some(Some("Profile".to_string())));
        assert_eq!(request.terminal, Some("c64".to_string()));
        assert!(!request.is_empty());
    }

    #[test]
    fn test_profile_update_request_empty() {
        let request = ProfileUpdateRequest::new();
        assert!(request.is_empty());
    }

    #[test]
    fn test_profile_error_display() {
        assert!(ProfileError::UserNotFound
            .to_string()
            .contains("見つかりません"));
        assert!(ProfileError::WrongPassword
            .to_string()
            .contains("正しくありません"));
        assert!(ProfileError::ProfileTooLong
            .to_string()
            .contains("文字以内"));
    }

    #[test]
    fn test_profile_with_newlines() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);
        let user = setup_user(&repo);

        let profile_text = "Line 1\nLine 2\nLine 3".to_string();
        let request = ProfileUpdateRequest::new().profile(Some(profile_text.clone()));
        let updated = update_profile(&repo, user.id, request).unwrap();

        assert_eq!(updated.profile, Some(profile_text));
    }
}
