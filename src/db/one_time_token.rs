//! One-time token repository for secure URL-based authentication.
//!
//! One-time tokens are short-lived tokens used for WebSocket connections
//! and file downloads where Authorization headers cannot be used.

use super::DbPool;
use crate::Result;

#[cfg(feature = "sqlite")]
const SQL_NOW: &str = "datetime('now')";
#[cfg(feature = "postgres")]
const SQL_NOW: &str = "TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS')";

/// Token purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    /// WebSocket connection.
    WebSocket,
    /// File download.
    Download,
}

impl TokenPurpose {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenPurpose::WebSocket => "websocket",
            TokenPurpose::Download => "download",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "websocket" => Some(TokenPurpose::WebSocket),
            "download" => Some(TokenPurpose::Download),
            _ => None,
        }
    }
}

/// One-time token entity.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OneTimeToken {
    /// Token ID.
    pub id: i64,
    /// User ID.
    pub user_id: i64,
    /// Token string.
    pub token: String,
    /// Token purpose.
    pub purpose: String,
    /// Optional target ID (e.g., file_id for downloads).
    pub target_id: Option<i64>,
    /// Expiration timestamp.
    pub expires_at: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Used timestamp (None if not used).
    pub used_at: Option<String>,
}

impl OneTimeToken {
    /// Get the token purpose as enum.
    pub fn purpose(&self) -> Option<TokenPurpose> {
        TokenPurpose::from_str(&self.purpose)
    }

    /// Check if the token has been used.
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }
}

/// New one-time token for creation.
pub struct NewOneTimeToken {
    /// User ID.
    pub user_id: i64,
    /// Token string.
    pub token: String,
    /// Token purpose.
    pub purpose: TokenPurpose,
    /// Optional target ID.
    pub target_id: Option<i64>,
    /// Expiration timestamp.
    pub expires_at: String,
}

