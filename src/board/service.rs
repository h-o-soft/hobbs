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
    pub fn list_boards(&self, user_role: Role) -> Result<Vec<Board>> {
        let repo = BoardRepository::new(self.db);
        repo.list_accessible(user_role)
    }

    /// Get a board by ID with permission check.
    ///
    /// Returns an error if the board doesn't exist or the user doesn't have
    /// read permission.
    pub fn get_board(&self, board_id: i64, user_role: Role) -> Result<Board> {
        let repo = BoardRepository::new(self.db);
        let board = repo
            .get_by_id(board_id)?
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
    pub fn list_threads(
        &self,
        board_id: i64,
        user_role: Role,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Thread>> {
        // First check board access
        let board = self.get_board(board_id, user_role)?;

        if board.board_type != BoardType::Thread {
            return Err(HobbsError::Validation(
                "この掲示板はスレッド形式ではありません".to_string(),
            ));
        }

        let repo = ThreadRepository::new(self.db);
        let total = repo.count_by_board(board_id)?;
        let items = repo.list_by_board_paginated(board_id, pagination.offset, pagination.limit)?;

        Ok(PaginatedResult {
            items,
            total,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    /// List all threads in a board without pagination.
    pub fn list_all_threads(&self, board_id: i64, user_role: Role) -> Result<Vec<Thread>> {
        // First check board access
        let board = self.get_board(board_id, user_role)?;

        if board.board_type != BoardType::Thread {
            return Err(HobbsError::Validation(
                "この掲示板はスレッド形式ではありません".to_string(),
            ));
        }

        let repo = ThreadRepository::new(self.db);
        repo.list_by_board(board_id)
    }

    /// Get a thread by ID with permission check.
    pub fn get_thread(&self, thread_id: i64, user_role: Role) -> Result<Thread> {
        let thread_repo = ThreadRepository::new(self.db);
        let thread = thread_repo
            .get_by_id(thread_id)?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))?;

        // Check board access
        self.get_board(thread.board_id, user_role)?;

        Ok(thread)
    }

    /// List posts in a thread with permission check and pagination.
    pub fn list_posts_in_thread(
        &self,
        thread_id: i64,
        user_role: Role,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Post>> {
        // First get the thread to check permissions
        let thread = self.get_thread(thread_id, user_role)?;

        let repo = PostRepository::new(self.db);
        let total = repo.count_by_thread(thread_id)?;
        let items =
            repo.list_by_thread_paginated(thread_id, pagination.offset, pagination.limit)?;

        // Verify the thread belongs to a thread-type board
        let board_repo = BoardRepository::new(self.db);
        if let Some(board) = board_repo.get_by_id(thread.board_id)? {
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
    pub fn list_all_posts_in_thread(&self, thread_id: i64, user_role: Role) -> Result<Vec<Post>> {
        // First get the thread to check permissions
        let thread = self.get_thread(thread_id, user_role)?;

        // Verify the thread belongs to a thread-type board
        let board_repo = BoardRepository::new(self.db);
        if let Some(board) = board_repo.get_by_id(thread.board_id)? {
            if board.board_type != BoardType::Thread {
                return Err(HobbsError::Validation(
                    "この掲示板はスレッド形式ではありません".to_string(),
                ));
            }
        }

        let repo = PostRepository::new(self.db);
        repo.list_by_thread(thread_id)
    }

    /// List posts in a flat board with permission check and pagination.
    pub fn list_posts_in_flat_board(
        &self,
        board_id: i64,
        user_role: Role,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Post>> {
        // First check board access
        let board = self.get_board(board_id, user_role)?;

        if board.board_type != BoardType::Flat {
            return Err(HobbsError::Validation(
                "この掲示板はフラット形式ではありません".to_string(),
            ));
        }

        let repo = PostRepository::new(self.db);
        let total = repo.count_by_flat_board(board_id)?;
        let items =
            repo.list_by_flat_board_paginated(board_id, pagination.offset, pagination.limit)?;

        Ok(PaginatedResult {
            items,
            total,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    /// List all posts in a flat board without pagination.
    pub fn list_all_posts_in_flat_board(
        &self,
        board_id: i64,
        user_role: Role,
    ) -> Result<Vec<Post>> {
        // First check board access
        let board = self.get_board(board_id, user_role)?;

        if board.board_type != BoardType::Flat {
            return Err(HobbsError::Validation(
                "この掲示板はフラット形式ではありません".to_string(),
            ));
        }

        let repo = PostRepository::new(self.db);
        repo.list_by_flat_board(board_id)
    }

    /// Check if a user can write to a board.
    pub fn can_write(&self, board_id: i64, user_role: Role) -> Result<bool> {
        let board = self.get_board(board_id, user_role)?;
        Ok(board.can_write(user_role))
    }

    // ========== Create Operations ==========

    /// Create a new thread in a thread-type board.
    ///
    /// Returns the created thread.
    pub fn create_thread(
        &self,
        board_id: i64,
        title: impl Into<String>,
        author_id: i64,
        user_role: Role,
    ) -> Result<Thread> {
        // Check board access and write permission
        let board = self.get_board(board_id, user_role)?;

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

        let thread_repo = ThreadRepository::new(self.db);
        let new_thread = super::NewThread::new(board_id, title, author_id);
        thread_repo.create(&new_thread)
    }

    /// Create a new post in a thread.
    ///
    /// This automatically updates the thread's `updated_at` and `post_count`.
    pub fn create_thread_post(
        &self,
        thread_id: i64,
        author_id: i64,
        body: impl Into<String>,
        user_role: Role,
    ) -> Result<Post> {
        // Get thread to check permissions and get board_id
        let thread = self.get_thread(thread_id, user_role)?;

        // Check write permission on the board
        let board = self.get_board(thread.board_id, user_role)?;
        if !board.can_write(user_role) {
            return Err(HobbsError::Permission(
                "この掲示板に書き込む権限がありません".to_string(),
            ));
        }

        // Create the post
        let post_repo = PostRepository::new(self.db);
        let new_post = super::NewThreadPost::new(thread.board_id, thread_id, author_id, body);
        let post = post_repo.create_thread_post(&new_post)?;

        // Update thread's updated_at and post_count
        let thread_repo = ThreadRepository::new(self.db);
        thread_repo.touch_and_increment(thread_id)?;

        Ok(post)
    }

    /// Create a new post in a flat board.
    pub fn create_flat_post(
        &self,
        board_id: i64,
        author_id: i64,
        title: impl Into<String>,
        body: impl Into<String>,
        user_role: Role,
    ) -> Result<Post> {
        // Check board access and write permission
        let board = self.get_board(board_id, user_role)?;

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

        let post_repo = PostRepository::new(self.db);
        let new_post = super::NewFlatPost::new(board_id, author_id, title, body);
        post_repo.create_flat_post(&new_post)
    }

    // ========== Delete Operations ==========

    /// Delete a post by ID.
    ///
    /// Permission rules:
    /// - The post author can delete their own post
    /// - SubOp or higher can delete any post
    ///
    /// If the post is in a thread, this automatically decrements the thread's `post_count`.
    pub fn delete_post(&self, post_id: i64, user_id: Option<i64>, user_role: Role) -> Result<bool> {
        let post_repo = PostRepository::new(self.db);
        let post = post_repo
            .get_by_id(post_id)?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))?;

        // Check board access
        self.get_board(post.board_id, user_role)?;

        // Check delete permission
        let is_owner = user_id.is_some() && user_id == Some(post.author_id);
        let is_operator = user_role >= Role::SubOp;

        if !is_owner && !is_operator {
            return Err(HobbsError::Permission(
                "この投稿を削除する権限がありません".to_string(),
            ));
        }

        // If this is a thread post, decrement the thread's post count
        if let Some(thread_id) = post.thread_id {
            let thread_repo = ThreadRepository::new(self.db);
            thread_repo.decrement_post_count(thread_id)?;
        }

        post_repo.delete(post_id)
    }

    /// Delete a thread by ID.
    ///
    /// Permission rules:
    /// - The thread author can delete their own thread
    /// - SubOp or higher can delete any thread
    ///
    /// Note: This cascades to delete all posts in the thread.
    pub fn delete_thread(
        &self,
        thread_id: i64,
        user_id: Option<i64>,
        user_role: Role,
    ) -> Result<bool> {
        let thread_repo = ThreadRepository::new(self.db);
        let thread = thread_repo
            .get_by_id(thread_id)?
            .ok_or_else(|| HobbsError::NotFound("thread".to_string()))?;

        // Check board access
        self.get_board(thread.board_id, user_role)?;

        // Check delete permission
        let is_owner = user_id.is_some() && user_id == Some(thread.author_id);
        let is_operator = user_role >= Role::SubOp;

        if !is_owner && !is_operator {
            return Err(HobbsError::Permission(
                "このスレッドを削除する権限がありません".to_string(),
            ));
        }

        thread_repo.delete(thread_id)
    }

    /// Get a post by ID with permission check.
    pub fn get_post(&self, post_id: i64, user_role: Role) -> Result<Post> {
        let post_repo = PostRepository::new(self.db);
        let post = post_repo
            .get_by_id(post_id)?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))?;

        // Check board access
        self.get_board(post.board_id, user_role)?;

        Ok(post)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{NewBoard, NewFlatPost, NewThread, NewThreadPost};
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

    // list_boards tests
    #[test]
    fn test_list_boards_guest() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);

        // Create boards with different read permissions
        board_repo
            .create(&NewBoard::new("public").with_min_read_role(Role::Guest))
            .unwrap();
        board_repo
            .create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .unwrap();
        board_repo
            .create(&NewBoard::new("staff").with_min_read_role(Role::SubOp))
            .unwrap();

        let service = BoardService::new(&db);
        let boards = service.list_boards(Role::Guest).unwrap();

        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "public");
    }

    #[test]
    fn test_list_boards_member() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);

        board_repo
            .create(&NewBoard::new("public").with_min_read_role(Role::Guest))
            .unwrap();
        board_repo
            .create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .unwrap();
        board_repo
            .create(&NewBoard::new("staff").with_min_read_role(Role::SubOp))
            .unwrap();

        let service = BoardService::new(&db);
        let boards = service.list_boards(Role::Member).unwrap();

        assert_eq!(boards.len(), 2);
    }

    #[test]
    fn test_list_boards_sysop() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);

        board_repo
            .create(&NewBoard::new("public").with_min_read_role(Role::Guest))
            .unwrap();
        board_repo
            .create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .unwrap();
        board_repo
            .create(&NewBoard::new("staff").with_min_read_role(Role::SubOp))
            .unwrap();

        let service = BoardService::new(&db);
        let boards = service.list_boards(Role::SysOp).unwrap();

        assert_eq!(boards.len(), 3);
    }

    // get_board tests
    #[test]
    fn test_get_board_success() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);
        let board = board_repo.create(&NewBoard::new("test")).unwrap();

        let service = BoardService::new(&db);
        let result = service.get_board(board.id, Role::Guest).unwrap();

        assert_eq!(result.name, "test");
    }

    #[test]
    fn test_get_board_not_found() {
        let db = setup_db();
        let service = BoardService::new(&db);
        let result = service.get_board(999, Role::Guest);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_board_permission_denied() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_board(board.id, Role::Guest);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_board_inactive() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);
        let board = board_repo.create(&NewBoard::new("test")).unwrap();

        // Deactivate the board
        board_repo
            .update(board.id, &crate::board::BoardUpdate::new().is_active(false))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_board(board.id, Role::SysOp);

        assert!(result.is_err());
    }

    // list_threads tests
    #[test]
    fn test_list_threads() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        for i in 1..=5 {
            thread_repo
                .create(&NewThread::new(board.id, format!("Thread {i}"), author_id))
                .unwrap();
        }

        let service = BoardService::new(&db);
        let result = service
            .list_threads(board.id, Role::Guest, Pagination::new(0, 3))
            .unwrap();

        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 5);
        assert!(result.has_more());
    }

    #[test]
    fn test_list_threads_pagination() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        for i in 1..=5 {
            thread_repo
                .create(&NewThread::new(board.id, format!("Thread {i}"), author_id))
                .unwrap();
        }

        let service = BoardService::new(&db);

        // First page
        let page1 = service
            .list_threads(board.id, Role::Guest, Pagination::first(2))
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.has_more());

        // Second page
        let page2 = service
            .list_threads(board.id, Role::Guest, page1.next_page().unwrap())
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert!(page2.has_more());

        // Third page
        let page3 = service
            .list_threads(board.id, Role::Guest, page2.next_page().unwrap())
            .unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(!page3.has_more());
    }

    #[test]
    fn test_list_threads_flat_board_error() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.list_threads(board.id, Role::Guest, Pagination::first(10));

        assert!(result.is_err());
    }

    // list_posts_in_thread tests
    #[test]
    fn test_list_posts_in_thread() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .unwrap();

        let post_repo = PostRepository::new(&db);
        for i in 1..=5 {
            post_repo
                .create_thread_post(&NewThreadPost::new(
                    board.id,
                    thread.id,
                    author_id,
                    format!("Post {i}"),
                ))
                .unwrap();
        }

        let service = BoardService::new(&db);
        let result = service
            .list_posts_in_thread(thread.id, Role::Guest, Pagination::new(0, 3))
            .unwrap();

        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 5);
        assert!(result.has_more());
    }

    #[test]
    fn test_list_all_posts_in_thread() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .unwrap();

        let post_repo = PostRepository::new(&db);
        for i in 1..=5 {
            post_repo
                .create_thread_post(&NewThreadPost::new(
                    board.id,
                    thread.id,
                    author_id,
                    format!("Post {i}"),
                ))
                .unwrap();
        }

        let service = BoardService::new(&db);
        let posts = service
            .list_all_posts_in_thread(thread.id, Role::Guest)
            .unwrap();

        assert_eq!(posts.len(), 5);
    }

    // list_posts_in_flat_board tests
    #[test]
    fn test_list_posts_in_flat_board() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let post_repo = PostRepository::new(&db);
        for i in 1..=5 {
            post_repo
                .create_flat_post(&NewFlatPost::new(
                    board.id,
                    author_id,
                    format!("Title {i}"),
                    format!("Body {i}"),
                ))
                .unwrap();
        }

        let service = BoardService::new(&db);
        let result = service
            .list_posts_in_flat_board(board.id, Role::Guest, Pagination::new(0, 3))
            .unwrap();

        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 5);
        assert!(result.has_more());
    }

    #[test]
    fn test_list_posts_in_flat_board_thread_error() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("thread").with_board_type(BoardType::Thread))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.list_posts_in_flat_board(board.id, Role::Guest, Pagination::first(10));

        assert!(result.is_err());
    }

    #[test]
    fn test_list_all_posts_in_flat_board() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let post_repo = PostRepository::new(&db);
        for i in 1..=5 {
            post_repo
                .create_flat_post(&NewFlatPost::new(
                    board.id,
                    author_id,
                    format!("Title {i}"),
                    format!("Body {i}"),
                ))
                .unwrap();
        }

        let service = BoardService::new(&db);
        let posts = service
            .list_all_posts_in_flat_board(board.id, Role::Guest)
            .unwrap();

        assert_eq!(posts.len(), 5);
    }

    // can_write tests
    #[test]
    fn test_can_write() {
        let db = setup_db();
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_min_write_role(Role::Member))
            .unwrap();

        let service = BoardService::new(&db);

        assert!(!service.can_write(board.id, Role::Guest).unwrap());
        assert!(service.can_write(board.id, Role::Member).unwrap());
        assert!(service.can_write(board.id, Role::SysOp).unwrap());
    }

    // get_thread tests
    #[test]
    fn test_get_thread() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_thread(thread.id, Role::Guest).unwrap();

        assert_eq!(result.title, "Test Thread");
    }

    #[test]
    fn test_get_thread_not_found() {
        let db = setup_db();
        let service = BoardService::new(&db);
        let result = service.get_thread(999, Role::Guest);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_thread_permission_denied() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(
                &NewBoard::new("members")
                    .with_min_read_role(Role::Member)
                    .with_board_type(BoardType::Thread),
            )
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_thread(thread.id, Role::Guest);

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

    #[test]
    fn test_create_thread_success() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .unwrap();

        assert_eq!(thread.title, "Test Thread");
        assert_eq!(thread.board_id, board.id);
        assert_eq!(thread.author_id, author_id);
        assert_eq!(thread.post_count, 0);
    }

    #[test]
    fn test_create_thread_flat_board_error() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.create_thread(board.id, "Test Thread", author_id, Role::Member);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_thread_permission_denied() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(
                &NewBoard::new("members")
                    .with_board_type(BoardType::Thread)
                    .with_min_write_role(Role::Member),
            )
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.create_thread(board.id, "Test Thread", author_id, Role::Guest);

        assert!(result.is_err());
    }

    // ========== create_thread_post tests ==========

    #[test]
    fn test_create_thread_post_success() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .unwrap();
        assert_eq!(thread.post_count, 0);

        let service = BoardService::new(&db);
        let post = service
            .create_thread_post(thread.id, author_id, "Test Body", Role::Member)
            .unwrap();

        assert_eq!(post.body, "Test Body");
        assert_eq!(post.thread_id, Some(thread.id));
        assert_eq!(post.board_id, board.id);

        // Check thread post_count was incremented
        let updated_thread = thread_repo.get_by_id(thread.id).unwrap().unwrap();
        assert_eq!(updated_thread.post_count, 1);
    }

    #[test]
    fn test_create_thread_post_permission_denied() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(
                &NewBoard::new("subop")
                    .with_board_type(BoardType::Thread)
                    .with_min_write_role(Role::SubOp),
            )
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        let thread = thread_repo
            .create(&NewThread::new(board.id, "Test Thread", author_id))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.create_thread_post(thread.id, author_id, "Test Body", Role::Member);

        assert!(result.is_err());
    }

    // ========== create_flat_post tests ==========

    #[test]
    fn test_create_flat_post_success() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .unwrap();

        assert_eq!(post.title, Some("Test Title".to_string()));
        assert_eq!(post.body, "Test Body");
        assert_eq!(post.board_id, board.id);
        assert!(post.thread_id.is_none());
    }

    #[test]
    fn test_create_flat_post_thread_board_error() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("thread").with_board_type(BoardType::Thread))
            .unwrap();

        let service = BoardService::new(&db);
        let result =
            service.create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_flat_post_permission_denied() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(
                &NewBoard::new("subop")
                    .with_board_type(BoardType::Flat)
                    .with_min_write_role(Role::SubOp),
            )
            .unwrap();

        let service = BoardService::new(&db);
        let result =
            service.create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member);

        assert!(result.is_err());
    }

    // ========== delete_post tests ==========

    #[test]
    fn test_delete_post_by_owner() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .unwrap();

        let deleted = service
            .delete_post(post.id, Some(author_id), Role::Member)
            .unwrap();

        assert!(deleted);

        // Verify post is gone
        let result = service.get_post(post.id, Role::Member);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_post_by_subop() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .unwrap();

        // SubOp (not owner) can delete
        let deleted = service
            .delete_post(post.id, Some(999), Role::SubOp)
            .unwrap();

        assert!(deleted);
    }

    #[test]
    fn test_delete_post_permission_denied() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .unwrap();

        // Other user (not owner, not SubOp) cannot delete
        let result = service.delete_post(post.id, Some(999), Role::Member);

        assert!(result.is_err());
    }

    #[test]
    fn test_delete_post_guest_cannot_delete_others_post() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .unwrap();

        // Guest with no user_id cannot delete
        let result = service.delete_post(post.id, None, Role::Guest);

        assert!(result.is_err());
    }

    #[test]
    fn test_delete_thread_post_decrements_count() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .unwrap();

        // Create two posts
        service
            .create_thread_post(thread.id, author_id, "Post 1", Role::Member)
            .unwrap();
        let post2 = service
            .create_thread_post(thread.id, author_id, "Post 2", Role::Member)
            .unwrap();

        let thread_repo = ThreadRepository::new(&db);
        let thread_before = thread_repo.get_by_id(thread.id).unwrap().unwrap();
        assert_eq!(thread_before.post_count, 2);

        // Delete one post
        service
            .delete_post(post2.id, Some(author_id), Role::Member)
            .unwrap();

        let thread_after = thread_repo.get_by_id(thread.id).unwrap().unwrap();
        assert_eq!(thread_after.post_count, 1);
    }

    // ========== delete_thread tests ==========

    #[test]
    fn test_delete_thread_by_owner() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .unwrap();

        let deleted = service
            .delete_thread(thread.id, Some(author_id), Role::Member)
            .unwrap();

        assert!(deleted);

        // Verify thread is gone
        let result = service.get_thread(thread.id, Role::Member);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_thread_by_subop() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .unwrap();

        // SubOp (not owner) can delete
        let deleted = service
            .delete_thread(thread.id, Some(999), Role::SubOp)
            .unwrap();

        assert!(deleted);
    }

    #[test]
    fn test_delete_thread_permission_denied() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .unwrap();

        // Other user (not owner, not SubOp) cannot delete
        let result = service.delete_thread(thread.id, Some(999), Role::Member);

        assert!(result.is_err());
    }

    #[test]
    fn test_delete_thread_cascades_posts() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("test").with_board_type(BoardType::Thread))
            .unwrap();

        let service = BoardService::new(&db);
        let thread = service
            .create_thread(board.id, "Test Thread", author_id, Role::Member)
            .unwrap();

        // Create posts in the thread
        let post1 = service
            .create_thread_post(thread.id, author_id, "Post 1", Role::Member)
            .unwrap();
        let post2 = service
            .create_thread_post(thread.id, author_id, "Post 2", Role::Member)
            .unwrap();

        // Delete thread
        service
            .delete_thread(thread.id, Some(author_id), Role::Member)
            .unwrap();

        // Verify posts are also gone
        let post_repo = PostRepository::new(&db);
        assert!(post_repo.get_by_id(post1.id).unwrap().is_none());
        assert!(post_repo.get_by_id(post2.id).unwrap().is_none());
    }

    // ========== get_post tests ==========

    #[test]
    fn test_get_post_success() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(&NewBoard::new("flat").with_board_type(BoardType::Flat))
            .unwrap();

        let service = BoardService::new(&db);
        let post = service
            .create_flat_post(board.id, author_id, "Test Title", "Test Body", Role::Member)
            .unwrap();

        let result = service.get_post(post.id, Role::Guest).unwrap();

        assert_eq!(result.id, post.id);
        assert_eq!(result.body, "Test Body");
    }

    #[test]
    fn test_get_post_not_found() {
        let db = setup_db();
        let service = BoardService::new(&db);
        let result = service.get_post(999, Role::Guest);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_post_permission_denied() {
        let db = setup_db();
        let author_id = create_test_user(&db);
        let board_repo = BoardRepository::new(&db);
        let board = board_repo
            .create(
                &NewBoard::new("members")
                    .with_board_type(BoardType::Flat)
                    .with_min_read_role(Role::Member),
            )
            .unwrap();

        let post_repo = PostRepository::new(&db);
        let post = post_repo
            .create_flat_post(&NewFlatPost::new(board.id, author_id, "Title", "Body"))
            .unwrap();

        let service = BoardService::new(&db);
        let result = service.get_post(post.id, Role::Guest);

        assert!(result.is_err());
    }
}
