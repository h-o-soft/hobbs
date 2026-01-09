//! Thread repository for HOBBS.
//!
//! This module provides CRUD operations for threads in the database.

use sqlx::{FromRow, QueryBuilder};

use super::thread::{NewThread, ThreadUpdate};
use crate::db::DbPool;
use crate::{HobbsError, Result};

/// Thread entity representing a discussion thread in a board.
///
/// This struct is used for database queries with sqlx::FromRow.
#[derive(Debug, Clone, FromRow)]
pub struct ThreadRow {
    /// Unique thread ID.
    pub id: i64,
    /// ID of the board this thread belongs to.
    pub board_id: i64,
    /// Thread title.
    pub title: String,
    /// ID of the user who created the thread.
    pub author_id: i64,
    /// Number of posts in this thread.
    pub post_count: i32,
    /// Thread creation timestamp.
    pub created_at: String,
    /// Last update timestamp (when a new post was added).
    pub updated_at: String,
}

impl From<ThreadRow> for super::thread::Thread {
    fn from(row: ThreadRow) -> Self {
        super::thread::Thread {
            id: row.id,
            board_id: row.board_id,
            title: row.title,
            author_id: row.author_id,
            post_count: row.post_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Repository for thread CRUD operations.
pub struct ThreadRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> ThreadRepository<'a> {
    /// Create a new ThreadRepository with the given database pool reference.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new thread in the database.
    ///
    /// Returns the created thread with the assigned ID.
    #[cfg(feature = "sqlite")]
    pub async fn create(&self, new_thread: &NewThread) -> Result<super::thread::Thread> {
        let id: i64 =
            sqlx::query_scalar("INSERT INTO threads (board_id, title, author_id) VALUES (?, ?, ?) RETURNING id")
                .bind(new_thread.board_id)
                .bind(&new_thread.title)
                .bind(new_thread.author_id)
                .fetch_one(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))
    }

    /// Create a new thread in the database.
    ///
    /// Returns the created thread with the assigned ID.
    #[cfg(feature = "postgres")]
    pub async fn create(&self, new_thread: &NewThread) -> Result<super::thread::Thread> {
        let id: i64 =
            sqlx::query_scalar("INSERT INTO threads (board_id, title, author_id) VALUES ($1, $2, $3) RETURNING id")
                .bind(new_thread.board_id)
                .bind(&new_thread.title)
                .bind(new_thread.author_id)
                .fetch_one(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))
    }

