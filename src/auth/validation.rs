//! Input validation for HOBBS user registration.
//!
//! This module provides validation functions for usernames, passwords,
//! nicknames, and email addresses.

use thiserror::Error;

/// Minimum username length.
pub const MIN_USERNAME_LENGTH: usize = 4;

/// Maximum username length.
pub const MAX_USERNAME_LENGTH: usize = 16;

/// Minimum password length.
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Maximum password length.
pub const MAX_PASSWORD_LENGTH: usize = 128;

/// Maximum nickname length.
pub const MAX_NICKNAME_LENGTH: usize = 20;

/// Maximum email length.
pub const MAX_EMAIL_LENGTH: usize = 254;

/// Validation errors.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Username is too short.
    #[error("username must be at least {MIN_USERNAME_LENGTH} characters")]
    UsernameTooShort,

    /// Username is too long.
    #[error("username must be at most {MAX_USERNAME_LENGTH} characters")]
    UsernameTooLong,

    /// Username contains invalid characters.
    #[error("username can only contain alphanumeric characters and underscores")]
    UsernameInvalidChars,

    /// Username is reserved.
    #[error("this username is reserved")]
    UsernameReserved,

    /// Password is too short.
    #[error("password must be at least {MIN_PASSWORD_LENGTH} characters")]
    PasswordTooShort,

    /// Password is too long.
    #[error("password must be at most {MAX_PASSWORD_LENGTH} characters")]
    PasswordTooLong,

    /// Password is the same as username.
    #[error("password cannot be the same as username")]
    PasswordSameAsUsername,

    /// Nickname is empty.
    #[error("nickname cannot be empty")]
    NicknameEmpty,

    /// Nickname is too long.
    #[error("nickname must be at most {MAX_NICKNAME_LENGTH} characters")]
    NicknameTooLong,

    /// Nickname contains invalid characters.
    #[error("nickname contains invalid characters")]
    NicknameInvalidChars,

    /// Email is too long.
    #[error("email must be at most {MAX_EMAIL_LENGTH} characters")]
    EmailTooLong,

    /// Email format is invalid.
    #[error("invalid email format")]
    EmailInvalidFormat,
}

/// Reserved usernames that cannot be registered.
const RESERVED_USERNAMES: &[&str] = &[
    "guest",
    "admin",
    "sysop",
    "subop",
    "root",
    "system",
    "anonymous",
    "administrator",
    "moderator",
    "operator",
    "support",
    "help",
    "info",
    "test",
    "demo",
    "null",
    "undefined",
    "hobbs",
];

/// Check if a username is reserved.
pub fn is_reserved_username(username: &str) -> bool {
    let lower = username.to_lowercase();
    RESERVED_USERNAMES.iter().any(|&r| r == lower)
}

/// Validate a username.
///
/// Requirements:
/// - Length: 4-16 characters
/// - Characters: alphanumeric (a-z, A-Z, 0-9) and underscore (_)
/// - Not a reserved username
///
/// # Examples
///
/// ```
/// use hobbs::auth::validation::validate_username;
///
/// assert!(validate_username("john_doe").is_ok());
/// assert!(validate_username("ab").is_err()); // too short
/// assert!(validate_username("guest").is_err()); // reserved
/// ```
pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    // Check length
    if username.len() < MIN_USERNAME_LENGTH {
        return Err(ValidationError::UsernameTooShort);
    }
    if username.len() > MAX_USERNAME_LENGTH {
        return Err(ValidationError::UsernameTooLong);
    }

    // Check characters: must be alphanumeric or underscore
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(ValidationError::UsernameInvalidChars);
    }

    // Check reserved usernames
    if is_reserved_username(username) {
        return Err(ValidationError::UsernameReserved);
    }

    Ok(())
}

/// Validate a password.
///
/// Requirements:
/// - Length: 8-128 characters
/// - Must not be the same as the username (if provided)
///
/// # Examples
///
/// ```
/// use hobbs::auth::validation::validate_registration_password;
///
/// assert!(validate_registration_password("secure_pass123", Some("john")).is_ok());
/// assert!(validate_registration_password("short", None).is_err()); // too short
/// assert!(validate_registration_password("john", Some("john")).is_err()); // same as username
/// ```
pub fn validate_registration_password(
    password: &str,
    username: Option<&str>,
) -> Result<(), ValidationError> {
    // Check length
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(ValidationError::PasswordTooShort);
    }
    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(ValidationError::PasswordTooLong);
    }

    // Check if same as username
    if let Some(user) = username {
        if password.eq_ignore_ascii_case(user) {
            return Err(ValidationError::PasswordSameAsUsername);
        }
    }

    Ok(())
}

