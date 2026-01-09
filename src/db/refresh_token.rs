//! Refresh token repository for JWT authentication.

use super::DbPool;
use crate::Result;

#[cfg(feature = "sqlite")]
const SQL_NOW: &str = "datetime('now')";
#[cfg(feature = "postgres")]
const SQL_NOW: &str = "TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS')";

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
pub struct NewRefreshToken {
    /// User ID.
    pub user_id: i64,
    /// Token string.
    pub token: String,
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
    pub async fn create(&self, new_token: &NewRefreshToken) -> Result<RefreshToken> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(new_token.user_id)
        .bind(&new_token.token)
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
    pub async fn get_by_token(&self, token: &str) -> Result<Option<RefreshToken>> {
        let result = sqlx::query_as::<_, RefreshToken>(
            "SELECT id, user_id, token, expires_at, created_at, revoked_at
             FROM refresh_tokens WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Get a valid (not expired, not revoked) refresh token.
    pub async fn get_valid_token(&self, token: &str) -> Result<Option<RefreshToken>> {
        let sql = format!(
            "SELECT id, user_id, token, expires_at, created_at, revoked_at
             FROM refresh_tokens
             WHERE token = $1
               AND revoked_at IS NULL
               AND expires_at > {}",
            SQL_NOW
        );
        let result = sqlx::query_as::<_, RefreshToken>(&sql)
            .bind(token)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Revoke a refresh token.
    pub async fn revoke(&self, token: &str) -> Result<bool> {
        let sql = format!(
            "UPDATE refresh_tokens SET revoked_at = {} WHERE token = $1 AND revoked_at IS NULL",
            SQL_NOW
        );
        let result = sqlx::query(&sql)
            .bind(token)
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

        let new_token = NewRefreshToken {
            user_id: 1,
            token: "test-token-123".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };

        let token = repo.create(&new_token).await.unwrap();
        assert_eq!(token.user_id, 1);
        assert_eq!(token.token, "test-token-123");
        assert!(token.revoked_at.is_none());
    }

    #[tokio::test]
    async fn test_get_by_token() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        let new_token = NewRefreshToken {
            user_id: 1,
            token: "lookup-token-456".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).await.unwrap();

        let found = repo.get_by_token("lookup-token-456").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().token, "lookup-token-456");

        let not_found = repo.get_by_token("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_valid_token() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        // Create a valid token
        let valid_token = NewRefreshToken {
            user_id: 1,
            token: "valid-token".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&valid_token).await.unwrap();

        // Create an expired token
        let expired_token = NewRefreshToken {
            user_id: 1,
            token: "expired-token".to_string(),
            expires_at: "2000-01-01 00:00:00".to_string(),
        };
        repo.create(&expired_token).await.unwrap();

        // Valid token should be found
        let found = repo.get_valid_token("valid-token").await.unwrap();
        assert!(found.is_some());

        // Expired token should not be found
        let not_found = repo.get_valid_token("expired-token").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        let new_token = NewRefreshToken {
            user_id: 1,
            token: "revoke-me".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).await.unwrap();

        // Revoke the token
        let revoked = repo.revoke("revoke-me").await.unwrap();
        assert!(revoked);

        // Token should no longer be valid
        let found = repo.get_valid_token("revoke-me").await.unwrap();
        assert!(found.is_none());

        // But should still exist in get_by_token
        let exists = repo.get_by_token("revoke-me").await.unwrap();
        assert!(exists.is_some());
        assert!(exists.unwrap().revoked_at.is_some());
    }

    #[tokio::test]
    async fn test_revoke_all_for_user() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        // Create multiple tokens for the user
        for i in 0..3 {
            let new_token = NewRefreshToken {
                user_id: 1,
                token: format!("user-token-{}", i),
                expires_at: "2099-12-31 23:59:59".to_string(),
            };
            repo.create(&new_token).await.unwrap();
        }

        // Revoke all tokens for user
        let count = repo.revoke_all_for_user(1).await.unwrap();
        assert_eq!(count, 3);

        // All tokens should be invalid
        for i in 0..3 {
            let found = repo
                .get_valid_token(&format!("user-token-{}", i))
                .await
                .unwrap();
            assert!(found.is_none());
        }
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let db = setup_db().await;
        let repo = RefreshTokenRepository::new(db.pool());

        // Create an expired token
        let expired = NewRefreshToken {
            user_id: 1,
            token: "old-expired".to_string(),
            expires_at: "2000-01-01 00:00:00".to_string(),
        };
        repo.create(&expired).await.unwrap();

        // Create a valid token
        let valid = NewRefreshToken {
            user_id: 1,
            token: "still-valid".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&valid).await.unwrap();

        // Cleanup
        let deleted = repo.cleanup_expired().await.unwrap();
        assert_eq!(deleted, 1);

        // Expired token should be gone
        let gone = repo.get_by_token("old-expired").await.unwrap();
        assert!(gone.is_none());

        // Valid token should still exist
        let exists = repo.get_by_token("still-valid").await.unwrap();
        assert!(exists.is_some());
    }
}
