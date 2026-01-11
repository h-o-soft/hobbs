//! Refresh token repository for JWT authentication.
//!
//! Security: Refresh tokens are stored as SHA256 hashes in the database.
//! The raw token is only known to the client; the server stores and compares hashes.

use sha2::{Digest, Sha256};

use super::DbPool;
use crate::Result;

#[cfg(feature = "sqlite")]
const SQL_NOW: &str = "datetime('now')";
#[cfg(feature = "postgres")]
const SQL_NOW: &str = "TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS')";

/// Hash a token using SHA256.
///
/// This function is used to hash refresh tokens before storing or comparing them.
/// The raw token should never be stored in the database.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Refresh token entity.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshToken {
    /// Token ID.
    pub id: i64,
    /// User ID.
    pub user_id: i64,
    /// Token string.
    pub token: String,
    /// Expiration timestamp.
    pub expires_at: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Revocation timestamp (None if not revoked).
    pub revoked_at: Option<String>,
}

/// New refresh token for creation.
///
/// Note: The `token_hash` field should contain the SHA256 hash of the raw token,
/// not the raw token itself. Use `hash_token()` to generate the hash.
pub struct NewRefreshToken {
    /// User ID.
    pub user_id: i64,
    /// Hashed token (SHA256 hash of the raw token).
    pub token_hash: String,
    /// Expiration timestamp.
    pub expires_at: String,
}

