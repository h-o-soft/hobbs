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

use chrono::{DateTime, Utc};

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

// ============================================================================
// Board Repository Trait
// ============================================================================

// Re-export types needed for the trait (these are defined in crate::board)
// For Phase A, we define the trait with placeholder types that will be
// resolved when the trait is implemented.

/// Trait for board repository operations.
///
/// This trait defines the interface for board CRUD operations.
/// Implementations can use different database backends.
pub trait BoardRepositoryTrait {
    /// The board type used by this implementation.
    type Board;
    /// The new board type used by this implementation.
    type NewBoard;
    /// The board update type used by this implementation.
    type BoardUpdate;
    /// The board type enum used by this implementation.
    type BoardType;

    /// Create a new board in the database.
    fn create(&self, new_board: &Self::NewBoard) -> Result<Self::Board>;

    /// Get a board by ID.
    fn get_by_id(&self, id: i64) -> Result<Option<Self::Board>>;

    /// Get a board by name.
    fn get_by_name(&self, name: &str) -> Result<Option<Self::Board>>;

    /// Update a board by ID.
    fn update(&self, id: i64, update: &Self::BoardUpdate) -> Result<Option<Self::Board>>;

    /// Delete a board by ID.
    fn delete(&self, id: i64) -> Result<bool>;

    /// List all active boards.
    fn list_active(&self) -> Result<Vec<Self::Board>>;

    /// List all boards (including inactive).
    fn list_all(&self) -> Result<Vec<Self::Board>>;

    /// List boards accessible by a user with the given role.
    fn list_accessible(&self, user_role: Role) -> Result<Vec<Self::Board>>;

    /// List boards writable by a user with the given role.
    fn list_writable(&self, user_role: Role) -> Result<Vec<Self::Board>>;

    /// Count all boards.
    fn count(&self) -> Result<i64>;

    /// Count active boards.
    fn count_active(&self) -> Result<i64>;

    /// Check if a board name is already taken.
    fn name_exists(&self, name: &str) -> Result<bool>;
}

// ============================================================================
// Thread Repository Trait
// ============================================================================

/// Trait for thread repository operations.
///
/// This trait defines the interface for thread CRUD operations.
/// Implementations can use different database backends.
pub trait ThreadRepositoryTrait {
    /// The thread type used by this implementation.
    type Thread;
    /// The new thread type used by this implementation.
    type NewThread;
    /// The thread update type used by this implementation.
    type ThreadUpdate;

    /// Create a new thread in the database.
    fn create(&self, new_thread: &Self::NewThread) -> Result<Self::Thread>;

    /// Get a thread by ID.
    fn get_by_id(&self, id: i64) -> Result<Option<Self::Thread>>;

    /// Update a thread by ID.
    fn update(&self, id: i64, update: &Self::ThreadUpdate) -> Result<Option<Self::Thread>>;

    /// Delete a thread by ID.
    fn delete(&self, id: i64) -> Result<bool>;

    /// List threads in a board, ordered by updated_at descending.
    fn list_by_board(&self, board_id: i64) -> Result<Vec<Self::Thread>>;

    /// List threads in a board with pagination.
    fn list_by_board_paginated(
        &self,
        board_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Self::Thread>>;

    /// List threads by author.
    fn list_by_author(&self, author_id: i64) -> Result<Vec<Self::Thread>>;

    /// Count threads in a board.
    fn count_by_board(&self, board_id: i64) -> Result<i64>;

    /// Touch a thread and increment post count.
    fn touch_and_increment(&self, id: i64) -> Result<Option<Self::Thread>>;

    /// Decrement post count when a post is deleted.
    fn decrement_post_count(&self, id: i64) -> Result<Option<Self::Thread>>;
}

// ============================================================================
// Post Repository Trait
// ============================================================================

/// Trait for post repository operations.
///
/// This trait defines the interface for post CRUD operations.
/// Implementations can use different database backends.
pub trait PostRepositoryTrait {
    /// The post type used by this implementation.
    type Post;
    /// The new thread post type used by this implementation.
    type NewThreadPost;
    /// The new flat post type used by this implementation.
    type NewFlatPost;
    /// The post update type used by this implementation.
    type PostUpdate;

    /// Create a new post in a thread.
    fn create_thread_post(&self, new_post: &Self::NewThreadPost) -> Result<Self::Post>;

    /// Create a new post in a flat board.
    fn create_flat_post(&self, new_post: &Self::NewFlatPost) -> Result<Self::Post>;

    /// Get a post by ID.
    fn get_by_id(&self, id: i64) -> Result<Option<Self::Post>>;

    /// Update a post by ID.
    fn update(&self, id: i64, update: &Self::PostUpdate) -> Result<Option<Self::Post>>;

    /// Delete a post by ID.
    fn delete(&self, id: i64) -> Result<bool>;