/// Validate a nickname.
///
/// Requirements:
/// - Not empty
/// - Length: at most 20 characters
/// - No control characters
///
/// # Examples
///
/// ```
/// use hobbs::auth::validation::validate_nickname;
///
/// assert!(validate_nickname("John Doe").is_ok());
/// assert!(validate_nickname("").is_err()); // empty
/// ```
pub fn validate_nickname(nickname: &str) -> Result<(), ValidationError> {
    // Check empty
    if nickname.is_empty() {
        return Err(ValidationError::NicknameEmpty);
    }

    // Check length (in characters, not bytes)
    if nickname.chars().count() > MAX_NICKNAME_LENGTH {
        return Err(ValidationError::NicknameTooLong);
    }

    // Check for control characters (except space)
    if nickname.chars().any(|c| c.is_control()) {
        return Err(ValidationError::NicknameInvalidChars);
    }

    Ok(())
}

/// Validate an email address (optional field).
///
/// If empty, returns Ok (email is optional).
/// If provided, performs basic format validation.
///
/// # Examples
///
/// ```
/// use hobbs::auth::validation::validate_email;
///
/// assert!(validate_email("").is_ok()); // optional
/// assert!(validate_email("user@example.com").is_ok());
/// assert!(validate_email("invalid").is_err());
/// ```
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    // Empty is OK (email is optional)
    if email.is_empty() {
        return Ok(());
    }

    // Check length
    if email.len() > MAX_EMAIL_LENGTH {
        return Err(ValidationError::EmailTooLong);
    }

    // Basic format check: must contain @ and have text before and after
    // This is intentionally simple - we don't try to fully validate email format
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(ValidationError::EmailInvalidFormat);
    }

    let (local, domain) = (parts[0], parts[1]);

    // Local part must not be empty
    if local.is_empty() {
        return Err(ValidationError::EmailInvalidFormat);
    }

    // Domain must contain at least one dot and not be empty on either side
    if !domain.contains('.') {
        return Err(ValidationError::EmailInvalidFormat);
    }

    let domain_parts: Vec<&str> = domain.split('.').collect();
    if domain_parts.iter().any(|p| p.is_empty()) {
        return Err(ValidationError::EmailInvalidFormat);
    }

    // No whitespace allowed
    if email.chars().any(|c| c.is_whitespace()) {
        return Err(ValidationError::EmailInvalidFormat);
    }

    Ok(())
}

