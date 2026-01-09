//! User repository for HOBBS.
//!
//! This module provides CRUD operations for users in the database.

use sqlx::{QueryBuilder, SqlitePool};

use super::user::{NewUser, Role, User, UserUpdate};
use crate::{HobbsError, Result};

/// Repository for user CRUD operations.
pub struct UserRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> UserRepository<'a> {
    /// Create a new UserRepository with the given database pool reference.
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new user in the database.
    ///
    /// Returns the created user with the assigned ID.
    pub async fn create(&self, new_user: &NewUser) -> Result<User> {
        let result = sqlx::query(
            "INSERT INTO users (username, password, nickname, email, role, terminal, encoding, language, auto_paging)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&new_user.username)
        .bind(&new_user.password)
        .bind(&new_user.nickname)
        .bind(&new_user.email)
        .bind(new_user.role.as_str())
        .bind(&new_user.terminal)
        .bind(new_user.encoding.as_str())
        .bind(&new_user.language)
        .bind(new_user.auto_paging)
        .execute(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        let id = result.last_insert_rowid();
        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("user".to_string()))
    }

    /// Get a user by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<User>> {
        let result = sqlx::query_as::<_, User>(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Get a user by username (case-insensitive).
    pub async fn get_by_username(&self, username: &str) -> Result<Option<User>> {
        let result = sqlx::query_as::<_, User>(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Update a user by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated user, or None if not found.
    pub async fn update(&self, id: i64, update: &UserUpdate) -> Result<Option<User>> {
        if update.is_empty() {
            return self.get_by_id(id).await;
        }

        let mut query: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("UPDATE users SET ");
        let mut separated = query.separated(", ");

        if let Some(ref password) = update.password {
            separated.push("password = ");
            separated.push_bind_unseparated(password);
        }
        if let Some(ref nickname) = update.nickname {
            separated.push("nickname = ");
            separated.push_bind_unseparated(nickname);
        }
        if let Some(ref email) = update.email {
            separated.push("email = ");
            separated.push_bind_unseparated(email.clone());
        }
        if let Some(role) = update.role {
            separated.push("role = ");
            separated.push_bind_unseparated(role.as_str().to_string());
        }
        if let Some(ref profile) = update.profile {
            separated.push("profile = ");
            separated.push_bind_unseparated(profile.clone());
        }
        if let Some(ref terminal) = update.terminal {
            separated.push("terminal = ");
            separated.push_bind_unseparated(terminal);
        }
        if let Some(encoding) = update.encoding {
            separated.push("encoding = ");
            separated.push_bind_unseparated(encoding.as_str().to_string());
        }
        if let Some(ref language) = update.language {
            separated.push("language = ");
            separated.push_bind_unseparated(language);
        }
        if let Some(is_active) = update.is_active {
            separated.push("is_active = ");
            separated.push_bind_unseparated(is_active);
        }
        if let Some(auto_paging) = update.auto_paging {
            separated.push("auto_paging = ");
            separated.push_bind_unseparated(auto_paging);
        }

        query.push(" WHERE id = ");
        query.push_bind(id);

        let result = query
            .build()
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_by_id(id).await
    }

    /// Update the last login timestamp for a user.
    pub async fn update_last_login(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE users SET last_login = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete a user by ID.
    ///
    /// Returns true if a user was deleted, false if not found.
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }

    /// List all active users.
    pub async fn list_active(&self) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users WHERE is_active = 1 ORDER BY username",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(users)
    }

    /// List all users (including inactive).
    pub async fn list_all(&self) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users ORDER BY username",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(users)
    }

    /// List users by role.
    pub async fn list_by_role(&self, role: Role) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users WHERE role = ? AND is_active = 1 ORDER BY username",
        )
        .bind(role.as_str())
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(users)
    }

    /// Count all users.
    pub async fn count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(count.0)
    }

    /// Count active users.
    pub async fn count_active(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = 1")
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(count.0)
    }

    /// Check if a username is already taken (case-insensitive).
    pub async fn username_exists(&self, username: &str) -> Result<bool> {
        let exists: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM users WHERE username = ? COLLATE NOCASE)")
                .bind(username)
                .fetch_one(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(exists.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::CharacterEncoding;
    use crate::Database;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_create_user() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).await.unwrap();

        assert_eq!(user.id, 1);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.nickname, "Test User");
        assert_eq!(user.role, Role::Member);
        assert!(user.is_active);
    }

    #[tokio::test]
    async fn test_create_user_with_options() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("admin", "hashedpw", "Administrator")
            .with_email("admin@example.com")
            .with_role(Role::SysOp)
            .with_terminal("c64");

        let user = repo.create(&new_user).await.unwrap();

        assert_eq!(user.username, "admin");
        assert_eq!(user.email, Some("admin@example.com".to_string()));
        assert_eq!(user.role, Role::SysOp);
        assert_eq!(user.terminal, "c64");
    }

    #[tokio::test]
    async fn test_create_duplicate_username() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        repo.create(&new_user).await.unwrap();

        let duplicate = NewUser::new("testuser", "otherpw", "Other User");
        let result = repo.create(&duplicate).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let created = repo.create(&new_user).await.unwrap();

        let found = repo.get_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "testuser");

        let not_found = repo.get_by_id(999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_by_username() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        repo.create(&new_user).await.unwrap();

        let found = repo.get_by_username("testuser").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().nickname, "Test User");

        let not_found = repo.get_by_username("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_user() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).await.unwrap();

        let update = UserUpdate::new()
            .nickname("Updated Name")
            .email(Some("new@example.com".to_string()))
            .role(Role::SubOp);

        let updated = repo.update(user.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.nickname, "Updated Name");
        assert_eq!(updated.email, Some("new@example.com".to_string()));
        assert_eq!(updated.role, Role::SubOp);
        // Unchanged fields
        assert_eq!(updated.username, "testuser");
        assert_eq!(updated.password, "hashedpw");
    }

    #[tokio::test]
    async fn test_update_nonexistent_user() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let update = UserUpdate::new().nickname("New Name");
        let result = repo.update(999, &update).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_empty() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).await.unwrap();

        let update = UserUpdate::new();
        let result = repo.update(user.id, &update).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().nickname, "Test User");
    }

    #[tokio::test]
    async fn test_update_is_active() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).await.unwrap();
        assert!(user.is_active);

        let update = UserUpdate::new().is_active(false);
        let updated = repo.update(user.id, &update).await.unwrap().unwrap();

        assert!(!updated.is_active);
    }

    #[tokio::test]
    async fn test_update_last_login() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).await.unwrap();
        assert!(user.last_login.is_none());

        repo.update_last_login(user.id).await.unwrap();

        let updated = repo.get_by_id(user.id).await.unwrap().unwrap();
        assert!(updated.last_login.is_some());
    }

    #[tokio::test]
    async fn test_delete_user() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).await.unwrap();

        let deleted = repo.delete(user.id).await.unwrap();
        assert!(deleted);

        let found = repo.get_by_id(user.id).await.unwrap();
        assert!(found.is_none());

        // Deleting again should return false
        let deleted_again = repo.delete(user.id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_list_active() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        // Create some users
        repo.create(&NewUser::new("user1", "pw", "User 1"))
            .await
            .unwrap();
        let user2 = repo
            .create(&NewUser::new("user2", "pw", "User 2"))
            .await
            .unwrap();
        repo.create(&NewUser::new("user3", "pw", "User 3"))
            .await
            .unwrap();

        // Deactivate user2
        repo.update(user2.id, &UserUpdate::new().is_active(false))
            .await
            .unwrap();

        let active = repo.list_active().await.unwrap();
        assert_eq!(active.len(), 2);
        assert!(active.iter().all(|u| u.username != "user2"));
    }

    #[tokio::test]
    async fn test_list_all() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        repo.create(&NewUser::new("user1", "pw", "User 1"))
            .await
            .unwrap();
        let user2 = repo
            .create(&NewUser::new("user2", "pw", "User 2"))
            .await
            .unwrap();
        repo.create(&NewUser::new("user3", "pw", "User 3"))
            .await
            .unwrap();

        // Deactivate user2
        repo.update(user2.id, &UserUpdate::new().is_active(false))
            .await
            .unwrap();

        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_list_by_role() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        repo.create(&NewUser::new("member1", "pw", "Member 1"))
            .await
            .unwrap();
        repo.create(&NewUser::new("member2", "pw", "Member 2"))
            .await
            .unwrap();
        repo.create(&NewUser::new("subop", "pw", "SubOp").with_role(Role::SubOp))
            .await
            .unwrap();
        repo.create(&NewUser::new("sysop", "pw", "SysOp").with_role(Role::SysOp))
            .await
            .unwrap();

        let members = repo.list_by_role(Role::Member).await.unwrap();
        assert_eq!(members.len(), 2);

        let subops = repo.list_by_role(Role::SubOp).await.unwrap();
        assert_eq!(subops.len(), 1);

        let sysops = repo.list_by_role(Role::SysOp).await.unwrap();
        assert_eq!(sysops.len(), 1);
    }

    #[tokio::test]
    async fn test_count() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        assert_eq!(repo.count().await.unwrap(), 0);
        assert_eq!(repo.count_active().await.unwrap(), 0);

        repo.create(&NewUser::new("user1", "pw", "User 1"))
            .await
            .unwrap();
        let user2 = repo
            .create(&NewUser::new("user2", "pw", "User 2"))
            .await
            .unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(repo.count_active().await.unwrap(), 2);

        repo.update(user2.id, &UserUpdate::new().is_active(false))
            .await
            .unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(repo.count_active().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_username_exists() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        assert!(!repo.username_exists("testuser").await.unwrap());

        repo.create(&NewUser::new("testuser", "pw", "Test"))
            .await
            .unwrap();

        assert!(repo.username_exists("testuser").await.unwrap());
        assert!(!repo.username_exists("other").await.unwrap());
    }

    #[tokio::test]
    async fn test_get_by_username_case_insensitive() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("TestUser", "hashedpw", "Test User");
        repo.create(&new_user).await.unwrap();

        // Should find with exact case
        let found = repo.get_by_username("TestUser").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "TestUser");

        // Should find with lowercase
        let found_lower = repo.get_by_username("testuser").await.unwrap();
        assert!(found_lower.is_some());
        assert_eq!(found_lower.unwrap().username, "TestUser");

        // Should find with uppercase
        let found_upper = repo.get_by_username("TESTUSER").await.unwrap();
        assert!(found_upper.is_some());
        assert_eq!(found_upper.unwrap().username, "TestUser");

        // Should find with mixed case
        let found_mixed = repo.get_by_username("tEsTuSeR").await.unwrap();
        assert!(found_mixed.is_some());
        assert_eq!(found_mixed.unwrap().username, "TestUser");
    }

    #[tokio::test]
    async fn test_username_exists_case_insensitive() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        repo.create(&NewUser::new("TestUser", "pw", "Test"))
            .await
            .unwrap();

        // Should detect existence regardless of case
        assert!(repo.username_exists("TestUser").await.unwrap());
        assert!(repo.username_exists("testuser").await.unwrap());
        assert!(repo.username_exists("TESTUSER").await.unwrap());
        assert!(repo.username_exists("tEsTuSeR").await.unwrap());
        assert!(!repo.username_exists("other").await.unwrap());
    }

    #[tokio::test]
    async fn test_create_duplicate_username_different_case() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("TestUser", "hashedpw", "Test User");
        repo.create(&new_user).await.unwrap();

        // Attempting to create user with same name but different case should fail
        let duplicate_lower = NewUser::new("testuser", "otherpw", "Other User");
        let result = repo.create(&duplicate_lower).await;
        assert!(result.is_err());

        let duplicate_upper = NewUser::new("TESTUSER", "otherpw", "Other User");
        let result = repo.create(&duplicate_upper).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_user_with_encoding() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user =
            NewUser::new("testuser", "hash", "Test").with_encoding(CharacterEncoding::Utf8);
        let user = repo.create(&new_user).await.unwrap();

        assert_eq!(user.encoding, CharacterEncoding::Utf8);
    }

    #[tokio::test]
    async fn test_create_user_default_encoding() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hash", "Test");
        let user = repo.create(&new_user).await.unwrap();

        assert_eq!(user.encoding, CharacterEncoding::ShiftJIS);
    }

    #[tokio::test]
    async fn test_update_encoding() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hash", "Test");
        let user = repo.create(&new_user).await.unwrap();
        assert_eq!(user.encoding, CharacterEncoding::ShiftJIS);

        let update = UserUpdate::new().encoding(CharacterEncoding::Utf8);
        let updated = repo.update(user.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.encoding, CharacterEncoding::Utf8);
    }

    #[tokio::test]
    async fn test_create_user_with_language() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hash", "Test").with_language("ja");
        let user = repo.create(&new_user).await.unwrap();

        assert_eq!(user.language, "ja");
    }

    #[tokio::test]
    async fn test_create_user_default_language() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hash", "Test");
        let user = repo.create(&new_user).await.unwrap();

        assert_eq!(user.language, "en");
    }

    #[tokio::test]
    async fn test_update_language() {
        let db = setup_db().await;
        let repo = UserRepository::new(db.pool());

        let new_user = NewUser::new("testuser", "hash", "Test");
        let user = repo.create(&new_user).await.unwrap();
        assert_eq!(user.language, "en");

        let update = UserUpdate::new().language("ja");
        let updated = repo.update(user.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.language, "ja");
    }
}
