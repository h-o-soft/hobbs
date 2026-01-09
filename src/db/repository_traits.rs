//! Repository trait definitions for HOBBS.
//!
//! This module defines traits for repository operations, enabling
//! different database backends to provide their own implementations.
//!
//! # Design Notes
//!
//! These traits are designed to be:
//! - **Synchronous**: Current rusqlite implementation is synchronous.
//!   Phase B will introduce async versions when migrating to sqlx.
//! - **Generic**: Work with any database backend that implements
//!   the necessary traits.
//!
//! # Usage
//!
//! Currently, the existing repository implementations (e.g., `UserRepository`)
//! remain unchanged. These traits serve as documentation and preparation
//! for Phase B migration.
//!
//! ```ignore
//! // Phase A: Use existing repositories directly
//! let repo = UserRepository::new(&db);
//! let user = repo.get_by_id(1)?;
//!
//! // Phase B (future): Use trait-based repositories
//! fn get_user<R: UserRepositoryTrait>(repo: &R, id: i64) -> Result<Option<User>> {
//!     repo.get_by_id(id)
//! }
//! ```

use crate::db::{NewUser, Role, User, UserUpdate};
use crate::Result;

/// Trait for user repository operations.
///
/// This trait defines the interface for user CRUD operations.
/// Implementations can use different database backends.
pub trait UserRepositoryTrait {
    /// Create a new user in the database.
    fn create(&self, new_user: &NewUser) -> Result<User>;

    /// Get a user by ID.
    fn get_by_id(&self, id: i64) -> Result<Option<User>>;

    /// Get a user by username (case-insensitive).
    fn get_by_username(&self, username: &str) -> Result<Option<User>>;

    /// Update a user by ID.
    fn update(&self, id: i64, update: &UserUpdate) -> Result<Option<User>>;

    /// Update the last login timestamp for a user.
    fn update_last_login(&self, id: i64) -> Result<()>;

    /// Delete a user by ID.
    fn delete(&self, id: i64) -> Result<bool>;

    /// List all active users.
    fn list_active(&self) -> Result<Vec<User>>;

    /// List all users (including inactive).
    fn list_all(&self) -> Result<Vec<User>>;

    /// List users by role.
    fn list_by_role(&self, role: Role) -> Result<Vec<User>>;

    /// Count all users.
    fn count(&self) -> Result<i64>;

    /// Count active users.
    fn count_active(&self) -> Result<i64>;

    /// Check if a username is already taken (case-insensitive).
    fn username_exists(&self, username: &str) -> Result<bool>;
}

// Note: Additional repository traits will be defined as needed during
// Phase B migration. For Phase A, we focus on the User repository as
// a reference implementation pattern.
//
// Future traits to be defined:
// - BoardRepositoryTrait
// - PostRepositoryTrait
// - ThreadRepositoryTrait
// - MailRepositoryTrait
// - ScriptRepositoryTrait
// - RssFeedRepositoryTrait
// etc.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, UserRepository};

    // Verify that UserRepository implements the trait pattern
    // (even though we don't formally implement the trait yet)
    #[test]
    fn test_user_repository_matches_trait() {
        let db = Database::open_in_memory().unwrap();
        let repo = UserRepository::new(&db);

        // Create a user
        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).unwrap();

        // Test get_by_id
        let found = repo.get_by_id(user.id).unwrap();
        assert!(found.is_some());

        // Test get_by_username
        let found = repo.get_by_username("testuser").unwrap();
        assert!(found.is_some());

        // Test username_exists
        assert!(repo.username_exists("testuser").unwrap());

        // Test count
        assert_eq!(repo.count().unwrap(), 1);
        assert_eq!(repo.count_active().unwrap(), 1);

        // Test list methods
        assert_eq!(repo.list_active().unwrap().len(), 1);
        assert_eq!(repo.list_all().unwrap().len(), 1);
        assert_eq!(repo.list_by_role(Role::Member).unwrap().len(), 1);

        // Test update
        let update = UserUpdate::new().nickname("Updated Name");
        let updated = repo.update(user.id, &update).unwrap();
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().nickname, "Updated Name");

        // Test update_last_login
        repo.update_last_login(user.id).unwrap();

        // Test delete
        assert!(repo.delete(user.id).unwrap());
        assert!(repo.get_by_id(user.id).unwrap().is_none());
    }
}
