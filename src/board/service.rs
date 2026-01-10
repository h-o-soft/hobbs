//! Board service for HOBBS.
//!
//! This module provides high-level operations for boards, threads, and posts
//! with built-in permission checking and pagination support.

use crate::db::{Database, Role};
use crate::{HobbsError, Result};

use super::post_repository::PostRepository;
use super::repository::BoardRepository;
use super::thread_repository::ThreadRepository;
use super::types::{Board, BoardType};
use super::{Post, Thread};

// SQL datetime function for current timestamp
#[cfg(feature = "sqlite")]
const SQL_NOW: &str = "datetime('now')";
#[cfg(feature = "postgres")]
const SQL_NOW: &str = "NOW()";

/// Maximum length for post/thread titles (in characters).
pub const MAX_TITLE_LENGTH: usize = 50;

/// Maximum length for post body (in characters).
pub const MAX_BODY_LENGTH: usize = 10_000;

/// Validate a title string.
fn validate_title(title: &str) -> Result<()> {
    let char_count = title.chars().count();
    if char_count > MAX_TITLE_LENGTH {
        return Err(HobbsError::Validation(format!(
            "タイトルが長すぎます（{}文字以内）",
            MAX_TITLE_LENGTH
        )));
    }
    if title.trim().is_empty() {
        return Err(HobbsError::Validation(
            "タイトルを入力してください".to_string(),
        ));
    }
    Ok(())
}

/// Validate a post body string.
fn validate_body(body: &str) -> Result<()> {
    let char_count = body.chars().count();
    if char_count > MAX_BODY_LENGTH {
        return Err(HobbsError::Validation(format!(
            "本文が長すぎます（{}文字以内）",
            MAX_BODY_LENGTH
        )));
    }
    if body.trim().is_empty() {
        return Err(HobbsError::Validation("本文を入力してください".to_string()));
    }
    Ok(())
}

/// Pagination parameters.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pagination {
    /// Number of items to skip.
    pub offset: i64,
    /// Maximum number of items to return.
    pub limit: i64,
}

impl Pagination {
    /// Create new pagination parameters.
    pub fn new(offset: i64, limit: i64) -> Self {
        Self { offset, limit }
    }

    /// Create pagination for the first page with given limit.
    pub fn first(limit: i64) -> Self {
        Self { offset: 0, limit }
    }
}

/// Result of a paginated query.
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    /// The items in this page.
    pub items: Vec<T>,
    /// Total number of items (across all pages).
    pub total: i64,
    /// Current offset.
    pub offset: i64,
    /// Limit used for this query.
    pub limit: i64,
}

impl<T> PaginatedResult<T> {
    /// Check if there are more items after this page.
    pub fn has_more(&self) -> bool {
        self.offset + (self.items.len() as i64) < self.total
    }

    /// Get the next page pagination, or None if no more pages.
    pub fn next_page(&self) -> Option<Pagination> {
        if self.has_more() {
            Some(Pagination::new(self.offset + self.limit, self.limit))
        } else {
            None
        }
    }
}

/// Service for board operations with permission checking.
pub struct BoardService<'a> {
    db: &'a Database,
}

impl<'a> BoardService<'a> {
    /// Create a new BoardService with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// List all boards accessible by a user with the given role.
    ///
    /// Returns boards where `min_read_role <= user_role`.
    pub async fn list_boards(&self, user_role: Role) -> Result<Vec<Board>> {
        let repo = BoardRepository::new(self.db.pool());
        repo.list_accessible(user_role).await
    }

    /// Get a board by ID with permission check.
    ///
    /// Returns an error if the board doesn't exist or the user doesn't have
    /// read permission.
    pub async fn get_board(&self, board_id: i64, user_role: Role) -> Result<Board> {
        let repo = BoardRepository::new(self.db.pool());
        let board = repo
            .get_by_id(board_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("board".to_string()))?;

        if !board.is_active {
            return Err(HobbsError::NotFound("board".to_string()));
        }

        if !board.can_read(user_role) {
            return Err(HobbsError::Permission(
                "この掲示板を閲覧する権限がありません".to_string(),
            ));
        }

        Ok(board)
    }