    /// Get a thread by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<super::thread::Thread>> {
        let result = sqlx::query_as::<_, ThreadRow>(
            "SELECT id, board_id, title, author_id, post_count, created_at, updated_at
             FROM threads WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(Into::into))
    }

    /// Update a thread by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated thread, or None if not found.
    #[cfg(feature = "sqlite")]
    pub async fn update(
        &self,
        id: i64,
        update: &ThreadUpdate,
    ) -> Result<Option<super::thread::Thread>> {
        if update.is_empty() {
            return self.get_by_id(id).await;
        }

        let mut query: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("UPDATE threads SET ");
        let mut separated = query.separated(", ");

        if let Some(ref title) = update.title {
            separated.push("title = ");
            separated.push_bind_unseparated(title);
        }
        if let Some(delta) = update.post_count_delta {
            separated.push("post_count = post_count + ");
            separated.push_bind_unseparated(delta);
        }
        if update.touch {
            separated.push("updated_at = datetime('now')");
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

    /// Update a thread by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated thread, or None if not found.
    #[cfg(feature = "postgres")]
    pub async fn update(
        &self,
        id: i64,
        update: &ThreadUpdate,
    ) -> Result<Option<super::thread::Thread>> {
        if update.is_empty() {
            return self.get_by_id(id).await;
        }

        let mut query: QueryBuilder<sqlx::Postgres> = QueryBuilder::new("UPDATE threads SET ");
        let mut separated = query.separated(", ");

        if let Some(ref title) = update.title {
            separated.push("title = ");
            separated.push_bind_unseparated(title);
        }
        if let Some(delta) = update.post_count_delta {
            separated.push("post_count = post_count + ");
            separated.push_bind_unseparated(delta);
        }
        if update.touch {
            separated.push("updated_at = NOW()");
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

    /// Delete a thread by ID.
    ///
    /// Returns true if a thread was deleted, false if not found.
    /// Note: This will cascade delete all posts in the thread.
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM threads WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }

    /// List threads in a board, ordered by updated_at descending.
    pub async fn list_by_board(&self, board_id: i64) -> Result<Vec<super::thread::Thread>> {
        let rows = sqlx::query_as::<_, ThreadRow>(
            "SELECT id, board_id, title, author_id, post_count, created_at, updated_at
             FROM threads WHERE board_id = ? ORDER BY updated_at DESC, id DESC",
        )
        .bind(board_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// List threads in a board with pagination.
    pub async fn list_by_board_paginated(
        &self,
        board_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<super::thread::Thread>> {
        let rows = sqlx::query_as::<_, ThreadRow>(
            "SELECT id, board_id, title, author_id, post_count, created_at, updated_at
             FROM threads WHERE board_id = ? ORDER BY updated_at DESC, id DESC LIMIT ? OFFSET ?",
        )
        .bind(board_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// List threads by author.
    pub async fn list_by_author(&self, author_id: i64) -> Result<Vec<super::thread::Thread>> {
        let rows = sqlx::query_as::<_, ThreadRow>(
            "SELECT id, board_id, title, author_id, post_count, created_at, updated_at
             FROM threads WHERE author_id = ? ORDER BY updated_at DESC",
        )
        .bind(author_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Count threads in a board.
    pub async fn count_by_board(&self, board_id: i64) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM threads WHERE board_id = ?")
            .bind(board_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(count.0)
    }

    /// Touch a thread (update updated_at to now) and increment post count.
    ///
    /// This is a convenience method for when a new post is added to a thread.
    pub async fn touch_and_increment(&self, id: i64) -> Result<Option<super::thread::Thread>> {
        self.update(id, &ThreadUpdate::new().touch().increment_post_count())
            .await
    }

    /// Decrement post count when a post is deleted.
    pub async fn decrement_post_count(&self, id: i64) -> Result<Option<super::thread::Thread>> {
        self.update(id, &ThreadUpdate::new().decrement_post_count())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardRepository, NewBoard, NewThread};
    use crate::db::{NewUser, UserRepository};
    use crate::Database;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_test_board(db: &Database) -> i64 {
        let repo = BoardRepository::new(db.pool());
        let board = repo.create(&NewBoard::new("test-board")).await.unwrap();
        board.id
    }

    async fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db.pool());
        let user = repo
            .create(&NewUser::new("testuser", "hash", "Test User"))
            .await
            .unwrap();
        user.id
    }

    #[tokio::test]
    async fn test_create_thread() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).await.unwrap();

        assert_eq!(thread.board_id, board_id);
        assert_eq!(thread.title, "Test Thread");
        assert_eq!(thread.author_id, author_id);
        assert_eq!(thread.post_count, 0);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let created = repo.create(&new_thread).await.unwrap();

        let found = repo.get_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Test Thread");

        let not_found = repo.get_by_id(999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_thread_title() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        let new_thread = NewThread::new(board_id, "Original Title", author_id);
        let thread = repo.create(&new_thread).await.unwrap();

        let update = ThreadUpdate::new().title("Updated Title");
        let updated = repo.update(thread.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.title, "Updated Title");
    }

    #[tokio::test]
    async fn test_update_empty() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).await.unwrap();

        let update = ThreadUpdate::new();
        let result = repo.update(thread.id, &update).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().title, "Test Thread");
    }

    #[tokio::test]
    async fn test_update_nonexistent_thread() {
        let db = setup_db().await;
        let repo = ThreadRepository::new(db.pool());

        let update = ThreadUpdate::new().title("New Title");
        let result = repo.update(999, &update).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_touch_and_increment() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).await.unwrap();
        let original_updated_at = thread.updated_at.clone();
        assert_eq!(thread.post_count, 0);

        // Wait a tiny bit to ensure timestamp changes
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let updated = repo.touch_and_increment(thread.id).await.unwrap().unwrap();
        assert_eq!(updated.post_count, 1);
        // updated_at should be different (or at least not less than original)
        assert!(updated.updated_at >= original_updated_at);
    }

    #[tokio::test]
    async fn test_decrement_post_count() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).await.unwrap();

        // First increment
        repo.touch_and_increment(thread.id).await.unwrap();
        repo.touch_and_increment(thread.id).await.unwrap();

        let thread = repo.get_by_id(thread.id).await.unwrap().unwrap();
        assert_eq!(thread.post_count, 2);

        // Then decrement
        let updated = repo.decrement_post_count(thread.id).await.unwrap().unwrap();
        assert_eq!(updated.post_count, 1);
    }

    #[tokio::test]
    async fn test_delete_thread() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        let new_thread = NewThread::new(board_id, "Test Thread", author_id);
        let thread = repo.create(&new_thread).await.unwrap();

        let deleted = repo.delete(thread.id).await.unwrap();
        assert!(deleted);

        let found = repo.get_by_id(thread.id).await.unwrap();
        assert!(found.is_none());

        // Deleting again should return false
        let deleted_again = repo.delete(thread.id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_list_by_board() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        // Create some threads
        repo.create(&NewThread::new(board_id, "Thread 1", author_id))
            .await
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 2", author_id))
            .await
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 3", author_id))
            .await
            .unwrap();

        let threads = repo.list_by_board(board_id).await.unwrap();
        assert_eq!(threads.len(), 3);
        // Should be ordered by updated_at DESC, so newest first
        assert_eq!(threads[0].title, "Thread 3");
    }

    #[tokio::test]
    async fn test_list_by_board_paginated() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        // Create some threads
        for i in 1..=5 {
            repo.create(&NewThread::new(board_id, format!("Thread {i}"), author_id))
                .await
                .unwrap();
        }

        // Get first page
        let page1 = repo.list_by_board_paginated(board_id, 0, 2).await.unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].title, "Thread 5");
        assert_eq!(page1[1].title, "Thread 4");

        // Get second page
        let page2 = repo.list_by_board_paginated(board_id, 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].title, "Thread 3");
        assert_eq!(page2[1].title, "Thread 2");
    }

    #[tokio::test]
    async fn test_list_by_author() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;

        // Create another user
        let user_repo = UserRepository::new(db.pool());
        let other_author = user_repo
            .create(&NewUser::new("other", "hash", "Other"))
            .await
            .unwrap();

        let repo = ThreadRepository::new(db.pool());

        // Create threads by different authors
        repo.create(&NewThread::new(board_id, "Thread 1", author_id))
            .await
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 2", other_author.id))
            .await
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 3", author_id))
            .await
            .unwrap();

        let threads = repo.list_by_author(author_id).await.unwrap();
        assert_eq!(threads.len(), 2);
    }

    #[tokio::test]
    async fn test_count_by_board() {
        let db = setup_db().await;
        let board_id = create_test_board(&db).await;
        let author_id = create_test_user(&db).await;
        let repo = ThreadRepository::new(db.pool());

        assert_eq!(repo.count_by_board(board_id).await.unwrap(), 0);

        repo.create(&NewThread::new(board_id, "Thread 1", author_id))
            .await
            .unwrap();
        repo.create(&NewThread::new(board_id, "Thread 2", author_id))
            .await
            .unwrap();

        assert_eq!(repo.count_by_board(board_id).await.unwrap(), 2);
    }
}