/// Repository for refresh token operations.
pub struct RefreshTokenRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> RefreshTokenRepository<'a> {
    /// Create a new repository instance.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new refresh token.
    ///
    /// The `new_token.token_hash` should already be hashed with `hash_token()`.
    pub async fn create(&self, new_token: &NewRefreshToken) -> Result<RefreshToken> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(new_token.user_id)
        .bind(&new_token.token_hash)
        .bind(&new_token.expires_at)
        .fetch_one(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| crate::HobbsError::NotFound("Refresh token not found".into()))
    }

    /// Get a refresh token by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<RefreshToken>> {
        let token = sqlx::query_as::<_, RefreshToken>(
            "SELECT id, user_id, token, expires_at, created_at, revoked_at
             FROM refresh_tokens WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(token)
    }

    /// Get a refresh token by token string.
    ///
    /// The input token is automatically hashed before comparison.
    pub async fn get_by_token(&self, token: &str) -> Result<Option<RefreshToken>> {
        let token_hash = hash_token(token);
        let result = sqlx::query_as::<_, RefreshToken>(
            "SELECT id, user_id, token, expires_at, created_at, revoked_at
             FROM refresh_tokens WHERE token = $1",
        )
        .bind(&token_hash)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Get a valid (not expired, not revoked) refresh token.
    ///
    /// The input token is automatically hashed before comparison.
    pub async fn get_valid_token(&self, token: &str) -> Result<Option<RefreshToken>> {
        let token_hash = hash_token(token);
        let sql = format!(
            "SELECT id, user_id, token, expires_at, created_at, revoked_at
             FROM refresh_tokens
             WHERE token = $1
               AND revoked_at IS NULL
               AND expires_at > {}",
            SQL_NOW
        );
        let result = sqlx::query_as::<_, RefreshToken>(&sql)
            .bind(&token_hash)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Revoke a refresh token.
    ///
    /// The input token is automatically hashed before comparison.
    pub async fn revoke(&self, token: &str) -> Result<bool> {
        let token_hash = hash_token(token);
        let sql = format!(
            "UPDATE refresh_tokens SET revoked_at = {} WHERE token = $1 AND revoked_at IS NULL",
            SQL_NOW
        );
        let result = sqlx::query(&sql)
            .bind(&token_hash)
            .execute(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke all tokens for a user.
    pub async fn revoke_all_for_user(&self, user_id: i64) -> Result<u64> {
        let sql = format!(
            "UPDATE refresh_tokens SET revoked_at = {} WHERE user_id = $1 AND revoked_at IS NULL",
            SQL_NOW
        );
        let result = sqlx::query(&sql)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Delete expired and revoked tokens (cleanup).
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let sql = format!(
            "DELETE FROM refresh_tokens WHERE expires_at < {} OR revoked_at IS NOT NULL",
            SQL_NOW
        );
        let result = sqlx::query(&sql)
            .execute(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    async fn setup_db() -> Database {
        let db = Database::open_in_memory().await.unwrap();
        // Create a test user
        sqlx::query("INSERT INTO users (username, password, nickname, role) VALUES ($1, $2, $3, $4)")
            .bind("testuser")
            .bind("hashedpassword")
            .bind("Test User")
            .bind("member")
            .execute(db.pool())
            .await
            .unwrap();
        db
    }

    #[tokio::test]
    async fn test_create_refresh_token() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        let raw_token = "test-token-123";
        let new_token = NewRefreshToken {
            user_id: 1,
            token_hash: hash_token(raw_token),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };

        let token = repo.create(&new_token).await.unwrap();
        assert_eq!(token.user_id, 1);
        // Token stored in DB is the hash, not the raw token
        assert_eq!(token.token, hash_token(raw_token));
        assert!(token.revoked_at.is_none());
    }

    #[tokio::test]
    async fn test_get_by_token() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        let raw_token = "lookup-token-456";
        let new_token = NewRefreshToken {
            user_id: 1,
            token_hash: hash_token(raw_token),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).await.unwrap();

        // Pass raw token - repository hashes it internally
        let found = repo.get_by_token(raw_token).await.unwrap();
        assert!(found.is_some());
        // DB stores the hash
        assert_eq!(found.unwrap().token, hash_token(raw_token));

        let not_found = repo.get_by_token("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_valid_token() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        // Create a valid token
        let raw_valid = "valid-token";
        let valid_token = NewRefreshToken {
            user_id: 1,
            token_hash: hash_token(raw_valid),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&valid_token).await.unwrap();

        // Create an expired token
        let raw_expired = "expired-token";
        let expired_token = NewRefreshToken {
            user_id: 1,
            token_hash: hash_token(raw_expired),
            expires_at: "2000-01-01 00:00:00".to_string(),
        };
        repo.create(&expired_token).await.unwrap();

        // Valid token should be found (pass raw token)
        let found = repo.get_valid_token(raw_valid).await.unwrap();
        assert!(found.is_some());

        // Expired token should not be found
        let not_found = repo.get_valid_token(raw_expired).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        let raw_token = "revoke-me";
        let new_token = NewRefreshToken {
            user_id: 1,
            token_hash: hash_token(raw_token),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).await.unwrap();

        // Revoke the token (pass raw token)
        let revoked = repo.revoke(raw_token).await.unwrap();
        assert!(revoked);

        // Token should no longer be valid
        let found = repo.get_valid_token(raw_token).await.unwrap();
        assert!(found.is_none());

        // But should still exist in get_by_token
        let exists = repo.get_by_token(raw_token).await.unwrap();
        assert!(exists.is_some());
        assert!(exists.unwrap().revoked_at.is_some());
    }

    #[tokio::test]
    async fn test_revoke_all_for_user() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        // Create multiple tokens for the user
        let raw_tokens: Vec<String> = (0..3).map(|i| format!("user-token-{}", i)).collect();
        for raw_token in &raw_tokens {
            let new_token = NewRefreshToken {
                user_id: 1,
                token_hash: hash_token(raw_token),
                expires_at: "2099-12-31 23:59:59".to_string(),
            };
            repo.create(&new_token).await.unwrap();
        }

        // Revoke all tokens for user
        let count = repo.revoke_all_for_user(1).await.unwrap();
        assert_eq!(count, 3);

        // All tokens should be invalid (pass raw tokens)
        for raw_token in &raw_tokens {
            let found = repo.get_valid_token(raw_token).await.unwrap();
            assert!(found.is_none());
        }
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        // Create an expired token
        let raw_expired = "old-expired";
        let expired = NewRefreshToken {
            user_id: 1,
            token_hash: hash_token(raw_expired),
            expires_at: "2000-01-01 00:00:00".to_string(),
        };
        repo.create(&expired).await.unwrap();

        // Create a valid token
        let raw_valid = "still-valid";
        let valid = NewRefreshToken {
            user_id: 1,
            token_hash: hash_token(raw_valid),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&valid).await.unwrap();

        // Cleanup
        let deleted = repo.cleanup_expired().await.unwrap();
        assert_eq!(deleted, 1);

        // Expired token should be gone (pass raw token)
        let gone = repo.get_by_token(raw_expired).await.unwrap();
        assert!(gone.is_none());

        // Valid token should still exist (pass raw token)
        let exists = repo.get_by_token(raw_valid).await.unwrap();
        assert!(exists.is_some());
    }

    #[test]
    fn test_hash_token() {
        // Test that hash_token produces consistent results
        let token = "test-token";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);
        assert_eq!(hash1, hash2);

        // Test that different tokens produce different hashes
        let hash3 = hash_token("different-token");
        assert_ne!(hash1, hash3);

        // Test that hash is 64 characters (SHA256 hex)
        assert_eq!(hash1.len(), 64);
    }
}