    /// List threads in a board with permission check and pagination.
    ///
    /// Only works for thread-type boards.
    pub async fn list_threads(
        &self,
        board_id: i64,
        user_role: Role,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Thread>> {
        // First check board access
        let board = self.get_board(board_id, user_role).await?;

        if board.board_type != BoardType::Thread {
            return Err(HobbsError::Validation(
                "この掲示板はスレッド形式ではありません".to_string(),
            ));
        }

        let repo = ThreadRepository::new(self.db.pool());
        let total = repo.count_by_board(board_id).await?;
        let items = repo
            .list_by_board_paginated(board_id, pagination.offset, pagination.limit)
            .await?;

        Ok(PaginatedResult {
            items,
            total,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    /// List all threads in a board without pagination.
    pub async fn list_all_threads(&self, board_id: i64, user_role: Role) -> Result<Vec<Thread>> {
        // First check board access
        let board = self.get_board(board_id, user_role).await?;

        if board.board_type != BoardType::Thread {
            return Err(HobbsError::Validation(
                "この掲示板はスレッド形式ではありません".to_string(),
            ));
        }

        let repo = ThreadRepository::new(self.db.pool());
        repo.list_by_board(board_id).await
    }

    /// Get a thread by ID with permission check.
    pub async fn get_thread(&self, thread_id: i64, user_role: Role) -> Result<Thread> {
        let thread_repo = ThreadRepository::new(self.db.pool());
        let thread = thread_repo
            .get_by_id(thread_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))?;

        // Check board access
        self.get_board(thread.board_id, user_role).await?;

        Ok(thread)
    }

    /// List posts in a thread with permission check and pagination.
    pub async fn list_posts_in_thread(
        &self,
        thread_id: i64,
        user_role: Role,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Post>> {
        // First get the thread to check permissions
        let thread = self.get_thread(thread_id, user_role).await?;

        let repo = PostRepository::new(self.db.pool());
        let total = repo.count_by_thread(thread_id).await?;
        let items = repo
            .list_by_thread_paginated(thread_id, pagination.offset, pagination.limit)
            .await?;

        // Verify the thread belongs to a thread-type board
        let board_repo = BoardRepository::new(self.db.pool());
        if let Some(board) = board_repo.get_by_id(thread.board_id).await? {
            if board.board_type != BoardType::Thread {
                return Err(HobbsError::Validation(
                    "この掲示板はスレッド形式ではありません".to_string(),
                ));
            }
        }

        Ok(PaginatedResult {
            items,
            total,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    /// List all posts in a thread without pagination.
    pub async fn list_all_posts_in_thread(
        &self,
        thread_id: i64,
        user_role: Role,
    ) -> Result<Vec<Post>> {
        // First get the thread to check permissions
        let thread = self.get_thread(thread_id, user_role).await?;

        // Verify the thread belongs to a thread-type board
        let board_repo = BoardRepository::new(self.db.pool());
        if let Some(board) = board_repo.get_by_id(thread.board_id).await? {
            if board.board_type != BoardType::Thread {
                return Err(HobbsError::Validation(
                    "この掲示板はスレッド形式ではありません".to_string(),
                ));
            }
        }

        let repo = PostRepository::new(self.db.pool());
        repo.list_by_thread(thread_id).await
    }

    /// List posts in a flat board with permission check and pagination.
    pub async fn list_posts_in_flat_board(
        &self,
        board_id: i64,
        user_role: Role,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Post>> {
        // First check board access
        let board = self.get_board(board_id, user_role).await?;

        if board.board_type != BoardType::Flat {
            return Err(HobbsError::Validation(
                "この掲示板はフラット形式ではありません".to_string(),
            ));
        }

        let repo = PostRepository::new(self.db.pool());
        let total = repo.count_by_flat_board(board_id).await?;
        let items = repo
            .list_by_flat_board_paginated(board_id, pagination.offset, pagination.limit)
            .await?;

        Ok(PaginatedResult {
            items,
            total,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    /// List all posts in a flat board without pagination.
    pub async fn list_all_posts_in_flat_board(
        &self,
        board_id: i64,
        user_role: Role,
    ) -> Result<Vec<Post>> {
        // First check board access
        let board = self.get_board(board_id, user_role).await?;

        if board.board_type != BoardType::Flat {
            return Err(HobbsError::Validation(
                "この掲示板はフラット形式ではありません".to_string(),
            ));
        }

        let repo = PostRepository::new(self.db.pool());
        repo.list_by_flat_board(board_id).await
    }

    /// Check if a user can write to a board.
    pub async fn can_write(&self, board_id: i64, user_role: Role) -> Result<bool> {
        let board = self.get_board(board_id, user_role).await?;
        Ok(board.can_write(user_role))
    }

    // ========== Create Operations ==========

    /// Create a new thread in a thread-type board.
    ///
    /// Returns the created thread.
    pub async fn create_thread(
        &self,
        board_id: i64,
        title: impl Into<String>,
        author_id: i64,
        user_role: Role,
    ) -> Result<Thread> {
        let title = title.into();

        // Validate title
        validate_title(&title)?;

        // Check board access and write permission
        let board = self.get_board(board_id, user_role).await?;

        if board.board_type != BoardType::Thread {
            return Err(HobbsError::Validation(
                "この掲示板はスレッド形式ではありません".to_string(),
            ));
        }

        if !board.can_write(user_role) {
            return Err(HobbsError::Permission(
                "この掲示板に書き込む権限がありません".to_string(),
            ));
        }

        let thread_repo = ThreadRepository::new(self.db.pool());
        let new_thread = super::NewThread::new(board_id, title, author_id);
        thread_repo.create(&new_thread).await
    }

    /// Create a new post in a thread.
    ///
    /// This automatically updates the thread's `updated_at` and `post_count`.
    /// The post creation and thread update are performed atomically within a transaction.
    #[cfg(feature = "sqlite")]
    pub async fn create_thread_post(
        &self,
        thread_id: i64,
        author_id: i64,
        body: impl Into<String>,
        user_role: Role,
    ) -> Result<Post> {
        let body = body.into();

        // Validate body
        validate_body(&body)?;

        // Get thread to check permissions and get board_id
        let thread = self.get_thread(thread_id, user_role).await?;

        // Check write permission on the board
        let board = self.get_board(thread.board_id, user_role).await?;
        if !board.can_write(user_role) {
            return Err(HobbsError::Permission(
                "この掲示板に書き込む権限がありません".to_string(),
            ));
        }

        // Start transaction
        let mut tx = self.db.begin().await?;

        // Create the post within transaction
        let post_id: i64 = sqlx::query_scalar(
            "INSERT INTO posts (board_id, thread_id, author_id, body) VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(thread.board_id)
        .bind(thread_id)
        .bind(author_id)
        .bind(&body)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        // Update thread's updated_at and post_count within transaction
        let update_sql = format!(
            "UPDATE threads SET post_count = post_count + 1, updated_at = {} WHERE id = ?",
            SQL_NOW
        );
        sqlx::query(&update_sql)
            .bind(thread_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        // Commit transaction
        tx.commit().await.map_err(|e| HobbsError::Database(e.to_string()))?;

        // Fetch the created post
        let post_repo = PostRepository::new(self.db.pool());
        post_repo
            .get_by_id(post_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Create a new post in a thread.
    ///
    /// This automatically updates the thread's `updated_at` and `post_count`.
    /// The post creation and thread update are performed atomically within a transaction.
    #[cfg(feature = "postgres")]
    pub async fn create_thread_post(
        &self,
        thread_id: i64,
        author_id: i64,
        body: impl Into<String>,
        user_role: Role,
    ) -> Result<Post> {
        let body = body.into();

        // Validate body
        validate_body(&body)?;

        // Get thread to check permissions and get board_id
        let thread = self.get_thread(thread_id, user_role).await?;

        // Check write permission on the board
        let board = self.get_board(thread.board_id, user_role).await?;
        if !board.can_write(user_role) {
            return Err(HobbsError::Permission(
                "この掲示板に書き込む権限がありません".to_string(),
            ));
        }

        // Start transaction
        let mut tx = self.db.begin().await?;

        // Create the post within transaction
        let post_id: i64 = sqlx::query_scalar(
            "INSERT INTO posts (board_id, thread_id, author_id, body) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(thread.board_id)
        .bind(thread_id)
        .bind(author_id)
        .bind(&body)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        // Update thread's updated_at and post_count within transaction
        let update_sql = format!(
            "UPDATE threads SET post_count = post_count + 1, updated_at = {} WHERE id = $1",
            SQL_NOW
        );
        sqlx::query(&update_sql)
            .bind(thread_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        // Commit transaction
        tx.commit().await.map_err(|e| HobbsError::Database(e.to_string()))?;

        // Fetch the created post
        let post_repo = PostRepository::new(self.db.pool());
        post_repo
            .get_by_id(post_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Create a new post in a flat board.
    pub async fn create_flat_post(
        &self,
        board_id: i64,
        author_id: i64,
        title: impl Into<String>,
        body: impl Into<String>,
        user_role: Role,
    ) -> Result<Post> {
        let title = title.into();
        let body = body.into();

        // Validate title and body
        validate_title(&title)?;
        validate_body(&body)?;

        // Check board access and write permission
        let board = self.get_board(board_id, user_role).await?;

        if board.board_type != BoardType::Flat {
            return Err(HobbsError::Validation(
                "この掲示板はフラット形式ではありません".to_string(),
            ));
        }

        if !board.can_write(user_role) {
            return Err(HobbsError::Permission(
                "この掲示板に書き込む権限がありません".to_string(),
            ));
        }

        let post_repo = PostRepository::new(self.db.pool());
        let new_post = super::NewFlatPost::new(board_id, author_id, title, body);
        post_repo.create_flat_post(&new_post).await
    }

    // ========== Delete Operations ==========

    /// Delete a post by ID.
    ///
    /// Permission rules:
    /// - The post author can delete their own post
    /// - SubOp or higher can delete any post
    ///
    /// If the post is in a thread, this automatically decrements the thread's `post_count`.
    /// The thread update and post deletion are performed atomically within a transaction.
    #[cfg(feature = "sqlite")]
    pub async fn delete_post(
        &self,
        post_id: i64,
        user_id: Option<i64>,
        user_role: Role,
    ) -> Result<bool> {
        let post_repo = PostRepository::new(self.db.pool());
        let post = post_repo
            .get_by_id(post_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))?;

        // Check board access
        self.get_board(post.board_id, user_role).await?;

        // Check delete permission
        let is_owner = user_id.is_some() && user_id == Some(post.author_id);
        let is_operator = user_role >= Role::SubOp;

        if !is_owner && !is_operator {
            return Err(HobbsError::Permission(
                "この投稿を削除する権限がありません".to_string(),
            ));
        }

        // Start transaction
        let mut tx = self.db.begin().await?;

        // If this is a thread post, decrement the thread's post count within transaction
        if let Some(thread_id) = post.thread_id {
            sqlx::query("UPDATE threads SET post_count = post_count - 1 WHERE id = ?")
                .bind(thread_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;
        }

        // Delete the post within transaction
        let result = sqlx::query("DELETE FROM posts WHERE id = ?")
            .bind(post_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        // Commit transaction
        tx.commit().await.map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a post by ID.
    ///
    /// Permission rules:
    /// - The post author can delete their own post
    /// - SubOp or higher can delete any post
    ///
    /// If the post is in a thread, this automatically decrements the thread's `post_count`.
    /// The thread update and post deletion are performed atomically within a transaction.
    #[cfg(feature = "postgres")]
    pub async fn delete_post(
        &self,
        post_id: i64,
        user_id: Option<i64>,
        user_role: Role,
    ) -> Result<bool> {
        let post_repo = PostRepository::new(self.db.pool());
        let post = post_repo
            .get_by_id(post_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))?;

        // Check board access
        self.get_board(post.board_id, user_role).await?;

        // Check delete permission
        let is_owner = user_id.is_some() && user_id == Some(post.author_id);
        let is_operator = user_role >= Role::SubOp;

        if !is_owner && !is_operator {
            return Err(HobbsError::Permission(
                "この投稿を削除する権限がありません".to_string(),
            ));
        }

        // Start transaction
        let mut tx = self.db.begin().await?;

        // If this is a thread post, decrement the thread's post count within transaction
        if let Some(thread_id) = post.thread_id {
            sqlx::query("UPDATE threads SET post_count = post_count - 1 WHERE id = $1")
                .bind(thread_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;
        }

        // Delete the post within transaction
        let result = sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(post_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        // Commit transaction
        tx.commit().await.map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ========== Update Operations ==========

    /// Update a post by ID.
    ///
    /// Permission rules:
    /// - The post author can edit their own post
    /// - SubOp or higher can edit any post
    pub async fn update_post(
        &self,
        post_id: i64,
        user_id: Option<i64>,
        user_role: Role,
        title: Option<String>,
        body: String,
    ) -> Result<Post> {
        // Validate input
        if let Some(ref t) = title {
            if !t.is_empty() {
                validate_title(t)?;
            }
        }
        validate_body(&body)?;

        let post_repo = PostRepository::new(self.db.pool());
        let post = post_repo
            .get_by_id(post_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))?;

        // Check board access
        self.get_board(post.board_id, user_role).await?;

        // Check edit permission
        let is_owner = user_id.is_some() && user_id == Some(post.author_id);
        let is_operator = user_role >= Role::SubOp;

        if !is_owner && !is_operator {
            return Err(HobbsError::Permission(
                "この投稿を編集する権限がありません".to_string(),
            ));
        }

        let mut update = super::post::PostUpdate::new();
        if title.is_some() {
            update.title = Some(title);
        }
        update.body = Some(body);

        post_repo
            .update(post_id, &update)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Update a thread by ID.
    ///
    /// Permission rules:
    /// - The thread author can edit their own thread
    /// - SubOp or higher can edit any thread
    pub async fn update_thread(
        &self,
        thread_id: i64,
        user_id: Option<i64>,
        user_role: Role,
        title: String,
    ) -> Result<Thread> {
        // Validate input
        validate_title(&title)?;

        let thread_repo = ThreadRepository::new(self.db.pool());
        let thread = thread_repo
            .get_by_id(thread_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))?;

        // Check board access
        self.get_board(thread.board_id, user_role).await?;

        // Check edit permission
        let is_owner = user_id.is_some() && user_id == Some(thread.author_id);
        let is_operator = user_role >= Role::SubOp;

        if !is_owner && !is_operator {
            return Err(HobbsError::Permission(
                "このスレッドを編集する権限がありません".to_string(),
            ));
        }

        let update = super::thread::ThreadUpdate::new().title(title);

        thread_repo
            .update(thread_id, &update)
            .await?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))
    }

    /// Delete a thread by ID.
    ///
    /// Permission rules:
    /// - The thread author can delete their own thread
    /// - SubOp or higher can delete any thread
    ///
    /// Note: This cascades to delete all posts in the thread.
    pub async fn delete_thread(
        &self,
        thread_id: i64,
        user_id: Option<i64>,
        user_role: Role,
    ) -> Result<bool> {
        let thread_repo = ThreadRepository::new(self.db.pool());
        let thread = thread_repo
            .get_by_id(thread_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))?;

        // Check board access
        self.get_board(thread.board_id, user_role).await?;

        // Check delete permission
        let is_owner = user_id.is_some() && user_id == Some(thread.author_id);
        let is_operator = user_role >= Role::SubOp;

        if !is_owner && !is_operator {
            return Err(HobbsError::Permission(
                "このスレッドを削除する権限がありません".to_string(),
            ));
        }

        thread_repo.delete(thread_id).await
    }

    /// Get a post by ID with permission check.
    pub async fn get_post(&self, post_id: i64, user_role: Role) -> Result<Post> {
        let post_repo = PostRepository::new(self.db.pool());
        let post = post_repo
            .get_by_id(post_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))?;

        // Check board access
        self.get_board(post.board_id, user_role).await?;

        Ok(post)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{NewBoard, NewFlatPost, NewThread, NewThreadPost};
    use crate::db::{NewUser, UserRepository};

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db.pool());
        let user = repo
            .create(&NewUser::new("testuser", "hash", "Test User"))
            .await
            .unwrap();
        user.id
    }

    // list_boards tests
    #[tokio::test]
    async fn test_list_boards_guest() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());

        // Create boards with different read permissions
        board_repo
            .create(&NewBoard::new("public").with_min_read_role(Role::Guest))
            .await
            .unwrap();
        board_repo
            .create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .await
            .unwrap();
        board_repo
            .create(&NewBoard::new("staff").with_min_read_role(Role::SubOp))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let boards = service.list_boards(Role::Guest).await.unwrap();

        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "public");
    }

    #[tokio::test]
    async fn test_list_boards_member() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());

        board_repo
            .create(&NewBoard::new("public").with_min_read_role(Role::Guest))
            .await
            .unwrap();
        board_repo
            .create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .await
            .unwrap();
        board_repo
            .create(&NewBoard::new("staff").with_min_read_role(Role::SubOp))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let boards = service.list_boards(Role::Member).await.unwrap();

        assert_eq!(boards.len(), 2);
    }

    #[tokio::test]
    async fn test_list_boards_sysop() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());

        board_repo
            .create(&NewBoard::new("public").with_min_read_role(Role::Guest))
            .await
            .unwrap();
        board_repo
            .create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .await
            .unwrap();
        board_repo
            .create(&NewBoard::new("staff").with_min_read_role(Role::SubOp))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let boards = service.list_boards(Role::SysOp).await.unwrap();

        assert_eq!(boards.len(), 3);
    }

    // get_board tests
    #[tokio::test]
    async fn test_get_board_success() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo.create(&NewBoard::new("test")).await.unwrap();

        let service = BoardService::new(&db);
        let result = service.get_board(board.id, Role::Guest).await.unwrap();

        assert_eq!(result.name, "test");
    }

    #[tokio::test]
    async fn test_get_board_not_found() {
        let db = setup_db().await;
        let service = BoardService::new(&db);
        let result = service.get_board(999, Role::Guest).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_board_permission_denied() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_board(board.id, Role::Guest).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_board_inactive() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo.create(&NewBoard::new("test")).await.unwrap();

        // Deactivate the board
        board_repo
            .update(board.id, &crate::board::BoardUpdate::new().is_active(false))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_board(board.id, Role::SysOp).await;

        assert!(result.is_err());
    }

    // list_threads tests
    #[tokio::test]
    async fn test_list_threads() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        for i in 1..=5 {
            thread_repo
                .create(&NewThread::new(board.id, format!("Thread {i}"), author_id))
                .await
                .unwrap();
        }

        let service = BoardService::new(&db);
        let result = service
            .list_threads(board.id, Role::Guest, Pagination::new(0, 3))
            .await
            .unwrap();

        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 5);
        assert!(result.has_more());
    }

    #[tokio::test]
    async fn test_list_threads_pagination() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        for i in 1..=5 {
            thread_repo
                .create(&NewThread::new(board.id, format!("Thread {i}"), author_id))
                .await
                .unwrap();
        }

        let service = BoardService::new(&db);

        // First page
        let page1 = service
            .list_threads(board.id, Role::Guest, Pagination::first(2))
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.has_more());

        // Second page
        let page2 = service
            .list_threads(board.id, Role::Guest, page1.next_page().unwrap())
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert!(page2.has_more());

        // Third page
        let page3 = service
            .list_threads(board.id, Role::Guest, page2.next_page().unwrap())
            .await
            .unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(!page3.has_more());
    }

    #[tokio::test]
    async fn test_list_threads_flat_board_error() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service
            .list_threads(board.id, Role::Guest, Pagination::first(10))
            .await;

        assert!(result.is_err());
    }

    // list_posts_in_thread tests
    #[tokio::test]
    async fn test_list_posts_in_thread() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .await
            .unwrap();

        let post_repo = PostRepository::new(db.pool());
        for i in 1..=5 {
            post_repo
                .create_thread_post(&NewThreadPost::new(
                    board.id,
                    thread.id,
                    author_id,
                    format!("Post {i}"),
                ))
                .await
                .unwrap();
        }

        let service = BoardService::new(&db);
        let result = service
            .list_posts_in_thread(thread.id, Role::Guest, Pagination::new(0, 3))
            .await
            .unwrap();

        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 5);
        assert!(result.has_more());
    }

    #[tokio::test]
    async fn test_list_all_posts_in_thread() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .await
            .unwrap();

        let post_repo = PostRepository::new(db.pool());
        for i in 1..=5 {
            post_repo
                .create_thread_post(&NewThreadPost::new(
                    board.id,
                    thread.id,
                    author_id,
                    format!("Post {i}"),
                ))
                .await
                .unwrap();
        }

        let service = BoardService::new(&db);
        let posts = service
            .list_all_posts_in_thread(thread.id, Role::Guest)
            .await
            .unwrap();

        assert_eq!(posts.len(), 5);
    }

    // list_posts_in_flat_board tests
    #[tokio::test]
    async fn test_list_posts_in_flat_board() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let post_repo = PostRepository::new(db.pool());
        for i in 1..=5 {
            post_repo
                .create_flat_post(&NewFlatPost::new(
                    board.id,
                    author_id,
                    format!("Title {i}"),
                    format!("Body {i}"),
                ))
                .await
                .unwrap();
        }

        let service = BoardService::new(&db);
        let result = service
            .list_posts_in_flat_board(board.id, Role::Guest, Pagination::new(0, 3))
            .await
            .unwrap();

        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 5);
        assert!(result.has_more());
    }

    #[tokio::test]
    async fn test_list_posts_in_flat_board_thread_error() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("thread").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service
            .list_posts_in_flat_board(board.id, Role::Guest, Pagination::first(10))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_all_posts_in_flat_board() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let post_repo = PostRepository::new(db.pool());
        for i in 1..=5 {
            post_repo
                .create_flat_post(&NewFlatPost::new(
                    board.id,
                    author_id,
                    format!("Title {i}"),
                    format!("Body {i}"),
                ))
                .await
                .unwrap();
        }

        let service = BoardService::new(&db);
        let posts = service
            .list_all_posts_in_flat_board(board.id, Role::Guest)
            .await
            .unwrap();

        assert_eq!(posts.len(), 5);
    }

    // can_write tests
    #[tokio::test]
    async fn test_can_write() {
        let db = setup_db().await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_min_write_role(Role::Member))
            .await
            .unwrap();

        let service = BoardService::new(&db);

        assert!(!service.can_write(board.id, Role::Guest).await.unwrap());
        assert!(service.can_write(board.id, Role::Member).await.unwrap());
        assert!(service.can_write(board.id, Role::SysOp).await.unwrap());
    }

    // get_thread tests
    #[tokio::test]
    async fn test_get_thread() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_thread(thread.id, Role::Guest).await.unwrap();

        assert_eq!(result.title, "Test Thread");
    }

    #[tokio::test]
    async fn test_get_thread_not_found() {
        let db = setup_db().await;
        let service = BoardService::new(&db);
        let result = service.get_thread(999, Role::Guest).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_thread_permission_denied() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(
                &NewBoard::new("members")
                    .with_min_read_role(Role::Member)
                    .with_board_type(BoardType::Thread),
            )
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_thread(thread.id, Role::Guest).await;

        assert!(result.is_err());
    }

    // Pagination tests
    #[test]
    fn test_pagination_new() {
        let pagination = Pagination::new(10, 20);
        assert_eq!(pagination.offset, 10);
        assert_eq!(pagination.limit, 20);
    }

    #[test]
    fn test_pagination_first() {
        let pagination = Pagination::first(15);
        assert_eq!(pagination.offset, 0);
        assert_eq!(pagination.limit, 15);
    }

    #[test]
    fn test_paginated_result_has_more() {
        let result: PaginatedResult<i32> = PaginatedResult {
            items: vec![1, 2, 3],
            total: 10,
            offset: 0,
            limit: 3,
        };
        assert!(result.has_more());

        let result2: PaginatedResult<i32> = PaginatedResult {
            items: vec![8, 9, 10],
            total: 10,
            offset: 7,
            limit: 3,
        };
        assert!(!result2.has_more());
    }

    #[test]
    fn test_paginated_result_next_page() {
        let result: PaginatedResult<i32> = PaginatedResult {
            items: vec![1, 2, 3],
            total: 10,
            offset: 0,
            limit: 3,
        };

        let next = result.next_page().unwrap();
        assert_eq!(next.offset, 3);
        assert_eq!(next.limit, 3);
    }

    #[test]
    fn test_paginated_result_no_next_page() {
        let result: PaginatedResult<i32> = PaginatedResult {
            items: vec![8, 9, 10],
            total: 10,
            offset: 7,
            limit: 3,
        };

        assert!(result.next_page().is_none());
    }

    // ========== create_thread tests ==========

    #[tokio::test]
    async fn test_create_thread_success() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .await
            .unwrap();

        assert_eq!(thread.title, "Test Thread");
        assert_eq!(thread.board_id, board.id);
        assert_eq!(thread.author_id, author_id);
        assert_eq!(thread.post_count, 0);
    }

    #[tokio::test]
    async fn test_create_thread_flat_board_error() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_thread_permission_denied() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(
                &NewBoard::new("members")
                    .with_board_type(BoardType::Thread)
                    .with_min_write_role(Role::Member),
            )
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service
            .create_thread(board.id, "Test Thread", author_id, Role::Guest)
            .await;

        assert!(result.is_err());
    }

    // ========== create_thread_post tests ==========

    #[tokio::test]
    async fn test_create_thread_post_success() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .await
            .unwrap();
        assert_eq!(thread.post_count, 0);

        let service = BoardService::new(&db);
        let post = service
            .create_thread_post(thread.id, author_id, "Test Body", Role::Member)
            .await
            .unwrap();

        assert_eq!(post.body, "Test Body");
        assert_eq!(post.thread_id, Some(thread.id));
        assert_eq!(post.board_id, board.id);

        // Check thread post_count was incremented
        let updated_thread = thread_repo.get_by_id(thread.id).await.unwrap().unwrap();
        assert_eq!(updated_thread.post_count, 1);
    }

    #[tokio::test]
    async fn test_create_thread_post_permission_denied() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(
                &NewBoard::new("subop")
                    .with_board_type(BoardType::Thread)
                    .with_min_write_role(Role::SubOp),
            )
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service
            .create_thread_post(thread.id, author_id, "Test Body", Role::Member)
            .await;

        assert!(result.is_err());
    }

    // ========== create_flat_post tests ==========

    #[tokio::test]
    async fn test_create_flat_post_success() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .await
            .unwrap();

        assert_eq!(post.title, Some("Test Title".to_string()));
        assert_eq!(post.body, "Test Body");
        assert_eq!(post.board_id, board.id);
        assert!(post.thread_id.is_none());
    }

    #[tokio::test]
    async fn test_create_flat_post_thread_board_error() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("thread").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_flat_post_permission_denied() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(
                &NewBoard::new("subop")
                    .with_board_type(BoardType::Flat)
                    .with_min_write_role(Role::SubOp),
            )
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .await;

        assert!(result.is_err());
    }

    // ========== delete_post tests ==========

    #[tokio::test]
    async fn test_delete_post_by_owner() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .await
            .unwrap();

        let deleted = service
            .delete_post(post.id, Some(author_id), Role::Member)
            .await
            .unwrap();

        assert!(deleted);

        // Verify post is gone
        let result = service.get_post(post.id, Role::Member).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_post_by_subop() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .await
            .unwrap();

        // SubOp (not owner) can delete
        let deleted = service
            .delete_post(post.id, Some(999), Role::SubOp)
            .await
            .unwrap();

        assert!(deleted);
    }

    #[tokio::test]
    async fn test_delete_post_permission_denied() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .await
            .unwrap();

        // Other user (not owner, not SubOp) cannot delete
        let result = service.delete_post(post.id, Some(999), Role::Member).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_post_guest_cannot_delete_others_post() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .await
            .unwrap();

        // Guest with no user_id cannot delete
        let result = service.delete_post(post.id, None, Role::Guest).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_thread_post_decrements_count() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .await
            .unwrap();

        // Create two posts
        service
            .create_thread_post(thread.id, author_id, "Post 1", Role::Member)
            .await
            .unwrap();
        let post2 = service
            .create_thread_post(thread.id, author_id, "Post 2", Role::Member)
            .await
            .unwrap();

        let thread_repo = ThreadRepository::new(db.pool());
        let thread_before = thread_repo.get_by_id(thread.id).await.unwrap().unwrap();
        assert_eq!(thread_before.post_count, 2);

        // Delete one post
        service
            .delete_post(post2.id, Some(author_id), Role::Member)
            .await
            .unwrap();

        let thread_after = thread_repo.get_by_id(thread.id).await.unwrap().unwrap();
        assert_eq!(thread_after.post_count, 1);
    }

    // ========== delete_thread tests ==========

    #[tokio::test]
    async fn test_delete_thread_by_owner() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .await
            .unwrap();

        let deleted = service
            .delete_thread(thread.id, Some(author_id), Role::Member)
            .await
            .unwrap();

        assert!(deleted);

        // Verify thread is gone
        let result = service.get_thread(thread.id, Role::Member).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_thread_by_subop() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .await
            .unwrap();

        // SubOp (not owner) can delete
        let deleted = service
            .delete_thread(thread.id, Some(999), Role::SubOp)
            .await
            .unwrap();

        assert!(deleted);
    }

    #[tokio::test]
    async fn test_delete_thread_permission_denied() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .await
            .unwrap();

        // Other user (not owner, not SubOp) cannot delete
        let result = service.delete_thread(thread.id, Some(999), Role::Member).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_thread_cascades_posts() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .await
            .unwrap();

        // Create posts in the thread
        let post1 = service
            .create_thread_post(thread.id, author_id, "Post 1", Role::Member)
            .await
            .unwrap();
        let post2 = service
            .create_thread_post(thread.id, author_id, "Post 2", Role::Member)
            .await
            .unwrap();

        // Delete thread
        service
            .delete_thread(thread.id, Some(author_id), Role::Member)
            .await
            .unwrap();

        // Verify posts are also gone
        let post_repo = PostRepository::new(db.pool());
        assert!(post_repo.get_by_id(post1.id).await.unwrap().is_none());
        assert!(post_repo.get_by_id(post2.id).await.unwrap().is_none());
    }

    // ========== update_thread tests ==========

    #[tokio::test]
    async fn test_update_thread_by_owner() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Original Title", author_id, Role::Member)
            .await
            .unwrap();

        let updated = service
            .update_thread(
                thread.id,
                Some(author_id),
                Role::Member,
                "New Title".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.id, thread.id);
    }

    #[tokio::test]
    async fn test_update_thread_by_subop() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Original Title", author_id, Role::Member)
            .await
            .unwrap();

        // SubOp (not owner) can edit
        let updated = service
            .update_thread(thread.id, Some(999), Role::SubOp, "Admin Edit".to_string())
            .await
            .unwrap();

        assert_eq!(updated.title, "Admin Edit");
    }

    #[tokio::test]
    async fn test_update_thread_permission_denied() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Original Title", author_id, Role::Member)
            .await
            .unwrap();

        // Other user (not owner, not SubOp) cannot edit
        let result = service
            .update_thread(thread.id, Some(999), Role::Member, "Hacked".to_string())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_thread_not_found() {
        let db = setup_db().await;
        let service = BoardService::new(&db);
        let result = service
            .update_thread(999, Some(1), Role::SysOp, "Title".to_string())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_thread_title_too_long() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Original Title", author_id, Role::Member)
            .await
            .unwrap();

        let long_title = "あ".repeat(51);
        let result = service
            .update_thread(thread.id, Some(author_id), Role::Member, long_title)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_thread_title_empty() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Original Title", author_id, Role::Member)
            .await
            .unwrap();

        let result = service
            .update_thread(thread.id, Some(author_id), Role::Member, "   ".to_string())
            .await;

        assert!(result.is_err());
    }

    // ========== get_post tests ==========

    #[tokio::test]
    async fn test_get_post_success() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .await
            .unwrap();

        let result = service.get_post(post.id, Role::Guest).await.unwrap();

        assert_eq!(result.id, post.id);
        assert_eq!(result.body, "Test Body");
    }

    #[tokio::test]
    async fn test_get_post_not_found() {
        let db = setup_db().await;
        let service = BoardService::new(&db);
        let result = service.get_post(999, Role::Guest).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_post_permission_denied() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(
                &NewBoard::new("members")
                    .with_board_type(BoardType::Flat)
                    .with_min_read_role(Role::Member),
            )
            .await
            .unwrap();

        let post_repo = PostRepository::new(db.pool());
        let post = post_repo
            .create_flat_post(&NewFlatPost::new(board.id, author_id, "Title", "Body"))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_post(post.id, Role::Guest).await;

        assert!(result.is_err());
    }

    // ========== Validation Tests ==========

    #[test]
    fn test_validate_title_ok() {
        assert!(super::validate_title("Normal title").is_ok());
        assert!(super::validate_title("a").is_ok());
        assert!(super::validate_title(&"あ".repeat(50)).is_ok()); // 50 chars
    }

    #[test]
    fn test_validate_title_empty() {
        assert!(super::validate_title("").is_err());
        assert!(super::validate_title("   ").is_err());
    }

    #[test]
    fn test_validate_title_too_long() {
        let long_title = "あ".repeat(51); // 51 chars
        assert!(super::validate_title(&long_title).is_err());
    }

    #[test]
    fn test_validate_body_ok() {
        assert!(super::validate_body("Normal body").is_ok());
        assert!(super::validate_body("a").is_ok());
        assert!(super::validate_body(&"あ".repeat(10_000)).is_ok()); // 10,000 chars
    }

    #[test]
    fn test_validate_body_empty() {
        assert!(super::validate_body("").is_err());
        assert!(super::validate_body("   ").is_err());
    }

    #[test]
    fn test_validate_body_too_long() {
        let long_body = "あ".repeat(10_001); // 10,001 chars
        assert!(super::validate_body(&long_body).is_err());
    }

    #[tokio::test]
    async fn test_create_thread_title_too_long() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let long_title = "あ".repeat(51);
        let result = service
            .create_thread(board.id, long_title, author_id, Role::Member)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_thread_post_body_too_long() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .await
            .unwrap();

        let long_body = "あ".repeat(10_001);
        let result = service
            .create_thread_post(thread.id, author_id, long_body, Role::Member)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_flat_post_title_too_long() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let long_title = "あ".repeat(51);
        let result = service
            .create_flat_post(board.id, author_id, long_title, "Body", Role::Member)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_flat_post_body_too_long() {
        let db = setup_db().await;
        let author_id = create_test_user(&db).await;
        let board_repo = BoardRepository::new(db.pool());
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Flat))
            .await
            .unwrap();

        let service = BoardService::new(&db);
        let long_body = "あ".repeat(10_001);
        let result = service
            .create_flat_post(board.id, author_id, "Title", long_body, Role::Member)
            .await;

        assert!(result.is_err());
    }
}
