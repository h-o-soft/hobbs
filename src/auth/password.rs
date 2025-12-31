//! Password hashing and validation for HOBBS.
//!
//! Uses Argon2id for secure password hashing.

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};
use rand_core::OsRng;
use thiserror::Error;

/// Minimum password length.
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Maximum password length.
pub const MAX_PASSWORD_LENGTH: usize = 128;

/// Password-related errors.
#[derive(Error, Debug)]
pub enum PasswordError {
    /// Password is too short.
    #[error("password must be at least {MIN_PASSWORD_LENGTH} characters")]
    TooShort,

    /// Password is too long.
    #[error("password must be at most {MAX_PASSWORD_LENGTH} characters")]
    TooLong,

    /// Password hashing failed.
    #[error("password hashing failed: {0}")]
    HashError(String),

    /// Password hash is invalid.
    #[error("invalid password hash format")]
    InvalidHash,

    /// Password verification failed (wrong password).
    #[error("password verification failed")]
    VerificationFailed,
}

/// Create the Argon2 hasher with recommended parameters.
///
/// Parameters:
/// - Memory cost: 64 MB (65536 KiB)
/// - Time cost: 3 iterations
/// - Parallelism: 4 threads
fn create_argon2() -> Argon2<'static> {
    // Memory cost in KiB (64 MB = 65536 KiB)
    let m_cost = 65536;
    // Time cost (iterations)
    let t_cost = 3;
    // Parallelism (threads)
    let p_cost = 4;

    let params = Params::new(m_cost, t_cost, p_cost, None).expect("valid Argon2 params");
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
}

/// Hash a password using Argon2id.
///
/// Returns a PHC-formatted hash string that includes the salt and parameters.
///
/// # Examples
///
/// ```
/// use hobbs::hash_password;
///
/// let hash = hash_password("my_secure_password").unwrap();
/// assert!(hash.starts_with("$argon2id$"));
/// ```
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    // Validate password length first
    validate_password(password)?;

    // Generate a random salt
    let salt = SaltString::generate(&mut OsRng);

    // Hash the password
    let argon2 = create_argon2();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| PasswordError::HashError(e.to_string()))?;

    Ok(hash.to_string())
}

/// Verify a password against a stored hash.
///
/// Returns `Ok(())` if the password matches, or an error if it doesn't.
///
/// # Examples
///
/// ```
/// use hobbs::{hash_password, verify_password};
///
/// let hash = hash_password("my_secure_password").unwrap();
/// assert!(verify_password("my_secure_password", &hash).is_ok());
/// assert!(verify_password("wrong_password", &hash).is_err());
/// ```
pub fn verify_password(password: &str, hash: &str) -> Result<(), PasswordError> {
    // Parse the stored hash
    let parsed_hash = PasswordHash::new(hash).map_err(|_| PasswordError::InvalidHash)?;

    // Verify the password
    // Note: The parameters are taken from the parsed hash, not from create_argon2()
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| PasswordError::VerificationFailed)
}

/// Validate password requirements.
///
/// Checks:
/// - Minimum length: 8 characters
/// - Maximum length: 128 characters
///
/// # Examples
///
/// ```
/// use hobbs::validate_password;
///
/// assert!(validate_password("short").is_err());
/// assert!(validate_password("valid_password_123").is_ok());
/// ```
pub fn validate_password(password: &str) -> Result<(), PasswordError> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(PasswordError::TooShort);
    }
    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(PasswordError::TooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_success() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        // Should be a valid PHC string
        assert!(hash.starts_with("$argon2id$"));
        assert!(hash.contains("$v=19$")); // Version 0x13 = 19
    }

    #[test]
    fn test_hash_password_different_hashes() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Same password should produce different hashes (different salts)
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "correct_password";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).is_ok());
    }

    #[test]
    fn test_verify_password_wrong() {
        let password = "correct_password";
        let hash = hash_password(password).unwrap();

        let result = verify_password("wrong_password", &hash);
        assert!(result.is_err());
        assert!(matches!(result, Err(PasswordError::VerificationFailed)));
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let result = verify_password("any_password", "not_a_valid_hash");
        assert!(result.is_err());
        assert!(matches!(result, Err(PasswordError::InvalidHash)));
    }

    #[test]
    fn test_validate_password_too_short() {
        let result = validate_password("short");
        assert!(result.is_err());
        assert!(matches!(result, Err(PasswordError::TooShort)));
    }

    #[test]
    fn test_validate_password_minimum_length() {
        // Exactly 8 characters
        let result = validate_password("12345678");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_password_too_long() {
        let long_password = "a".repeat(129);
        let result = validate_password(&long_password);
        assert!(result.is_err());
        assert!(matches!(result, Err(PasswordError::TooLong)));
    }

    #[test]
    fn test_validate_password_maximum_length() {
        // Exactly 128 characters
        let max_password = "a".repeat(128);
        let result = validate_password(&max_password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_password_too_short() {
        let result = hash_password("short");
        assert!(result.is_err());
        assert!(matches!(result, Err(PasswordError::TooShort)));
    }

    #[test]
    fn test_hash_password_too_long() {
        let long_password = "a".repeat(129);
        let result = hash_password(&long_password);
        assert!(result.is_err());
        assert!(matches!(result, Err(PasswordError::TooLong)));
    }

    #[test]
    fn test_password_with_unicode() {
        // Japanese characters in password
        let password = "パスワード123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).is_ok());
    }

    #[test]
    fn test_password_with_special_chars() {
        let password = "p@$$w0rd!#$%^&*()";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).is_ok());
    }

    #[test]
    fn test_password_error_display() {
        assert_eq!(
            PasswordError::TooShort.to_string(),
            "password must be at least 8 characters"
        );
        assert_eq!(
            PasswordError::TooLong.to_string(),
            "password must be at most 128 characters"
        );
        assert_eq!(
            PasswordError::VerificationFailed.to_string(),
            "password verification failed"
        );
    }

    #[test]
    fn test_argon2_params() {
        // Verify that the hash contains expected parameters
        let hash = hash_password("test_password").unwrap();

        // Should contain memory cost (m=65536)
        assert!(hash.contains("m=65536"));
        // Should contain time cost (t=3)
        assert!(hash.contains("t=3"));
        // Should contain parallelism (p=4)
        assert!(hash.contains("p=4"));
    }
}