    /// List posts in a thread, ordered by created_at ascending.
    fn list_by_thread(&self, thread_id: i64) -> Result<Vec<Self::Post>>;

    /// List posts in a thread with pagination.
    fn list_by_thread_paginated(
        &self,
        thread_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Self::Post>>;

    /// List posts in a flat board, ordered by created_at descending.
    fn list_by_flat_board(&self, board_id: i64) -> Result<Vec<Self::Post>>;

    /// List posts in a flat board with pagination.
    fn list_by_flat_board_paginated(
        &self,
        board_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Self::Post>>;

    /// List posts by author.
    fn list_by_author(&self, author_id: i64) -> Result<Vec<Self::Post>>;

    /// Count posts in a thread.
    fn count_by_thread(&self, thread_id: i64) -> Result<i64>;

    /// Count posts in a flat board.
    fn count_by_flat_board(&self, board_id: i64) -> Result<i64>;

    /// Count all posts in a board (both flat and thread posts).
    fn count_by_board(&self, board_id: i64) -> Result<i64>;

    /// Get the latest post in a thread.
    fn get_latest_in_thread(&self, thread_id: i64) -> Result<Option<Self::Post>>;
}

// ============================================================================
// Mail Repository Trait
// ============================================================================

/// Trait for mail repository operations.
///
/// This trait defines the interface for mail CRUD operations.
/// Implementations can use different database backends.
pub trait MailRepositoryTrait {
    /// The mail type used by this implementation.
    type Mail;
    /// The new mail type used by this implementation.
    type NewMail;
    /// The mail update type used by this implementation.
    type MailUpdate;

    /// Create a new mail.
    fn create(&self, mail: &Self::NewMail) -> Result<Self::Mail>;

    /// Get a mail by ID.
    fn get_by_id(&self, id: i64) -> Result<Option<Self::Mail>>;

    /// List inbox mails for a user (received mails, not deleted by recipient).
    fn list_inbox(&self, user_id: i64) -> Result<Vec<Self::Mail>>;

    /// List sent mails for a user (not deleted by sender).
    fn list_sent(&self, user_id: i64) -> Result<Vec<Self::Mail>>;

    /// Count unread mails for a user.
    fn count_unread(&self, user_id: i64) -> Result<i64>;

    /// Update a mail.
    fn update(&self, id: i64, update: &Self::MailUpdate) -> Result<bool>;

    /// Mark a mail as read.
    fn mark_as_read(&self, id: i64) -> Result<bool>;

    /// Delete a mail by user (logical deletion).
    fn delete_by_user(&self, id: i64, user_id: i64) -> Result<bool>;

    /// Physically delete a mail.
    fn purge(&self, id: i64) -> Result<bool>;

    /// Purge all mails deleted by both sender and recipient.
    fn purge_all_deleted(&self) -> Result<usize>;

    /// Count total mails.
    fn count(&self) -> Result<i64>;
}

// ============================================================================
// Script Repository Trait
// ============================================================================

/// Trait for script repository operations.
///
/// This trait defines the interface for script CRUD operations.
/// Implementations can use different database backends.
pub trait ScriptRepositoryTrait {
    /// The script type used by this implementation.
    type Script;

    /// List all enabled scripts accessible by the given role.
    fn list(&self, user_role: i32) -> Result<Vec<Self::Script>>;

    /// List all scripts (for admin).
    fn list_all(&self) -> Result<Vec<Self::Script>>;

    /// Get a script by ID.
    fn get_by_id(&self, id: i64) -> Result<Option<Self::Script>>;

    /// Get a script by slug.
    fn get_by_slug(&self, slug: &str) -> Result<Option<Self::Script>>;

    /// Get a script by file path.
    fn get_by_file_path(&self, file_path: &str) -> Result<Option<Self::Script>>;

    /// Insert or update a script (upsert).
    fn upsert(&self, script: &Self::Script) -> Result<Self::Script>;

    /// Update the enabled status of a script.
    fn update_enabled(&self, id: i64, enabled: bool) -> Result<()>;

    /// Delete a script by ID.
    fn delete(&self, id: i64) -> Result<()>;

    /// Delete a script by file path.
    fn delete_by_file_path(&self, file_path: &str) -> Result<()>;

    /// List all file paths in the database (for sync).
    fn list_all_file_paths(&self) -> Result<Vec<String>>;
}

// ============================================================================
// RSS Feed Repository Trait
// ============================================================================

/// Trait for RSS feed repository operations.
///
/// This trait defines the interface for RSS feed CRUD operations.
/// Implementations can use different database backends.
pub trait RssFeedRepositoryTrait {
    /// The RSS feed type used by this implementation.
    type RssFeed;
    /// The new RSS feed type used by this implementation.
    type NewRssFeed;
    /// The RSS feed update type used by this implementation.
    type RssFeedUpdate;
    /// The RSS feed with unread count type used by this implementation.
    type RssFeedWithUnread;

    /// Create a new feed.
    fn create(&self, feed: &Self::NewRssFeed) -> Result<Self::RssFeed>;

    /// Get a feed by ID.
    fn get_by_id(&self, id: i64) -> Result<Option<Self::RssFeed>>;

    /// Get a feed by URL.
    fn get_by_url(&self, url: &str) -> Result<Option<Self::RssFeed>>;

    /// Get a feed by URL for a specific user.
    fn get_by_user_url(&self, user_id: i64, url: &str) -> Result<Option<Self::RssFeed>>;

    /// List all active feeds.
    fn list_active(&self) -> Result<Vec<Self::RssFeed>>;

    /// List active feeds for a specific user.
    fn list_active_by_user(&self, user_id: i64) -> Result<Vec<Self::RssFeed>>;

    /// List all feeds (including inactive).
    fn list_all(&self) -> Result<Vec<Self::RssFeed>>;

    /// List feeds that are due for fetching.
    fn list_due_for_fetch(&self) -> Result<Vec<Self::RssFeed>>;

    /// List active feeds with unread counts for a user.
    fn list_with_unread(&self, user_id: Option<i64>) -> Result<Vec<Self::RssFeedWithUnread>>;

    /// Update a feed.
    fn update(&self, id: i64, update: &Self::RssFeedUpdate) -> Result<bool>;

    /// Update last fetched timestamp.
    fn update_last_fetched(&self, id: i64) -> Result<bool>;

    /// Update last item timestamp.
    fn update_last_item_at(&self, id: i64, last_item_at: DateTime<Utc>) -> Result<bool>;

    /// Increment error count and set error message.
    fn increment_error(&self, id: i64, error: &str) -> Result<bool>;

    /// Clear error count.
    fn clear_error(&self, id: i64) -> Result<bool>;

    /// Disable feeds that have exceeded the error threshold.
    fn disable_failed_feeds(&self, max_errors: i32) -> Result<usize>;

    /// Delete a feed.
    fn delete(&self, id: i64) -> Result<bool>;

    /// Count all feeds.
    fn count(&self) -> Result<i64>;
}

// ============================================================================
// RSS Item Repository Trait
// ============================================================================

/// Trait for RSS item repository operations.
///
/// This trait defines the interface for RSS item CRUD operations.
/// Implementations can use different database backends.
pub trait RssItemRepositoryTrait {
    /// The RSS item type used by this implementation.
    type RssItem;
    /// The new RSS item type used by this implementation.
    type NewRssItem;

    /// Create a new item, ignoring if duplicate (same feed_id + guid).
    fn create_or_ignore(&self, item: &Self::NewRssItem) -> Result<Option<i64>>;

    /// Get an item by ID.
    fn get_by_id(&self, id: i64) -> Result<Option<Self::RssItem>>;

    /// Get an item by feed ID and guid.
    fn get_by_guid(&self, feed_id: i64, guid: &str) -> Result<Option<Self::RssItem>>;

    /// List items for a feed (newest first).
    fn list_by_feed(&self, feed_id: i64, limit: usize, offset: usize)
        -> Result<Vec<Self::RssItem>>;

    /// Count items for a feed.
    fn count_by_feed(&self, feed_id: i64) -> Result<i64>;

    /// Count unread items for a user and feed.
    fn count_unread(&self, feed_id: i64, user_id: i64) -> Result<i64>;

    /// Get the newest item ID for a feed.
    fn get_newest_item_id(&self, feed_id: i64) -> Result<Option<i64>>;

    /// Delete old items for a feed, keeping only the most recent.
    fn prune_old_items(&self, feed_id: i64) -> Result<usize>;

    /// Delete all items for a feed.
    fn delete_by_feed(&self, feed_id: i64) -> Result<usize>;
}

// ============================================================================
// RSS Read Position Repository Trait
// ============================================================================

/// Trait for RSS read position repository operations.
///
/// This trait defines the interface for RSS read position CRUD operations.
/// Implementations can use different database backends.
pub trait RssReadPositionRepositoryTrait {
    /// The RSS read position type used by this implementation.
    type RssReadPosition;

    /// Get read position for a user and feed.
    fn get(&self, user_id: i64, feed_id: i64) -> Result<Option<Self::RssReadPosition>>;

    /// Update or insert read position.
    fn upsert(&self, user_id: i64, feed_id: i64, last_read_item_id: i64) -> Result<()>;

    /// Mark all items as read (set to newest item ID).
    fn mark_all_as_read(&self, user_id: i64, feed_id: i64) -> Result<bool>;

    /// Delete read position for a user and feed.
    fn delete(&self, user_id: i64, feed_id: i64) -> Result<bool>;

    /// Delete all read positions for a user.
    fn delete_by_user(&self, user_id: i64) -> Result<usize>;
}

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