/// Repository for one-time token operations.
pub struct OneTimeTokenRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> OneTimeTokenRepository<'a> {
    /// Create a new repository instance.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new one-time token.
    pub async fn create(&self, new_token: &NewOneTimeToken) -> Result<OneTimeToken> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO one_time_tokens (user_id, token, purpose, target_id, expires_at)
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(new_token.user_id)
        .bind(&new_token.token)
        .bind(new_token.purpose.as_str())
        .bind(new_token.target_id)
        .bind(&new_token.expires_at)
        .fetch_one(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| crate::HobbsError::NotFound("One-time token not found".into()))
    }

    /// Get a one-time token by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<OneTimeToken>> {
        let token = sqlx::query_as::<_, OneTimeToken>(
            "SELECT id, user_id, token, purpose, target_id, expires_at, created_at, used_at
             FROM one_time_tokens WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(token)
    }

    /// Get a valid (not expired, not used) one-time token and mark it as used atomically.
    ///
    /// Returns the token if it was valid and successfully marked as used.
    /// This ensures the token can only be used once even with concurrent requests.
    pub async fn consume_token(
        &self,
        token: &str,
        purpose: TokenPurpose,
        target_id: Option<i64>,
    ) -> Result<Option<OneTimeToken>> {
        // Use UPDATE ... RETURNING to atomically mark as used and return the token
        // This prevents race conditions where multiple requests try to use the same token
        let sql = if target_id.is_some() {
            format!(
                "UPDATE one_time_tokens
                 SET used_at = {}
                 WHERE token = $1
                   AND purpose = $2
                   AND target_id = $3
                   AND used_at IS NULL
                   AND expires_at > {}
                 RETURNING id, user_id, token, purpose, target_id, expires_at, created_at, used_at",
                SQL_NOW, SQL_NOW
            )
        } else {
            format!(
                "UPDATE one_time_tokens
                 SET used_at = {}
                 WHERE token = $1
                   AND purpose = $2
                   AND target_id IS NULL
                   AND used_at IS NULL
                   AND expires_at > {}
                 RETURNING id, user_id, token, purpose, target_id, expires_at, created_at, used_at",
                SQL_NOW, SQL_NOW
            )
        };

        let result = if target_id.is_some() {
            sqlx::query_as::<_, OneTimeToken>(&sql)
                .bind(token)
                .bind(purpose.as_str())
                .bind(target_id)
                .fetch_optional(self.pool)
                .await
        } else {
            sqlx::query_as::<_, OneTimeToken>(&sql)
                .bind(token)
                .bind(purpose.as_str())
                .fetch_optional(self.pool)
                .await
        };

        result.map_err(|e| crate::HobbsError::Database(e.to_string()))
    }

    /// Delete expired and used tokens (cleanup).
    pub async fn cleanup(&self) -> Result<u64> {
        let sql = format!(
            "DELETE FROM one_time_tokens WHERE expires_at < {} OR used_at IS NOT NULL",
            SQL_NOW
        );
        let result = sqlx::query(&sql)
            .execute(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Delete all tokens for a user.
    pub async fn delete_all_for_user(&self, user_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM one_time_tokens WHERE user_id = $1")
            .bind(user_id)
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
        sqlx::query(
            "INSERT INTO users (username, password, nickname, role) VALUES ($1, $2, $3, $4)",
        )
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
    async fn test_create_one_time_token() {
        let db = setup_db().await;
        let repo = OneTimeTokenRepository::new(db.pool());

        let new_token = NewOneTimeToken {
            user_id: 1,
            token: "test-token-123".to_string(),
            purpose: TokenPurpose::WebSocket,
            target_id: None,
            expires_at: "2099-12-31 23:59:59".to_string(),
        };

        let token = repo.create(&new_token).await.unwrap();
        assert_eq!(token.user_id, 1);
        assert_eq!(token.token, "test-token-123");
        assert_eq!(token.purpose, "websocket");
        assert!(token.used_at.is_none());
    }

    #[tokio::test]
    async fn test_consume_token_websocket() {
        let db = setup_db().await;
        let repo = OneTimeTokenRepository::new(db.pool());

        let new_token = NewOneTimeToken {
            user_id: 1,
            token: "ws-token".to_string(),
            purpose: TokenPurpose::WebSocket,
            target_id: None,
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).await.unwrap();

        // First consume should succeed
        let consumed = repo
            .consume_token("ws-token", TokenPurpose::WebSocket, None)
            .await
            .unwrap();
        assert!(consumed.is_some());
        let consumed = consumed.unwrap();
        assert_eq!(consumed.user_id, 1);
        assert!(consumed.used_at.is_some());

        // Second consume should fail (already used)
        let second = repo
            .consume_token("ws-token", TokenPurpose::WebSocket, None)
            .await
            .unwrap();
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn test_consume_token_download_with_target() {
        let db = setup_db().await;
        let repo = OneTimeTokenRepository::new(db.pool());

        let new_token = NewOneTimeToken {
            user_id: 1,
            token: "dl-token".to_string(),
            purpose: TokenPurpose::Download,
            target_id: Some(42),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).await.unwrap();

        // Consume with correct target_id
        let consumed = repo
            .consume_token("dl-token", TokenPurpose::Download, Some(42))
            .await
            .unwrap();
        assert!(consumed.is_some());

        // Wrong target_id should not match
        let new_token2 = NewOneTimeToken {
            user_id: 1,
            token: "dl-token-2".to_string(),
            purpose: TokenPurpose::Download,
            target_id: Some(42),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token2).await.unwrap();

        let wrong_target = repo
            .consume_token("dl-token-2", TokenPurpose::Download, Some(99))
            .await
            .unwrap();
        assert!(wrong_target.is_none());
    }

    #[tokio::test]
    async fn test_consume_expired_token() {
        let db = setup_db().await;
        let repo = OneTimeTokenRepository::new(db.pool());

        let new_token = NewOneTimeToken {
            user_id: 1,
            token: "expired-token".to_string(),
            purpose: TokenPurpose::WebSocket,
            target_id: None,
            expires_at: "2000-01-01 00:00:00".to_string(), // Already expired
        };
        repo.create(&new_token).await.unwrap();

        // Expired token should not be consumable
        let consumed = repo
            .consume_token("expired-token", TokenPurpose::WebSocket, None)
            .await
            .unwrap();
        assert!(consumed.is_none());
    }

    #[tokio::test]
    async fn test_wrong_purpose() {
        let db = setup_db().await;
        let repo = OneTimeTokenRepository::new(db.pool());

        let new_token = NewOneTimeToken {
            user_id: 1,
            token: "purpose-token".to_string(),
            purpose: TokenPurpose::WebSocket,
            target_id: None,
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).await.unwrap();

        // Wrong purpose should not match
        let consumed = repo
            .consume_token("purpose-token", TokenPurpose::Download, None)
            .await
            .unwrap();
        assert!(consumed.is_none());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let db = setup_db().await;
        let repo = OneTimeTokenRepository::new(db.pool());

        // Create an expired token
        let expired = NewOneTimeToken {
            user_id: 1,
            token: "cleanup-expired".to_string(),
            purpose: TokenPurpose::WebSocket,
            target_id: None,
            expires_at: "2000-01-01 00:00:00".to_string(),
        };
        repo.create(&expired).await.unwrap();

        // Create a valid token and consume it
        let used = NewOneTimeToken {
            user_id: 1,
            token: "cleanup-used".to_string(),
            purpose: TokenPurpose::WebSocket,
            target_id: None,
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&used).await.unwrap();
        repo.consume_token("cleanup-used", TokenPurpose::WebSocket, None)
            .await
            .unwrap();

        // Create a valid unused token
        let valid = NewOneTimeToken {
            user_id: 1,
            token: "cleanup-valid".to_string(),
            purpose: TokenPurpose::WebSocket,
            target_id: None,
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&valid).await.unwrap();

        // Cleanup should remove expired and used tokens
        let deleted = repo.cleanup().await.unwrap();
        assert_eq!(deleted, 2);

        // Valid token should still exist
        let still_valid = repo
            .consume_token("cleanup-valid", TokenPurpose::WebSocket, None)
            .await
            .unwrap();
        assert!(still_valid.is_some());
    }

    #[tokio::test]
    async fn test_token_purpose_conversion() {
        assert_eq!(TokenPurpose::WebSocket.as_str(), "websocket");
        assert_eq!(TokenPurpose::Download.as_str(), "download");

        assert_eq!(
            TokenPurpose::from_str("websocket"),
            Some(TokenPurpose::WebSocket)
        );
        assert_eq!(
            TokenPurpose::from_str("download"),
            Some(TokenPurpose::Download)
        );
        assert_eq!(TokenPurpose::from_str("unknown"), None);
    }
}