/// Validate all registration fields at once.
///
/// Returns the first validation error encountered, or Ok if all fields are valid.
pub fn validate_registration(
    username: &str,
    password: &str,
    nickname: &str,
    email: Option<&str>,
) -> Result<(), ValidationError> {
    validate_username(username)?;
    validate_registration_password(password, Some(username))?;
    validate_nickname(nickname)?;
    if let Some(e) = email {
        validate_email(e)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Username validation tests
    #[test]
    fn test_validate_username_valid() {
        assert!(validate_username("john").is_ok());
        assert!(validate_username("john_doe").is_ok());
        assert!(validate_username("JohnDoe123").is_ok());
        assert!(validate_username("user_name_123").is_ok());
        assert!(validate_username("a_b_").is_ok());
    }

    #[test]
    fn test_validate_username_too_short() {
        assert_eq!(
            validate_username("abc"),
            Err(ValidationError::UsernameTooShort)
        );
        assert_eq!(
            validate_username("ab"),
            Err(ValidationError::UsernameTooShort)
        );
        assert_eq!(
            validate_username("a"),
            Err(ValidationError::UsernameTooShort)
        );
        assert_eq!(
            validate_username(""),
            Err(ValidationError::UsernameTooShort)
        );
    }

    #[test]
    fn test_validate_username_too_long() {
        let long_name = "a".repeat(17);
        assert_eq!(
            validate_username(&long_name),
            Err(ValidationError::UsernameTooLong)
        );
    }

    #[test]
    fn test_validate_username_exact_lengths() {
        // Exactly 4 characters - minimum
        assert!(validate_username("abcd").is_ok());
        // Exactly 16 characters - maximum
        assert!(validate_username("abcdefghijklmnop").is_ok());
    }

    #[test]
    fn test_validate_username_invalid_chars() {
        assert_eq!(
            validate_username("john-doe"),
            Err(ValidationError::UsernameInvalidChars)
        );
        assert_eq!(
            validate_username("john.doe"),
            Err(ValidationError::UsernameInvalidChars)
        );
        assert_eq!(
            validate_username("john doe"),
            Err(ValidationError::UsernameInvalidChars)
        );
        assert_eq!(
            validate_username("john@doe"),
            Err(ValidationError::UsernameInvalidChars)
        );
        assert_eq!(
            validate_username("ユーザー"),
            Err(ValidationError::UsernameInvalidChars)
        );
    }

    #[test]
    fn test_validate_username_reserved() {
        assert_eq!(
            validate_username("guest"),
            Err(ValidationError::UsernameReserved)
        );
        assert_eq!(
            validate_username("GUEST"),
            Err(ValidationError::UsernameReserved)
        );
        assert_eq!(
            validate_username("Guest"),
            Err(ValidationError::UsernameReserved)
        );
        assert_eq!(
            validate_username("admin"),
            Err(ValidationError::UsernameReserved)
        );
        assert_eq!(
            validate_username("sysop"),
            Err(ValidationError::UsernameReserved)
        );
        assert_eq!(
            validate_username("root"),
            Err(ValidationError::UsernameReserved)
        );
    }

    #[test]
    fn test_is_reserved_username() {
        assert!(is_reserved_username("guest"));
        assert!(is_reserved_username("ADMIN"));
        assert!(is_reserved_username("SysOp"));
        assert!(!is_reserved_username("john"));
        assert!(!is_reserved_username("guestuser")); // contains but not exact
    }

    // Password validation tests
    #[test]
    fn test_validate_password_valid() {
        assert!(validate_registration_password("password123", None).is_ok());
        assert!(validate_registration_password("12345678", None).is_ok());
        assert!(validate_registration_password("a".repeat(128).as_str(), None).is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        assert_eq!(
            validate_registration_password("short", None),
            Err(ValidationError::PasswordTooShort)
        );
        assert_eq!(
            validate_registration_password("1234567", None),
            Err(ValidationError::PasswordTooShort)
        );
    }

    #[test]
    fn test_validate_password_too_long() {
        let long_pass = "a".repeat(129);
        assert_eq!(
            validate_registration_password(&long_pass, None),
            Err(ValidationError::PasswordTooLong)
        );
    }

    #[test]
    fn test_validate_password_same_as_username() {
        // Use 8+ character username to avoid triggering PasswordTooShort
        assert_eq!(
            validate_registration_password("john_doe", Some("john_doe")),
            Err(ValidationError::PasswordSameAsUsername)
        );
        // Case insensitive
        assert_eq!(
            validate_registration_password("John_Doe", Some("john_doe")),
            Err(ValidationError::PasswordSameAsUsername)
        );
    }

    // Nickname validation tests
    #[test]
    fn test_validate_nickname_valid() {
        assert!(validate_nickname("John").is_ok());
        assert!(validate_nickname("John Doe").is_ok());
        assert!(validate_nickname("太郎").is_ok());
        assert!(validate_nickname("User 123").is_ok());
    }

    #[test]
    fn test_validate_nickname_empty() {
        assert_eq!(validate_nickname(""), Err(ValidationError::NicknameEmpty));
    }

    #[test]
    fn test_validate_nickname_too_long() {
        // 21 ASCII characters
        assert_eq!(
            validate_nickname("a".repeat(21).as_str()),
            Err(ValidationError::NicknameTooLong)
        );
        // 21 Japanese characters (multi-byte but still 21 chars)
        assert_eq!(
            validate_nickname("あ".repeat(21).as_str()),
            Err(ValidationError::NicknameTooLong)
        );
    }

    #[test]
    fn test_validate_nickname_exact_length() {
        // Exactly 20 characters - maximum
        assert!(validate_nickname("a".repeat(20).as_str()).is_ok());
        assert!(validate_nickname("あ".repeat(20).as_str()).is_ok());
    }

    #[test]
    fn test_validate_nickname_control_chars() {
        assert_eq!(
            validate_nickname("John\x00Doe"),
            Err(ValidationError::NicknameInvalidChars)
        );
        assert_eq!(
            validate_nickname("John\nDoe"),
            Err(ValidationError::NicknameInvalidChars)
        );
    }

    // Email validation tests
    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("").is_ok()); // optional
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name@example.co.jp").is_ok());
        assert!(validate_email("user+tag@example.com").is_ok());
    }

    #[test]
    fn test_validate_email_invalid_format() {
        assert_eq!(
            validate_email("invalid"),
            Err(ValidationError::EmailInvalidFormat)
        );
        assert_eq!(
            validate_email("@example.com"),
            Err(ValidationError::EmailInvalidFormat)
        );
        assert_eq!(
            validate_email("user@"),
            Err(ValidationError::EmailInvalidFormat)
        );
        assert_eq!(
            validate_email("user@example"),
            Err(ValidationError::EmailInvalidFormat)
        );
        assert_eq!(
            validate_email("user@@example.com"),
            Err(ValidationError::EmailInvalidFormat)
        );
        assert_eq!(
            validate_email("user @example.com"),
            Err(ValidationError::EmailInvalidFormat)
        );
    }

    #[test]
    fn test_validate_email_too_long() {
        let long_email = format!("{}@example.com", "a".repeat(250));
        assert_eq!(
            validate_email(&long_email),
            Err(ValidationError::EmailTooLong)
        );
    }

    // Combined validation tests
    #[test]
    fn test_validate_registration_all_valid() {
        assert!(validate_registration("john_doe", "password123", "John Doe", None).is_ok());
        assert!(validate_registration(
            "john_doe",
            "password123",
            "John Doe",
            Some("john@example.com")
        )
        .is_ok());
    }

    #[test]
    fn test_validate_registration_fails_on_first_error() {
        // Should fail on username
        assert_eq!(
            validate_registration("ab", "password123", "John", None),
            Err(ValidationError::UsernameTooShort)
        );
    }

    #[test]
    fn test_validation_error_display() {
        assert!(ValidationError::UsernameTooShort
            .to_string()
            .contains("at least"));
        assert!(ValidationError::UsernameReserved
            .to_string()
            .contains("reserved"));
        assert!(ValidationError::PasswordTooShort
            .to_string()
            .contains("at least"));
    }
}
