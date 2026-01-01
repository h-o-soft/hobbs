//! User registration for HOBBS.
//!
//! This module provides the user registration functionality.

use thiserror::Error;
use tracing::info;

use crate::auth::validation::{validate_registration, ValidationError};
use crate::auth::{hash_password, PasswordError};
use crate::db::{NewUser, Role, User, UserRepository};
use crate::server::CharacterEncoding;

/// Registration-specific errors.
#[derive(Error, Debug)]
pub enum RegistrationError {
    /// Validation failed.
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Username already exists.
    #[error("username already exists")]
    UsernameExists,

    /// Password hashing failed.
    #[error("password error: {0}")]
    Password(#[from] PasswordError),

    /// Database error.
    #[error("database error: {0}")]
    Database(String),
}

/// Registration request data.
#[derive(Debug, Clone)]
pub struct RegistrationRequest {
    /// Desired username (4-16 alphanumeric + underscore).
    pub username: String,
    /// Password (8-128 characters).
    pub password: String,
    /// Display nickname (1-20 characters).
    pub nickname: String,
    /// Optional email address.
    pub email: Option<String>,
    /// Optional terminal profile.
    pub terminal: Option<String>,
    /// Character encoding preference.
    pub encoding: Option<CharacterEncoding>,
    /// Language preference.
    pub language: Option<String>,
}

impl RegistrationRequest {
    /// Create a new registration request.
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        nickname: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            nickname: nickname.into(),
            email: None,
            terminal: None,
            encoding: None,
            language: None,
        }
    }

    /// Set the email address.
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the terminal profile.
    pub fn with_terminal(mut self, terminal: impl Into<String>) -> Self {
        self.terminal = Some(terminal.into());
        self
    }

    /// Set the character encoding.
    pub fn with_encoding(mut self, encoding: CharacterEncoding) -> Self {
        self.encoding = Some(encoding);
        self
    }

    /// Set the language preference.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

/// Register a new user.
///
/// This function:
/// 1. Validates all input fields
/// 2. Checks if the username already exists
/// 3. Hashes the password
/// 4. Creates the user in the database
///
/// # Arguments
///
/// * `repo` - The user repository
/// * `request` - Registration request data
///
/// # Returns
///
/// The newly created user on success, or a `RegistrationError` on failure.
///
/// # Examples
///
/// ```ignore
/// use hobbs::auth::registration::{register, RegistrationRequest};
/// use hobbs::db::{Database, UserRepository};
///
/// let db = Database::open_in_memory()?;
/// let repo = UserRepository::new(db.conn());
///
/// let request = RegistrationRequest::new("john_doe", "password123", "John Doe")
///     .with_email("john@example.com");
///
/// let user = register(&repo, request)?;
/// println!("Registered user: {}", user.username);
/// ```
pub fn register(
    repo: &UserRepository,
    request: RegistrationRequest,
) -> std::result::Result<User, RegistrationError> {
    // 1. Validate all fields
    validate_registration(
        &request.username,
        &request.password,
        &request.nickname,
        request.email.as_deref(),
    )?;

    // 2. Check if username already exists
    if repo
        .username_exists(&request.username)
        .map_err(|e| RegistrationError::Database(e.to_string()))?
    {
        return Err(RegistrationError::UsernameExists);
    }

    // 3. Hash the password
    let password_hash = hash_password(&request.password)?;

    // 4. Create the user
    let mut new_user = NewUser::new(&request.username, &password_hash, &request.nickname);

    if let Some(ref email) = request.email {
        new_user = new_user.with_email(email);
    }

    if let Some(ref terminal) = request.terminal {
        new_user = new_user.with_terminal(terminal);
    }

    if let Some(encoding) = request.encoding {
        new_user = new_user.with_encoding(encoding);
    }

    if let Some(ref language) = request.language {
        new_user = new_user.with_language(language);
    }

    let user = repo
        .create(&new_user)
        .map_err(|e| RegistrationError::Database(e.to_string()))?;

    info!(
        username = %user.username,
        user_id = user.id,
        "New user registered"
    );

    Ok(user)
}

