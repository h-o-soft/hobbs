//! Unread management for HOBBS.
//!
//! This module provides functionality to track and manage unread posts
//! for each user per board.

use rusqlite::{params, Row};

use crate::db::{Database, Role};
use crate::Result;

use super::Post;

/// Unread post with board information for cross-board reading.
#[derive(Debug, Clone)]
pub struct UnreadPostWithBoard {
    /// The post.
    pub post: Post,
    /// The board name.
    pub board_name: String,
}

/// Read position tracking for a user on a board.
#[derive(Debug, Clone)]
pub struct ReadPosition {
    /// Unique ID.
    pub id: i64,
    /// User ID.
    pub user_id: i64,
    /// Board ID.
    pub board_id: i64,
    /// Last read post ID.
    pub last_read_post_id: i64,
    /// Last read timestamp.
    pub last_read_at: String,
}

/// Repository for unread management operations.
pub struct UnreadRepository<'a> {
    db: &'a Database,
}

impl<'a> UnreadRepository<'a> {
    /// Create a new UnreadRepository with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get the read position for a user on a board.
    pub fn get_read_position(&self, user_id: i64, board_id: i64) -> Result<Option<ReadPosition>> {
        let result = self.db.conn().query_row(
            "SELECT id, user_id, board_id, last_read_post_id, last_read_at
             FROM read_positions WHERE user_id = ? AND board_id = ?",
            params![user_id, board_id],
            Self::row_to_read_position,
        );

        match result {
            Ok(pos) => Ok(Some(pos)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Mark a board as read up to a specific post.
    ///
    /// This updates the read position for the user on the board.
    /// If no read position exists, one is created.
    pub fn mark_as_read(&self, user_id: i64, board_id: i64, post_id: i64) -> Result<()> {
        self.db.conn().execute(
            "INSERT INTO read_positions (user_id, board_id, last_read_post_id, last_read_at)
             VALUES (?, ?, ?, datetime('now'))
             ON CONFLICT(user_id, board_id) DO UPDATE SET
                 last_read_post_id = excluded.last_read_post_id,
                 last_read_at = datetime('now')",
            params![user_id, board_id, post_id],
        )?;
        Ok(())
    }

    /// Get unread count for a user on a board.
    ///
    /// Returns the number of posts with ID greater than the last read post ID.
    /// If the user has no read position for this board, returns the total post count.
    pub fn get_unread_count(&self, user_id: i64, board_id: i64) -> Result<i64> {
        let read_position = self.get_read_position(user_id, board_id)?;

        let count: i64 = match read_position {
            Some(pos) => self.db.conn().query_row(
                "SELECT COUNT(*) FROM posts WHERE board_id = ? AND id > ?",
                params![board_id, pos.last_read_post_id],
                |row| row.get(0),
            )?,
            None => self.db.conn().query_row(
                "SELECT COUNT(*) FROM posts WHERE board_id = ?",
                [board_id],
                |row| row.get(0),
            )?,
        };

        Ok(count)
    }

    /// Get unread counts for all boards for a user.
    ///
    /// Returns a list of (board_id, unread_count) tuples.
    pub fn get_all_unread_counts(&self, user_id: i64) -> Result<Vec<(i64, i64)>> {
        // Get all active boards
        let mut stmt = self.db.conn().prepare(
            "SELECT b.id,
                    (SELECT COUNT(*) FROM posts p WHERE p.board_id = b.id
                     AND p.id > COALESCE(
                         (SELECT last_read_post_id FROM read_positions
                          WHERE user_id = ? AND board_id = b.id),
                         0
                     )) as unread_count
             FROM boards b
             WHERE b.is_active = 1
             ORDER BY b.sort_order, b.id",
        )?;

        let counts = stmt
            .query_map([user_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(counts)
    }

    /// Get unread posts for a user on a board.
    ///
    /// Returns posts with ID greater than the last read post ID.
    /// If the user has no read position, returns all posts.
    pub fn get_unread_posts(&self, user_id: i64, board_id: i64) -> Result<Vec<Post>> {
        let read_position = self.get_read_position(user_id, board_id)?;

        let mut stmt = match read_position {
            Some(pos) => {
                let mut stmt = self.db.conn().prepare(
                    "SELECT id, board_id, thread_id, author_id, title, body, created_at
                     FROM posts WHERE board_id = ? AND id > ?
                     ORDER BY id ASC",
                )?;
                let posts = stmt
                    .query_map(params![board_id, pos.last_read_post_id], Self::row_to_post)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                return Ok(posts);
            }
            None => self.db.conn().prepare(
                "SELECT id, board_id, thread_id, author_id, title, body, created_at
                 FROM posts WHERE board_id = ?
                 ORDER BY id ASC",
            )?,
        };

        let posts = stmt
            .query_map([board_id], Self::row_to_post)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(posts)
    }

    /// Get unread posts with pagination.
    pub fn get_unread_posts_paginated(
        &self,
        user_id: i64,
        board_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Post>> {
        let read_position = self.get_read_position(user_id, board_id)?;

        match read_position {
            Some(pos) => {
                let mut stmt = self.db.conn().prepare(
                    "SELECT id, board_id, thread_id, author_id, title, body, created_at
                     FROM posts WHERE board_id = ? AND id > ?
                     ORDER BY id ASC LIMIT ? OFFSET ?",
                )?;
                let posts = stmt
                    .query_map(
                        params![board_id, pos.last_read_post_id, limit, offset],
                        Self::row_to_post,
                    )?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(posts)
            }
            None => {
                let mut stmt = self.db.conn().prepare(
                    "SELECT id, board_id, thread_id, author_id, title, body, created_at
                     FROM posts WHERE board_id = ?
                     ORDER BY id ASC LIMIT ? OFFSET ?",
                )?;
                let posts = stmt
                    .query_map(params![board_id, limit, offset], Self::row_to_post)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(posts)
            }
        }
    }

    /// Mark all posts in a board as read for a user.
    ///
    /// This sets the read position to the latest post ID.
    pub fn mark_all_as_read(&self, user_id: i64, board_id: i64) -> Result<bool> {
        // Get the latest post ID in the board
        let latest_post_id: Option<i64> = self.db.conn().query_row(
            "SELECT MAX(id) FROM posts WHERE board_id = ?",
            [board_id],
            |row| row.get(0),
        )?;

        match latest_post_id {
            Some(post_id) => {
                self.mark_as_read(user_id, board_id, post_id)?;
                Ok(true)
            }
            None => Ok(false), // No posts in board
        }
    }

    /// Delete read position for a user on a board.
    ///
    /// This effectively marks all posts as unread.
    pub fn delete_read_position(&self, user_id: i64, board_id: i64) -> Result<bool> {
        let affected = self.db.conn().execute(
            "DELETE FROM read_positions WHERE user_id = ? AND board_id = ?",
            params![user_id, board_id],
        )?;
        Ok(affected > 0)
    }

    /// Delete all read positions for a user.
    pub fn delete_all_read_positions(&self, user_id: i64) -> Result<i64> {
        let affected = self
            .db
            .conn()
            .execute("DELETE FROM read_positions WHERE user_id = ?", [user_id])?;
        Ok(affected as i64)
    }

    /// Get thread IDs that have unread posts.
    ///
    /// Returns a set of thread IDs that have at least one post newer than
    /// the user's last read position.
    pub fn get_unread_thread_ids(
        &self,
        user_id: i64,
        board_id: i64,
        thread_ids: &[i64],
    ) -> Result<std::collections::HashSet<i64>> {
        if thread_ids.is_empty() {
            return Ok(std::collections::HashSet::new());
        }

        let read_position = self.get_read_position(user_id, board_id)?;
        let last_read_id = read_position.map(|p| p.last_read_post_id).unwrap_or(0);

        // Build query with placeholders for thread IDs
        let placeholders: Vec<String> = thread_ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT DISTINCT thread_id FROM posts
             WHERE thread_id IN ({}) AND id > ?",
            placeholders.join(",")
        );

        let mut stmt = self.db.conn().prepare(&query)?;

        // Build params: thread_ids + last_read_id
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = thread_ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
            .collect();
        params.push(Box::new(last_read_id));

        let unread_ids = stmt
            .query_map(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
                row.get::<_, i64>(0)
            })?
            .collect::<rusqlite::Result<std::collections::HashSet<_>>>()?;

        Ok(unread_ids)
    }

    /// Get the last read post ID for a user on a board.
    ///
    /// Returns 0 if no read position exists (meaning all posts are unread).
    pub fn get_last_read_post_id(&self, user_id: i64, board_id: i64) -> Result<i64> {
        let read_position = self.get_read_position(user_id, board_id)?;
        Ok(read_position.map(|p| p.last_read_post_id).unwrap_or(0))
    }

    /// Get all unread posts across all boards for a user.
    ///
    /// Returns posts from all accessible boards that the user hasn't read,
    /// ordered by board sort order, then by post ID.
    /// Each post includes the board name for display purposes.
    pub fn get_all_unread_posts(&self, user_id: i64, user_role: Role) -> Result<Vec<UnreadPostWithBoard>> {
        // Get all active boards with their read positions
        let mut stmt = self.db.conn().prepare(
            "SELECT b.id, b.name, b.min_read_role,
                    COALESCE(
                        (SELECT last_read_post_id FROM read_positions
                         WHERE user_id = ? AND board_id = b.id),
                        0
                    ) as last_read
             FROM boards b
             WHERE b.is_active = 1
             ORDER BY b.sort_order, b.id",
        )?;

        let boards: Vec<(i64, String, String, i64)> = stmt
            .query_map(params![user_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut all_unread = Vec::new();

        // Get unread posts for each accessible board
        for (board_id, board_name, min_read_role_str, last_read_post_id) in boards {
            // Check if user can access this board
            let min_read_role: Role = min_read_role_str.parse().unwrap_or(Role::Guest);
            if !user_role.can_access(min_read_role) {
                continue;
            }

            let mut post_stmt = self.db.conn().prepare(
                "SELECT id, board_id, thread_id, author_id, title, body, created_at
                 FROM posts WHERE board_id = ? AND id > ?
                 ORDER BY id ASC",
            )?;

            let posts = post_stmt
                .query_map(params![board_id, last_read_post_id], Self::row_to_post)?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            for post in posts {
                all_unread.push(UnreadPostWithBoard {
                    post,
                    board_name: board_name.clone(),
                });
            }
        }

        Ok(all_unread)
    }

    /// Get total unread count across all boards for a user.
    pub fn get_total_unread_count(&self, user_id: i64, user_role: Role) -> Result<i64> {
        // Get all active boards with their read positions
        let mut stmt = self.db.conn().prepare(
            "SELECT b.id, b.min_read_role,
                    COALESCE(
                        (SELECT last_read_post_id FROM read_positions
                         WHERE user_id = ? AND board_id = b.id),
                        0
                    ) as last_read
             FROM boards b
             WHERE b.is_active = 1",
        )?;

        let boards: Vec<(i64, String, i64)> = stmt
            .query_map(params![user_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut total_count: i64 = 0;

        for (board_id, min_read_role_str, last_read_post_id) in boards {
            // Check if user can access this board
            let min_read_role: Role = min_read_role_str.parse().unwrap_or(Role::Guest);
            if !user_role.can_access(min_read_role) {
                continue;
            }

            let count: i64 = self.db.conn().query_row(
                "SELECT COUNT(*) FROM posts WHERE board_id = ? AND id > ?",
                params![board_id, last_read_post_id],
                |row| row.get(0),
            )?;

            total_count += count;
        }

        Ok(total_count)
    }

    /// Convert a database row to a ReadPosition struct.
    fn row_to_read_position(row: &Row<'_>) -> rusqlite::Result<ReadPosition> {
        Ok(ReadPosition {
            id: row.get(0)?,
            user_id: row.get(1)?,
            board_id: row.get(2)?,
            last_read_post_id: row.get(3)?,
            last_read_at: row.get(4)?,
        })
    }

    /// Convert a database row to a Post struct.
    fn row_to_post(row: &Row<'_>) -> rusqlite::Result<Post> {
        Ok(Post {
            id: row.get(0)?,
            board_id: row.get(1)?,
            thread_id: row.get(2)?,
            author_id: row.get(3)?,
            title: row.get(4)?,
            body: row.get(5)?,
            created_at: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardRepository, NewBoard, NewFlatPost, NewThread, NewThreadPost, PostRepository, ThreadRepository};
    use crate::db::{NewUser, UserRepository};

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db);
        let user = repo
            .create(&NewUser::new("testuser", "hash", "Test User"))
            .unwrap();
        user.id
    }

    fn create_test_board(db: &Database) -> i64 {
        let repo = BoardRepository::new(db);
        let board = repo.create(&NewBoard::new("test-board")).unwrap();
        board.id
    }

    fn create_test_post(db: &Database, board_id: i64, author_id: i64) -> i64 {
        let repo = PostRepository::new(db);
        let post = repo
            .create_flat_post(&NewFlatPost::new(board_id, author_id, "Title", "Body"))
            .unwrap();
        post.id
    }

    #[test]
    fn test_mark_as_read() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);
        let post_id = create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board_id, post_id).unwrap();

        let position = repo.get_read_position(user_id, board_id).unwrap().unwrap();
        assert_eq!(position.user_id, user_id);
        assert_eq!(position.board_id, board_id);
        assert_eq!(position.last_read_post_id, post_id);
    }

    #[test]
    fn test_mark_as_read_update() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);
        let post1_id = create_test_post(&db, board_id, user_id);
        let post2_id = create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);

        // Mark first post as read
        repo.mark_as_read(user_id, board_id, post1_id).unwrap();
        let pos1 = repo.get_read_position(user_id, board_id).unwrap().unwrap();
        assert_eq!(pos1.last_read_post_id, post1_id);

        // Update to second post
        repo.mark_as_read(user_id, board_id, post2_id).unwrap();
        let pos2 = repo.get_read_position(user_id, board_id).unwrap().unwrap();
        assert_eq!(pos2.last_read_post_id, post2_id);
    }

    #[test]
    fn test_get_read_position_not_found() {
        let db = setup_db();
        let repo = UnreadRepository::new(&db);

        let position = repo.get_read_position(999, 999).unwrap();
        assert!(position.is_none());
    }

    #[test]
    fn test_get_unread_count_no_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        // Create some posts
        create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);
        let count = repo.get_unread_count(user_id, board_id).unwrap();

        // All posts should be unread
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_unread_count_with_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        // Create posts
        let post1_id = create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);

