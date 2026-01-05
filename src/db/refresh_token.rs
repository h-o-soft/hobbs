//! Refresh token repository for JWT authentication.

use rusqlite::{params, Connection, OptionalExtension};

use crate::Result;

/// Refresh token entity.
#[derive(Debug, Clone)]
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
    conn: &'a Connection,
}

impl<'a> RefreshTokenRepository<'a> {
    /// Create a new repository instance.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Create a new refresh token.
    pub fn create(&self, new_token: &NewRefreshToken) -> Result<RefreshToken> {
        self.conn.execute(
            "INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES (?, ?, ?)",
            params![new_token.user_id, new_token.token, new_token.expires_at],
        )?;

        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)?
            .ok_or_else(|| crate::HobbsError::NotFound("Refresh token not found".into()))
    }

    /// Get a refresh token by ID.
    pub fn get_by_id(&self, id: i64) -> Result<Option<RefreshToken>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, token, expires_at, created_at, revoked_at
             FROM refresh_tokens WHERE id = ?",
        )?;

        let token = stmt.query_row([id], Self::row_to_token).optional()?;
        Ok(token)
    }

    /// Get a refresh token by token string.
    pub fn get_by_token(&self, token: &str) -> Result<Option<RefreshToken>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, token, expires_at, created_at, revoked_at
             FROM refresh_tokens WHERE token = ?",
        )?;

        let result = stmt.query_row([token], Self::row_to_token).optional()?;
        Ok(result)
    }

    /// Get a valid (not expired, not revoked) refresh token.
    pub fn get_valid_token(&self, token: &str) -> Result<Option<RefreshToken>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, token, expires_at, created_at, revoked_at
             FROM refresh_tokens
             WHERE token = ?
               AND revoked_at IS NULL
               AND datetime(expires_at) > datetime('now')",
        )?;

        let result = stmt.query_row([token], Self::row_to_token).optional()?;
        Ok(result)
    }

    /// Revoke a refresh token.
    pub fn revoke(&self, token: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE refresh_tokens SET revoked_at = datetime('now') WHERE token = ? AND revoked_at IS NULL",
            [token],
        )?;
        Ok(rows > 0)
    }

    /// Revoke all tokens for a user.
    pub fn revoke_all_for_user(&self, user_id: i64) -> Result<usize> {
        let rows = self.conn.execute(
            "UPDATE refresh_tokens SET revoked_at = datetime('now') WHERE user_id = ? AND revoked_at IS NULL",
            [user_id],
        )?;
        Ok(rows)
    }

    /// Delete expired and revoked tokens (cleanup).
    pub fn cleanup_expired(&self) -> Result<usize> {
        let rows = self.conn.execute(
            "DELETE FROM refresh_tokens WHERE datetime(expires_at) < datetime('now') OR revoked_at IS NOT NULL",
            [],
        )?;
        Ok(rows)
    }

    /// Convert a database row to a RefreshToken.
    fn row_to_token(row: &rusqlite::Row) -> rusqlite::Result<RefreshToken> {
        Ok(RefreshToken {
            id: row.get(0)?,
            user_id: row.get(1)?,
            token: row.get(2)?,
            expires_at: row.get(3)?,
            created_at: row.get(4)?,
            revoked_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        // Create a test user
        db.conn()
            .execute(
                "INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)",
                ["testuser", "hashedpassword", "Test User", "member"],
            )
            .unwrap();
        db
    }

    #[test]
    fn test_create_refresh_token() {
        let db = setup_db();
        let repo = RefreshTokenRepository::new(db.conn());

        let new_token = NewRefreshToken {
            user_id: 1,
            token: "test-token-123".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };

        let token = repo.create(&new_token).unwrap();
        assert_eq!(token.user_id, 1);
        assert_eq!(token.token, "test-token-123");
        assert!(token.revoked_at.is_none());
    }

    #[test]
    fn test_get_by_token() {
        let db = setup_db();
        let repo = RefreshTokenRepository::new(db.conn());

        let new_token = NewRefreshToken {
            user_id: 1,
            token: "lookup-token-456".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).unwrap();

        let found = repo.get_by_token("lookup-token-456").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().token, "lookup-token-456");

        let not_found = repo.get_by_token("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_valid_token() {
        let db = setup_db();
        let repo = RefreshTokenRepository::new(db.conn());

        // Create a valid token
        let valid_token = NewRefreshToken {
            user_id: 1,
            token: "valid-token".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&valid_token).unwrap();

        // Create an expired token
        let expired_token = NewRefreshToken {
            user_id: 1,
            token: "expired-token".to_string(),
            expires_at: "2000-01-01 00:00:00".to_string(),
        };
        repo.create(&expired_token).unwrap();

        // Valid token should be found
        let found = repo.get_valid_token("valid-token").unwrap();
        assert!(found.is_some());

        // Expired token should not be found
        let not_found = repo.get_valid_token("expired-token").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_revoke_token() {
        let db = setup_db();
        let repo = RefreshTokenRepository::new(db.conn());

        let new_token = NewRefreshToken {
            user_id: 1,
            token: "revoke-me".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&new_token).unwrap();

        // Revoke the token
        let revoked = repo.revoke("revoke-me").unwrap();
        assert!(revoked);

        // Token should no longer be valid
        let found = repo.get_valid_token("revoke-me").unwrap();
        assert!(found.is_none());

        // But should still exist in get_by_token
        let exists = repo.get_by_token("revoke-me").unwrap();
        assert!(exists.is_some());
        assert!(exists.unwrap().revoked_at.is_some());
    }

    #[test]
    fn test_revoke_all_for_user() {
        let db = setup_db();
        let repo = RefreshTokenRepository::new(db.conn());

        // Create multiple tokens for the user
        for i in 0..3 {
            let new_token = NewRefreshToken {
                user_id: 1,
                token: format!("user-token-{}", i),
                expires_at: "2099-12-31 23:59:59".to_string(),
            };
            repo.create(&new_token).unwrap();
        }

        // Revoke all tokens for user
        let count = repo.revoke_all_for_user(1).unwrap();
        assert_eq!(count, 3);

        // All tokens should be invalid
        for i in 0..3 {
            let found = repo.get_valid_token(&format!("user-token-{}", i)).unwrap();
            assert!(found.is_none());
        }
    }

    #[test]
    fn test_cleanup_expired() {
        let db = setup_db();
        let repo = RefreshTokenRepository::new(db.conn());

        // Create an expired token
        let expired = NewRefreshToken {
            user_id: 1,
            token: "old-expired".to_string(),
            expires_at: "2000-01-01 00:00:00".to_string(),
        };
        repo.create(&expired).unwrap();

        // Create a valid token
        let valid = NewRefreshToken {
            user_id: 1,
            token: "still-valid".to_string(),
            expires_at: "2099-12-31 23:59:59".to_string(),
        };
        repo.create(&valid).unwrap();

        // Cleanup
        let deleted = repo.cleanup_expired().unwrap();
        assert_eq!(deleted, 1);

        // Expired token should be gone
        let gone = repo.get_by_token("old-expired").unwrap();
        assert!(gone.is_none());

        // Valid token should still exist
        let exists = repo.get_by_token("still-valid").unwrap();
        assert!(exists.is_some());
    }
}