/// Register a new user with a specific role.
///
/// This is typically used for creating the initial SysOp account.
///
/// # Arguments
///
/// * `repo` - The user repository
/// * `request` - Registration request data
/// * `role` - The role to assign to the new user
pub fn register_with_role(
    repo: &UserRepository,
    request: RegistrationRequest,
    role: Role,
) -> std::result::Result<User, RegistrationError> {
    // 1. Validate all fields
    validate_registration(
        &request.username,
        &request.password,
        &request.nickname,
        request.email.as_deref(),
    )?;

    // 2. Check if username already exists
    if repo
        .username_exists(&request.username)
        .map_err(|e| RegistrationError::Database(e.to_string()))?
    {
        return Err(RegistrationError::UsernameExists);
    }

    // 3. Hash the password
    let password_hash = hash_password(&request.password)?;

    // 4. Create the user with specified role
    let mut new_user =
        NewUser::new(&request.username, &password_hash, &request.nickname).with_role(role);

    if let Some(ref email) = request.email {
        new_user = new_user.with_email(email);
    }

    if let Some(ref terminal) = request.terminal {
        new_user = new_user.with_terminal(terminal);
    }

    if let Some(encoding) = request.encoding {
        new_user = new_user.with_encoding(encoding);
    }

    if let Some(ref language) = request.language {
        new_user = new_user.with_language(language);
    }

    let user = repo
        .create(&new_user)
        .map_err(|e| RegistrationError::Database(e.to_string()))?;

    info!(
        username = %user.username,
        user_id = user.id,
        role = %role,
        "New user registered with role"
    );

    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_register_success() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let request = RegistrationRequest::new("testuser", "password123", "Test User");
        let result = register(&repo, request);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.nickname, "Test User");
        assert_eq!(user.role, Role::Member);
        assert!(user.is_active);
    }

    #[test]
    fn test_register_with_email() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let request = RegistrationRequest::new("testuser", "password123", "Test User")
            .with_email("test@example.com");
        let result = register(&repo, request);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_register_with_terminal() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let request =
            RegistrationRequest::new("testuser", "password123", "Test User").with_terminal("c64");
        let result = register(&repo, request);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.terminal, "c64");
    }

    #[test]
    fn test_register_duplicate_username() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        // Register first user
        let request1 = RegistrationRequest::new("testuser", "password123", "Test User 1");
        register(&repo, request1).unwrap();

        // Try to register with same username
        let request2 = RegistrationRequest::new("testuser", "password456", "Test User 2");
        let result = register(&repo, request2);

        assert!(matches!(result, Err(RegistrationError::UsernameExists)));
    }

    #[test]
    fn test_register_invalid_username() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        // Too short
        let request = RegistrationRequest::new("abc", "password123", "Test");
        let result = register(&repo, request);
        assert!(matches!(result, Err(RegistrationError::Validation(_))));

        // Reserved
        let request = RegistrationRequest::new("admin", "password123", "Admin");
        let result = register(&repo, request);
        assert!(matches!(result, Err(RegistrationError::Validation(_))));
    }

    #[test]
    fn test_register_invalid_password() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        // Too short
        let request = RegistrationRequest::new("testuser", "short", "Test User");
        let result = register(&repo, request);
        assert!(matches!(result, Err(RegistrationError::Validation(_))));

        // Same as username
        let request = RegistrationRequest::new("testuser", "testuser", "Test User");
        let result = register(&repo, request);
        assert!(matches!(result, Err(RegistrationError::Validation(_))));
    }

    #[test]
    fn test_register_invalid_nickname() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        // Empty
        let request = RegistrationRequest::new("testuser", "password123", "");
        let result = register(&repo, request);
        assert!(matches!(result, Err(RegistrationError::Validation(_))));
    }

    #[test]
    fn test_register_invalid_email() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let request = RegistrationRequest::new("testuser", "password123", "Test User")
            .with_email("invalid-email");
        let result = register(&repo, request);
        assert!(matches!(result, Err(RegistrationError::Validation(_))));
    }

    #[test]
    fn test_register_with_role() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let request = RegistrationRequest::new("sysop_user", "password123", "System Operator");
        let result = register_with_role(&repo, request, Role::SysOp);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.role, Role::SysOp);
    }

    #[test]
    fn test_registration_request_builder() {
        let request = RegistrationRequest::new("user", "pass", "nick")
            .with_email("a@b.com")
            .with_terminal("c64");

        assert_eq!(request.username, "user");
        assert_eq!(request.password, "pass");
        assert_eq!(request.nickname, "nick");
        assert_eq!(request.email, Some("a@b.com".to_string()));
        assert_eq!(request.terminal, Some("c64".to_string()));
    }

    #[test]
    fn test_registration_error_display() {
        let err = RegistrationError::UsernameExists;
        assert!(err.to_string().contains("already exists"));

        let err = RegistrationError::Validation(ValidationError::UsernameTooShort);
        assert!(err.to_string().contains("validation"));
    }

    #[test]
    fn test_password_is_hashed() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        let request = RegistrationRequest::new("testuser", "password123", "Test User");
        let user = register(&repo, request).unwrap();

        // Password should be hashed, not plain text
        assert_ne!(user.password, "password123");
        assert!(user.password.starts_with("$argon2id$"));
    }
}
