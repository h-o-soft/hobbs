//! Thread repository for HOBBS.
//!
//! This module provides CRUD operations for threads in the database.

use rusqlite::{params, Row};

use super::thread::{NewThread, Thread, ThreadUpdate};
use crate::db::Database;
use crate::{HobbsError, Result};

/// Repository for thread CRUD operations.
pub struct ThreadRepository<'a> {
    db: &'a Database,
}

impl<'a> ThreadRepository<'a> {
    /// Create a new ThreadRepository with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new thread in the database.
    ///
    /// Returns the created thread with the assigned ID.
    pub fn create(&self, new_thread: &NewThread) -> Result<Thread> {
        self.db.conn().execute(
            "INSERT INTO threads (board_id, title, author_id) VALUES (?, ?, ?)",
            params![new_thread.board_id, &new_thread.title, new_thread.author_id,],
        )?;

        let id = self.db.conn().last_insert_rowid();
        self.get_by_id(id)?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))
    }

    /// Get a thread by ID.
    pub fn get_by_id(&self, id: i64) -> Result<Option<Thread>> {
        let result = self.db.conn().query_row(
            "SELECT id, board_id, title, author_id, post_count, created_at, updated_at
             FROM threads WHERE id = ?",
            [id],
            Self::row_to_thread,
        );

        match result {
            Ok(thread) => Ok(Some(thread)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update a thread by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated thread, or None if not found.
    pub fn update(&self, id: i64, update: &ThreadUpdate) -> Result<Option<Thread>> {
        if update.is_empty() {
            return self.get_by_id(id);
        }

        let mut fields = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref title) = update.title {
            fields.push("title = ?");
            values.push(Box::new(title.clone()));
        }
        if let Some(delta) = update.post_count_delta {
            fields.push("post_count = post_count + ?");
            values.push(Box::new(delta));
        }
        if update.touch {
            fields.push("updated_at = datetime('now')");
        }

        let sql = format!("UPDATE threads SET {} WHERE id = ?", fields.join(", "));
        values.push(Box::new(id));

        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
        let affected = self.db.conn().execute(&sql, params.as_slice())?;

        if affected == 0 {
            return Ok(None);
        }

        self.get_by_id(id)
    }

    /// Delete a thread by ID.
    ///
    /// Returns true if a thread was deleted, false if not found.
    /// Note: This will cascade delete all posts in the thread.
    pub fn delete(&self, id: i64) -> Result<bool> {
        let affected = self
            .db
            .conn()
            .execute("DELETE FROM threads WHERE id = ?", [id])?;
        Ok(affected > 0)
    }

    /// List threads in a board, ordered by updated_at descending.
    pub fn list_by_board(&self, board_id: i64) -> Result<Vec<Thread>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, board_id, title, author_id, post_count, created_at, updated_at
             FROM threads WHERE board_id = ? ORDER BY updated_at DESC, id DESC",
        )?;

        let threads = stmt
            .query_map([board_id], Self::row_to_thread)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(threads)
    }

    /// List threads in a board with pagination.
    pub fn list_by_board_paginated(
        &self,
        board_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Thread>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, board_id, title, author_id, post_count, created_at, updated_at
             FROM threads WHERE board_id = ? ORDER BY updated_at DESC, id DESC LIMIT ? OFFSET ?",
        )?;

        let threads = stmt
            .query_map([board_id, limit, offset], Self::row_to_thread)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(threads)
    }

    /// List threads by author.
    pub fn list_by_author(&self, author_id: i64) -> Result<Vec<Thread>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, board_id, title, author_id, post_count, created_at, updated_at
             FROM threads WHERE author_id = ? ORDER BY updated_at DESC",
        )?;

        let threads = stmt
            .query_map([author_id], Self::row_to_thread)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(threads)
    }

    /// Count threads in a board.
    pub fn count_by_board(&self, board_id: i64) -> Result<i64> {
        let count: i64 = self.db.conn().query_row(
            "SELECT COUNT(*) FROM threads WHERE board_id = ?",
            [board_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Touch a thread (update updated_at to now) and increment post count.
    ///
    /// This is a convenience method for when a new post is added to a thread.
    pub fn touch_and_increment(&self, id: i64) -> Result<Option<Thread>> {
        self.update(id, &ThreadUpdate::new().touch().increment_post_count())
    }

    /// Decrement post count when a post is deleted.
    pub fn decrement_post_count(&self, id: i64) -> Result<Option<Thread>> {
        self.update(id, &ThreadUpdate::new().decrement_post_count())
    }

    /// Convert a database row to a Thread struct.
    fn row_to_thread(row: &Row<'_>) -> rusqlite::Result<Thread> {
        Ok(Thread {
            id: row.get(0)?,
            board_id: row.get(1)?,
            title: row.get(2)?,
            author_id: row.get(3)?,
            post_count: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardRepository, NewBoard};
    use crate::db::{NewUser, UserRepository};

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_board(db: &Database) -> i64 {
        let repo = BoardRepository::new(db);
        let board = repo.create(&NewBoard::new("test-board")).unwrap();
        board.id
    }

    fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db);
        let user = repo
            .create(&NewUser::new("testuser", "hash", "Test User"))
            .unwrap();
        user.id
    }

    #[test]
    fn test_create_thread() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).unwrap();

        assert_eq!(thread.board_id, board_id);
        assert_eq!(thread.title, "Test Thread");
        assert_eq!(thread.author_id, author_id);
        assert_eq!(thread.post_count, 0);
    }

    #[test]
    fn test_get_by_id() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let created = repo.create(&new_thread).unwrap();

        let found = repo.get_by_id(created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Test Thread");

        let not_found = repo.get_by_id(999).unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_update_thread_title() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        let new_thread = NewThread::new(board_id, "Original Title", author_id);
        let thread = repo.create(&new_thread).unwrap();

        let update = ThreadUpdate::new().title("Updated Title");
        let updated = repo.update(thread.id, &update).unwrap().unwrap();

        assert_eq!(updated.title, "Updated Title");
    }

    #[test]
    fn test_update_empty() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).unwrap();

        let update = ThreadUpdate::new();
        let result = repo.update(thread.id, &update).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().title, "Test Thread");
    }

    #[test]
    fn test_update_nonexistent_thread() {
        let db = setup_db();
        let repo = ThreadRepository::new(&db);

        let update = ThreadUpdate::new().title("New Title");
        let result = repo.update(999, &update).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_touch_and_increment() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).unwrap();
        let original_updated_at = thread.updated_at.clone();
        assert_eq!(thread.post_count, 0);

        // Wait a tiny bit to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        let updated = repo.touch_and_increment(thread.id).unwrap().unwrap();
        assert_eq!(updated.post_count, 1);
        // updated_at should be different (or at least not less than original)
        assert!(updated.updated_at >= original_updated_at);
    }

    #[test]
    fn test_decrement_post_count() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).unwrap();

        // First increment
        repo.touch_and_increment(thread.id).unwrap();
        repo.touch_and_increment(thread.id).unwrap();

        let thread = repo.get_by_id(thread.id).unwrap().unwrap();
        assert_eq!(thread.post_count, 2);

        // Then decrement
        let updated = repo.decrement_post_count(thread.id).unwrap().unwrap();
        assert_eq!(updated.post_count, 1);
    }

    #[test]
    fn test_delete_thread() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).unwrap();

        let deleted = repo.delete(thread.id).unwrap();
        assert!(deleted);

        let found = repo.get_by_id(thread.id).unwrap();
        assert!(found.is_none());

        // Deleting again should return false
        let deleted_again = repo.delete(thread.id).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_list_by_board() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        // Create some threads
        repo.create(&NewThread::new(board_id, "Thread 1", author_id))
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 2", author_id))
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 3", author_id))
            .unwrap();

        let threads = repo.list_by_board(board_id).unwrap();
        assert_eq!(threads.len(), 3);
        // Should be ordered by updated_at DESC, so newest first
        assert_eq!(threads[0].title, "Thread 3");
    }

    #[test]
    fn test_list_by_board_paginated() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        // Create some threads
        for i in 1..=5 {
            repo.create(&NewThread::new(board_id, format!("Thread {i}"), author_id))
                .unwrap();
        }

        // Get first page
        let page1 = repo.list_by_board_paginated(board_id, 0, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].title, "Thread 5");
        assert_eq!(page1[1].title, "Thread 4");

        // Get second page
        let page2 = repo.list_by_board_paginated(board_id, 2, 2).unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].title, "Thread 3");
        assert_eq!(page2[1].title, "Thread 2");
    }

    #[test]
    fn test_list_by_author() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);

        // Create another user
        let user_repo = UserRepository::new(&db);
        let other_author = user_repo
            .create(&NewUser::new("other", "hash", "Other"))
            .unwrap();

        let repo = ThreadRepository::new(&db);

        // Create threads by different authors
        repo.create(&NewThread::new(board_id, "Thread 1", author_id))
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 2", other_author.id))
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 3", author_id))
            .unwrap();

        let threads = repo.list_by_author(author_id).unwrap();
        assert_eq!(threads.len(), 2);
    }

    #[test]
    fn test_count_by_board() {
        let db = setup_db();
        let board_id = create_test_board(&db);
        let author_id = create_test_user(&db);
        let repo = ThreadRepository::new(&db);

        assert_eq!(repo.count_by_board(board_id).unwrap(), 0);

        repo.create(&NewThread::new(board_id, "Thread 1", author_id))
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 2", author_id))
            .unwrap();

        assert_eq!(repo.count_by_board(board_id).unwrap(), 2);
    }
}