        // Mark first post as read
        repo.mark_as_read(user_id, board_id, post1_id).unwrap();

        let count = repo.get_unread_count(user_id, board_id).unwrap();
        assert_eq!(count, 2); // 2 posts after the read position
    }

    #[test]
    fn test_get_unread_count_all_read() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);
        let post3_id = create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board_id, post3_id).unwrap();

        let count = repo.get_unread_count(user_id, board_id).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_all_unread_counts() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        // Create two boards
        let board_repo = BoardRepository::new(&db);
        let board1 = board_repo.create(&NewBoard::new("board1")).unwrap();
        let board2 = board_repo.create(&NewBoard::new("board2")).unwrap();

        // Create posts in each board
        create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board2.id, user_id);
        create_test_post(&db, board2.id, user_id);
        create_test_post(&db, board2.id, user_id);

        let repo = UnreadRepository::new(&db);
        let counts = repo.get_all_unread_counts(user_id).unwrap();

        assert_eq!(counts.len(), 2);
        // Find the counts for each board
        let board1_count = counts.iter().find(|(id, _)| *id == board1.id).unwrap().1;
        let board2_count = counts.iter().find(|(id, _)| *id == board2.id).unwrap().1;
        assert_eq!(board1_count, 2);
        assert_eq!(board2_count, 3);
    }

    #[test]
    fn test_get_unread_posts_no_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);
        let posts = repo.get_unread_posts(user_id, board_id).unwrap();

        assert_eq!(posts.len(), 3);
    }

    #[test]
    fn test_get_unread_posts_with_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        let post1_id = create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board_id, post1_id).unwrap();

        let posts = repo.get_unread_posts(user_id, board_id).unwrap();
        assert_eq!(posts.len(), 2);
        // All returned posts should have ID > post1_id
        for post in &posts {
            assert!(post.id > post1_id);
        }
    }

    #[test]
    fn test_get_unread_posts_paginated() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        for _ in 0..5 {
            create_test_post(&db, board_id, user_id);
        }

        let repo = UnreadRepository::new(&db);

        // Get first page
        let page1 = repo
            .get_unread_posts_paginated(user_id, board_id, 0, 2)
            .unwrap();
        assert_eq!(page1.len(), 2);

        // Get second page
        let page2 = repo
            .get_unread_posts_paginated(user_id, board_id, 2, 2)
            .unwrap();
        assert_eq!(page2.len(), 2);

        // Get third page
        let page3 = repo
            .get_unread_posts_paginated(user_id, board_id, 4, 2)
            .unwrap();
        assert_eq!(page3.len(), 1);
    }

    #[test]
    fn test_mark_all_as_read() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        create_test_post(&db, board_id, user_id);
        create_test_post(&db, board_id, user_id);
        let post3_id = create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);

        // Initially 3 unread
        assert_eq!(repo.get_unread_count(user_id, board_id).unwrap(), 3);

        // Mark all as read
        let result = repo.mark_all_as_read(user_id, board_id).unwrap();
        assert!(result);

        // Now 0 unread
        assert_eq!(repo.get_unread_count(user_id, board_id).unwrap(), 0);

        // Check read position
        let pos = repo.get_read_position(user_id, board_id).unwrap().unwrap();
        assert_eq!(pos.last_read_post_id, post3_id);
    }

    #[test]
    fn test_mark_all_as_read_empty_board() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        let repo = UnreadRepository::new(&db);
        let result = repo.mark_all_as_read(user_id, board_id).unwrap();

        assert!(!result); // No posts to mark
    }

    #[test]
    fn test_delete_read_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);
        let post_id = create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board_id, post_id).unwrap();

        // Verify position exists
        assert!(repo.get_read_position(user_id, board_id).unwrap().is_some());

        // Delete position
        let deleted = repo.delete_read_position(user_id, board_id).unwrap();
        assert!(deleted);

        // Verify position is gone
        assert!(repo.get_read_position(user_id, board_id).unwrap().is_none());
    }

    #[test]
    fn test_delete_read_position_not_found() {
        let db = setup_db();
        let repo = UnreadRepository::new(&db);

        let deleted = repo.delete_read_position(999, 999).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_delete_all_read_positions() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let board_repo = BoardRepository::new(&db);
        let board1 = board_repo.create(&NewBoard::new("board1")).unwrap();
        let board2 = board_repo.create(&NewBoard::new("board2")).unwrap();

        let post1_id = create_test_post(&db, board1.id, user_id);
        let post2_id = create_test_post(&db, board2.id, user_id);

        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board1.id, post1_id).unwrap();
        repo.mark_as_read(user_id, board2.id, post2_id).unwrap();

        // Delete all positions
        let deleted = repo.delete_all_read_positions(user_id).unwrap();
        assert_eq!(deleted, 2);

        // Verify all positions are gone
        assert!(repo
            .get_read_position(user_id, board1.id)
            .unwrap()
            .is_none());
        assert!(repo
            .get_read_position(user_id, board2.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_get_last_read_post_id_no_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        let repo = UnreadRepository::new(&db);
        let last_read = repo.get_last_read_post_id(user_id, board_id).unwrap();

        // Should return 0 when no position exists
        assert_eq!(last_read, 0);
    }

    #[test]
    fn test_get_last_read_post_id_with_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);
        let post_id = create_test_post(&db, board_id, user_id);

        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board_id, post_id).unwrap();

        let last_read = repo.get_last_read_post_id(user_id, board_id).unwrap();
        assert_eq!(last_read, post_id);
    }

    fn create_test_thread(db: &Database, board_id: i64, author_id: i64) -> i64 {
        let thread_repo = ThreadRepository::new(db);
        let thread = thread_repo
            .create(&NewThread::new(board_id, "Test Thread", author_id))
            .unwrap();
        thread.id
    }

    fn create_test_thread_post(db: &Database, board_id: i64, thread_id: i64, author_id: i64) -> i64 {
        let post_repo = PostRepository::new(db);
        let post = post_repo
            .create_thread_post(&NewThreadPost::new(board_id, thread_id, author_id, "Body"))
            .unwrap();
        post.id
    }

    #[test]
    fn test_get_unread_thread_ids_empty() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        let repo = UnreadRepository::new(&db);
        let unread_ids = repo.get_unread_thread_ids(user_id, board_id, &[]).unwrap();

        assert!(unread_ids.is_empty());
    }

    #[test]
    fn test_get_unread_thread_ids_no_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        // Create threads with posts
        let thread1_id = create_test_thread(&db, board_id, user_id);
        let thread2_id = create_test_thread(&db, board_id, user_id);
        create_test_thread_post(&db, board_id, thread1_id, user_id);
        create_test_thread_post(&db, board_id, thread2_id, user_id);

        let repo = UnreadRepository::new(&db);
        let unread_ids = repo
            .get_unread_thread_ids(user_id, board_id, &[thread1_id, thread2_id])
            .unwrap();

        // All threads should have unread posts (no read position)
        assert_eq!(unread_ids.len(), 2);
        assert!(unread_ids.contains(&thread1_id));
        assert!(unread_ids.contains(&thread2_id));
    }

    #[test]
    fn test_get_unread_thread_ids_with_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        // Create threads with posts
        let thread1_id = create_test_thread(&db, board_id, user_id);
        let thread2_id = create_test_thread(&db, board_id, user_id);
        let post1_id = create_test_thread_post(&db, board_id, thread1_id, user_id);
        let _post2_id = create_test_thread_post(&db, board_id, thread2_id, user_id);

        // Mark post1 as read (should mark thread1 as read)
        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board_id, post1_id).unwrap();

        let unread_ids = repo
            .get_unread_thread_ids(user_id, board_id, &[thread1_id, thread2_id])
            .unwrap();

        // Only thread2 should have unread posts (post2 > post1)
        assert_eq!(unread_ids.len(), 1);
        assert!(unread_ids.contains(&thread2_id));
        assert!(!unread_ids.contains(&thread1_id));
    }

    #[test]
    fn test_get_unread_thread_ids_all_read() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        // Create threads with posts
        let thread1_id = create_test_thread(&db, board_id, user_id);
        let thread2_id = create_test_thread(&db, board_id, user_id);
        create_test_thread_post(&db, board_id, thread1_id, user_id);
        let post2_id = create_test_thread_post(&db, board_id, thread2_id, user_id);

        // Mark last post as read (should mark all as read)
        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board_id, post2_id).unwrap();

        let unread_ids = repo
            .get_unread_thread_ids(user_id, board_id, &[thread1_id, thread2_id])
            .unwrap();

        // No threads should have unread posts
        assert!(unread_ids.is_empty());
    }

    #[test]
    fn test_get_unread_thread_ids_new_post_in_read_thread() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let board_id = create_test_board(&db);

        // Create thread with post
        let thread_id = create_test_thread(&db, board_id, user_id);
        let post1_id = create_test_thread_post(&db, board_id, thread_id, user_id);

        // Mark as read
        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board_id, post1_id).unwrap();

        // Add new post to thread
        create_test_thread_post(&db, board_id, thread_id, user_id);

        let unread_ids = repo.get_unread_thread_ids(user_id, board_id, &[thread_id]).unwrap();

        // Thread should now have unread posts
        assert_eq!(unread_ids.len(), 1);
        assert!(unread_ids.contains(&thread_id));
    }

    #[test]
    fn test_get_all_unread_posts_empty() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let repo = UnreadRepository::new(&db);
        let unread_posts = repo.get_all_unread_posts(user_id, Role::Member).unwrap();

        assert!(unread_posts.is_empty());
    }

    #[test]
    fn test_get_all_unread_posts_no_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        // Create two boards with posts
        let board_repo = BoardRepository::new(&db);
        let board1 = board_repo.create(&NewBoard::new("board1")).unwrap();
        let board2 = board_repo.create(&NewBoard::new("board2")).unwrap();

        create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board2.id, user_id);

        let repo = UnreadRepository::new(&db);
        let unread_posts = repo.get_all_unread_posts(user_id, Role::Member).unwrap();

        // All posts should be unread
        assert_eq!(unread_posts.len(), 3);
    }

    #[test]
    fn test_get_all_unread_posts_with_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        // Create boards with posts
        let board_repo = BoardRepository::new(&db);
        let board1 = board_repo.create(&NewBoard::new("board1")).unwrap();
        let board2 = board_repo.create(&NewBoard::new("board2")).unwrap();

        let post1_id = create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board1.id, user_id); // post 2
        create_test_post(&db, board2.id, user_id); // post 3

        let repo = UnreadRepository::new(&db);
        // Mark post1 as read in board1
        repo.mark_as_read(user_id, board1.id, post1_id).unwrap();

        let unread_posts = repo.get_all_unread_posts(user_id, Role::Member).unwrap();

        // Should have 2 unread posts (post2 from board1, post3 from board2)
        assert_eq!(unread_posts.len(), 2);
    }

    #[test]
    fn test_get_all_unread_posts_includes_board_name() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let board_repo = BoardRepository::new(&db);
        let board = board_repo.create(&NewBoard::new("TestBoard")).unwrap();
        create_test_post(&db, board.id, user_id);

        let repo = UnreadRepository::new(&db);
        let unread_posts = repo.get_all_unread_posts(user_id, Role::Member).unwrap();

        assert_eq!(unread_posts.len(), 1);
        assert_eq!(unread_posts[0].board_name, "TestBoard");
    }

    #[test]
    fn test_get_total_unread_count_empty() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let repo = UnreadRepository::new(&db);
        let count = repo.get_total_unread_count(user_id, Role::Member).unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_total_unread_count_multiple_boards() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let board_repo = BoardRepository::new(&db);
        let board1 = board_repo.create(&NewBoard::new("board1")).unwrap();
        let board2 = board_repo.create(&NewBoard::new("board2")).unwrap();

        create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board2.id, user_id);
        create_test_post(&db, board2.id, user_id);
        create_test_post(&db, board2.id, user_id);

        let repo = UnreadRepository::new(&db);
        let count = repo.get_total_unread_count(user_id, Role::Member).unwrap();

        assert_eq!(count, 5);
    }

    #[test]
    fn test_get_total_unread_count_with_position() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let board_repo = BoardRepository::new(&db);
        let board1 = board_repo.create(&NewBoard::new("board1")).unwrap();
        let board2 = board_repo.create(&NewBoard::new("board2")).unwrap();

        let post1_id = create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board1.id, user_id);
        create_test_post(&db, board2.id, user_id);

        let repo = UnreadRepository::new(&db);
        repo.mark_as_read(user_id, board1.id, post1_id).unwrap();

        let count = repo.get_total_unread_count(user_id, Role::Member).unwrap();

        // 1 unread in board1 (post2) + 1 unread in board2 (post3)
        assert_eq!(count, 2);
    }
}
