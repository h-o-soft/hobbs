//! Post repository for HOBBS.
//!
//! This module provides CRUD operations for posts in the database.

use super::post::{NewFlatPost, NewThreadPost, Post, PostUpdate};
use crate::db::DbPool;
use crate::{HobbsError, Result};

/// Repository for post CRUD operations.
pub struct PostRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> PostRepository<'a> {
    /// Create a new PostRepository with the given pool reference.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new post in a thread.
    ///
    /// Returns the created post with the assigned ID.
    #[cfg(feature = "sqlite")]
    pub async fn create_thread_post(&self, new_post: &NewThreadPost) -> Result<Post> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO posts (board_id, thread_id, author_id, body) VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(new_post.board_id)
        .bind(new_post.thread_id)
        .bind(new_post.author_id)
        .bind(&new_post.body)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Create a new post in a thread.
    ///
    /// Returns the created post with the assigned ID.
    #[cfg(feature = "postgres")]
    pub async fn create_thread_post(&self, new_post: &NewThreadPost) -> Result<Post> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO posts (board_id, thread_id, author_id, body) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(new_post.board_id)
        .bind(new_post.thread_id)
        .bind(new_post.author_id)
        .bind(&new_post.body)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Create a new post in a flat board.
    ///
    /// Returns the created post with the assigned ID.
    #[cfg(feature = "sqlite")]
    pub async fn create_flat_post(&self, new_post: &NewFlatPost) -> Result<Post> {
        let id: i64 =
            sqlx::query_scalar("INSERT INTO posts (board_id, author_id, title, body) VALUES (?, ?, ?, ?) RETURNING id")
                .bind(new_post.board_id)
                .bind(new_post.author_id)
                .bind(&new_post.title)
                .bind(&new_post.body)
                .fetch_one(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Create a new post in a flat board.
    ///
    /// Returns the created post with the assigned ID.
    #[cfg(feature = "postgres")]
    pub async fn create_flat_post(&self, new_post: &NewFlatPost) -> Result<Post> {
        let id: i64 =
            sqlx::query_scalar("INSERT INTO posts (board_id, author_id, title, body) VALUES ($1, $2, $3, $4) RETURNING id")
                .bind(new_post.board_id)
                .bind(new_post.author_id)
                .bind(&new_post.title)
                .bind(&new_post.body)
                .fetch_one(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Get a post by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Post>> {
        let post = sqlx::query_as::<_, Post>(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(post)
    }

    /// Update a post by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated post, or None if not found.
    pub async fn update(&self, id: i64, update: &PostUpdate) -> Result<Option<Post>> {
        if update.is_empty() {
            return self.get_by_id(id).await;
        }

        // Build dynamic query based on what fields are set
        let mut set_clauses = Vec::new();
        let mut param_num = 1;

        if update.title.is_some() {
            set_clauses.push(format!("title = ${}", param_num));
            param_num += 1;
        }
        if update.body.is_some() {
            set_clauses.push(format!("body = ${}", param_num));
            param_num += 1;
        }

        let sql = format!("UPDATE posts SET {} WHERE id = ${}", set_clauses.join(", "), param_num);

        // Build the query dynamically
        let mut query = sqlx::query(&sql);

        // Bind values in order
        if let Some(ref title_opt) = update.title {
            query = query.bind(title_opt.as_ref());
        }
        if let Some(ref body) = update.body {
            query = query.bind(body);
        }
        query = query.bind(id);

        let result = query
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_by_id(id).await
    }

    /// Delete a post by ID.
    ///
    /// Returns true if a post was deleted, false if not found.
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// List posts in a thread, ordered by created_at descending.
    pub async fn list_by_thread(&self, thread_id: i64) -> Result<Vec<Post>> {
        let posts = sqlx::query_as::<_, Post>(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE thread_id = $1 ORDER BY created_at DESC, id DESC",
        )
        .bind(thread_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(posts)
    }

    /// List posts in a thread with pagination.
    pub async fn list_by_thread_paginated(
        &self,
        thread_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Post>> {
        let posts = sqlx::query_as::<_, Post>(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE thread_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2 OFFSET $3",
        )
        .bind(thread_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(posts)
    }

    /// List posts in a flat board (posts without thread_id), ordered by created_at descending.
    pub async fn list_by_flat_board(&self, board_id: i64) -> Result<Vec<Post>> {
        let posts = sqlx::query_as::<_, Post>(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE board_id = $1 AND thread_id IS NULL ORDER BY created_at DESC, id DESC",
        )
        .bind(board_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(posts)
    }

    /// List posts in a flat board with pagination.
    pub async fn list_by_flat_board_paginated(
        &self,
        board_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Post>> {
        let posts = sqlx::query_as::<_, Post>(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE board_id = $1 AND thread_id IS NULL
             ORDER BY created_at DESC, id DESC LIMIT $2 OFFSET $3",
        )
        .bind(board_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(posts)
    }

    /// List posts by author.
    pub async fn list_by_author(&self, author_id: i64) -> Result<Vec<Post>> {
        let posts = sqlx::query_as::<_, Post>(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE author_id = $1 ORDER BY created_at DESC",
        )
        .bind(author_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(posts)
    }

    /// Count posts in a thread.
    pub async fn count_by_thread(&self, thread_id: i64) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE thread_id = $1")
            .bind(thread_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count)
    }

    /// Count posts in a flat board.
    pub async fn count_by_flat_board(&self, board_id: i64) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE board_id = $1 AND thread_id IS NULL",
        )
        .bind(board_id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count)
    }

    /// Count all posts in a board (both flat and thread posts).
    pub async fn count_by_board(&self, board_id: i64) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE board_id = $1")
            .bind(board_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count)
    }

    /// Get the latest post in a thread.
    pub async fn get_latest_in_thread(&self, thread_id: i64) -> Result<Option<Post>> {
        let post = sqlx::query_as::<_, Post>(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE thread_id = $1 ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(thread_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(post)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::board::{BoardRepository, BoardType, NewBoard, NewThread, ThreadRepository};
    use crate::Database;
    use sqlx::SqlitePool;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_test_board(pool: &SqlitePool, board_type: BoardType) -> i64 {
        let repo = BoardRepository::new(pool);
        let board = repo
            .create(&NewBoard::new("test-board").with_board_type(board_type))
            .await
            .unwrap();
        board.id
    }

    async fn create_test_user(pool: &SqlitePool) -> i64 {
        sqlx::query("INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)")
            .bind("testuser")
            .bind("hash")
            .bind("Test User")
            .bind("member")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn create_another_user(pool: &SqlitePool, username: &str) -> i64 {
        sqlx::query("INSERT INTO users (username, password, nickname, role) VALUES (?, ?, ?, ?)")
            .bind(username)
            .bind("hash")
            .bind("Other User")
            .bind("member")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn create_test_thread(pool: &SqlitePool, board_id: i64, author_id: i64) -> i64 {
        let repo = ThreadRepository::new(pool);
        let thread = repo
            .create(&NewThread::new(board_id, "Test Thread", author_id))
            .await
            .unwrap();
        thread.id
    }

    // Thread post tests
    #[tokio::test]
    async fn test_create_thread_post() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Hello World");
        let post = repo.create_thread_post(&new_post).await.unwrap();

        assert_eq!(post.board_id, board_id);
        assert_eq!(post.thread_id, Some(thread_id));
        assert_eq!(post.author_id, author_id);
        assert_eq!(post.body, "Hello World");
        assert!(post.title.is_none());
        assert!(post.is_thread_post());
    }

    #[tokio::test]
    async fn test_create_flat_post() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Flat).await;
        let repo = PostRepository::new(pool);

        let new_post = NewFlatPost::new(board_id, author_id, "Test Title", "Hello World");
        let post = repo.create_flat_post(&new_post).await.unwrap();

        assert_eq!(post.board_id, board_id);
        assert!(post.thread_id.is_none());
        assert_eq!(post.author_id, author_id);
        assert_eq!(post.title, Some("Test Title".to_string()));
        assert_eq!(post.body, "Hello World");
        assert!(post.is_flat_post());
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Hello");
        let created = repo.create_thread_post(&new_post).await.unwrap();

        let found = repo.get_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().body, "Hello");

        let not_found = repo.get_by_id(999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_post_body() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Original Body");
        let post = repo.create_thread_post(&new_post).await.unwrap();

        let update = PostUpdate::new().body("Updated Body");
        let updated = repo.update(post.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.body, "Updated Body");
    }

    #[tokio::test]
    async fn test_update_post_title() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Flat).await;
        let repo = PostRepository::new(pool);

        let new_post = NewFlatPost::new(board_id, author_id, "Original Title", "Body");
        let post = repo.create_flat_post(&new_post).await.unwrap();

        let update = PostUpdate::new().title(Some("Updated Title".to_string()));
        let updated = repo.update(post.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.title, Some("Updated Title".to_string()));
    }

    #[tokio::test]
    async fn test_update_empty() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Hello");
        let post = repo.create_thread_post(&new_post).await.unwrap();

        let update = PostUpdate::new();
        let result = repo.update(post.id, &update).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().body, "Hello");
    }

    #[tokio::test]
    async fn test_update_nonexistent_post() {
        let db = setup_db().await;
        let pool = db.pool();
        let repo = PostRepository::new(pool);

        let update = PostUpdate::new().body("New Body");
        let result = repo.update(999, &update).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_post() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Hello");
        let post = repo.create_thread_post(&new_post).await.unwrap();

        let deleted = repo.delete(post.id).await.unwrap();
        assert!(deleted);

        let found = repo.get_by_id(post.id).await.unwrap();
        assert!(found.is_none());

        // Deleting again should return false
        let deleted_again = repo.delete(post.id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_list_by_thread() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        // Create some posts
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 1",
        ))
        .await
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 2",
        ))
        .await
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 3",
        ))
        .await
        .unwrap();

        let posts = repo.list_by_thread(thread_id).await.unwrap();
        assert_eq!(posts.len(), 3);
        // Should be ordered by created_at DESC
        assert_eq!(posts[0].body, "Post 3");
        assert_eq!(posts[2].body, "Post 1");
    }

    #[tokio::test]
    async fn test_list_by_thread_paginated() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        // Create some posts
        for i in 1..=5 {
            repo.create_thread_post(&NewThreadPost::new(
                board_id,
                thread_id,
                author_id,
                format!("Post {i}"),
            ))
            .await
            .unwrap();
        }

        // Get first page
        let page1 = repo
            .list_by_thread_paginated(thread_id, 0, 2)
            .await
            .unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].body, "Post 5");
        assert_eq!(page1[1].body, "Post 4");

        // Get second page
        let page2 = repo
            .list_by_thread_paginated(thread_id, 2, 2)
            .await
            .unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].body, "Post 3");
        assert_eq!(page2[1].body, "Post 2");
    }

    #[tokio::test]
    async fn test_list_by_flat_board() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Flat).await;
        let repo = PostRepository::new(pool);

        // Create some flat posts
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 1", "Body 1"))
            .await
            .unwrap();
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 2", "Body 2"))
            .await
            .unwrap();
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 3", "Body 3"))
            .await
            .unwrap();

        let posts = repo.list_by_flat_board(board_id).await.unwrap();
        assert_eq!(posts.len(), 3);
        // Should be ordered by created_at DESC (newest first)
        assert_eq!(posts[0].title, Some("Title 3".to_string()));
    }

    #[tokio::test]
    async fn test_list_by_flat_board_paginated() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Flat).await;
        let repo = PostRepository::new(pool);

        // Create some flat posts
        for i in 1..=5 {
            repo.create_flat_post(&NewFlatPost::new(
                board_id,
                author_id,
                format!("Title {i}"),
                format!("Body {i}"),
            ))
            .await
            .unwrap();
        }

        // Get first page (newest first)
        let page1 = repo
            .list_by_flat_board_paginated(board_id, 0, 2)
            .await
            .unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].title, Some("Title 5".to_string()));
        assert_eq!(page1[1].title, Some("Title 4".to_string()));
    }

    #[tokio::test]
    async fn test_list_by_author() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Flat).await;

        // Create another user
        let other_author = create_another_user(pool, "other").await;

        let repo = PostRepository::new(pool);

        // Create posts by different authors
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 1", "Body 1"))
            .await
            .unwrap();
        repo.create_flat_post(&NewFlatPost::new(
            board_id,
            other_author,
            "Title 2",
            "Body 2",
        ))
        .await
        .unwrap();
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 3", "Body 3"))
            .await
            .unwrap();

        let posts = repo.list_by_author(author_id).await.unwrap();
        assert_eq!(posts.len(), 2);
    }

    #[tokio::test]
    async fn test_count_by_thread() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        assert_eq!(repo.count_by_thread(thread_id).await.unwrap(), 0);

        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 1",
        ))
        .await
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 2",
        ))
        .await
        .unwrap();

        assert_eq!(repo.count_by_thread(thread_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_count_by_flat_board() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Flat).await;
        let repo = PostRepository::new(pool);

        assert_eq!(repo.count_by_flat_board(board_id).await.unwrap(), 0);

        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 1", "Body 1"))
            .await
            .unwrap();
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 2", "Body 2"))
            .await
            .unwrap();

        assert_eq!(repo.count_by_flat_board(board_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_latest_in_thread() {
        let db = setup_db().await;
        let pool = db.pool();
        let author_id = create_test_user(pool).await;
        let board_id = create_test_board(pool, BoardType::Thread).await;
        let thread_id = create_test_thread(pool, board_id, author_id).await;
        let repo = PostRepository::new(pool);

        // No posts yet
        let latest = repo.get_latest_in_thread(thread_id).await.unwrap();
        assert!(latest.is_none());

        // Add some posts
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 1",
        ))
        .await
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 2",
        ))
        .await
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 3",
        ))
        .await
        .unwrap();

        let latest = repo.get_latest_in_thread(thread_id).await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().body, "Post 3");
    }
}
