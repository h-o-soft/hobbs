//! Post repository for HOBBS.
//!
//! This module provides CRUD operations for posts in the database.

use rusqlite::{params, Row};

use super::post::{NewFlatPost, NewThreadPost, Post, PostUpdate};
use crate::db::Database;
use crate::{HobbsError, Result};

/// Repository for post CRUD operations.
pub struct PostRepository<'a> {
    db: &'a Database,
}

impl<'a> PostRepository<'a> {
    /// Create a new PostRepository with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new post in a thread.
    ///
    /// Returns the created post with the assigned ID.
    pub fn create_thread_post(&self, new_post: &NewThreadPost) -> Result<Post> {
        self.db.conn().execute(
            "INSERT INTO posts (board_id, thread_id, author_id, body) VALUES (?, ?, ?, ?)",
            params![
                new_post.board_id,
                new_post.thread_id,
                new_post.author_id,
                &new_post.body,
            ],
        )?;

        let id = self.db.conn().last_insert_rowid();
        self.get_by_id(id)?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Create a new post in a flat board.
    ///
    /// Returns the created post with the assigned ID.
    pub fn create_flat_post(&self, new_post: &NewFlatPost) -> Result<Post> {
        self.db.conn().execute(
            "INSERT INTO posts (board_id, author_id, title, body) VALUES (?, ?, ?, ?)",
            params![
                new_post.board_id,
                new_post.author_id,
                &new_post.title,
                &new_post.body,
            ],
        )?;

        let id = self.db.conn().last_insert_rowid();
        self.get_by_id(id)?
            .ok_or_else(|| HobbsError::NotFound("post".to_string()))
    }

    /// Get a post by ID.
    pub fn get_by_id(&self, id: i64) -> Result<Option<Post>> {
        let result = self.db.conn().query_row(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE id = ?",
            [id],
            Self::row_to_post,
        );

        match result {
            Ok(post) => Ok(Some(post)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update a post by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated post, or None if not found.
    pub fn update(&self, id: i64, update: &PostUpdate) -> Result<Option<Post>> {
        if update.is_empty() {
            return self.get_by_id(id);
        }

        let mut fields = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref title) = update.title {
            fields.push("title = ?");
            values.push(Box::new(title.clone()));
        }
        if let Some(ref body) = update.body {
            fields.push("body = ?");
            values.push(Box::new(body.clone()));
        }

        let sql = format!("UPDATE posts SET {} WHERE id = ?", fields.join(", "));
        values.push(Box::new(id));

        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
        let affected = self.db.conn().execute(&sql, params.as_slice())?;

        if affected == 0 {
            return Ok(None);
        }

        self.get_by_id(id)
    }

    /// Delete a post by ID.
    ///
    /// Returns true if a post was deleted, false if not found.
    pub fn delete(&self, id: i64) -> Result<bool> {
        let affected = self
            .db
            .conn()
            .execute("DELETE FROM posts WHERE id = ?", [id])?;
        Ok(affected > 0)
    }

    /// List posts in a thread, ordered by created_at ascending.
    pub fn list_by_thread(&self, thread_id: i64) -> Result<Vec<Post>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE thread_id = ? ORDER BY created_at ASC",
        )?;

        let posts = stmt
            .query_map([thread_id], Self::row_to_post)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(posts)
    }

    /// List posts in a thread with pagination.
    pub fn list_by_thread_paginated(
        &self,
        thread_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Post>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE thread_id = ? ORDER BY created_at ASC LIMIT ? OFFSET ?",
        )?;

        let posts = stmt
            .query_map([thread_id, limit, offset], Self::row_to_post)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(posts)
    }

    /// List posts in a flat board (posts without thread_id), ordered by created_at descending.
    pub fn list_by_flat_board(&self, board_id: i64) -> Result<Vec<Post>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE board_id = ? AND thread_id IS NULL ORDER BY created_at DESC, id DESC",
        )?;

        let posts = stmt
            .query_map([board_id], Self::row_to_post)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(posts)
    }

    /// List posts in a flat board with pagination.
    pub fn list_by_flat_board_paginated(
        &self,
        board_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Post>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE board_id = ? AND thread_id IS NULL
             ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
        )?;

        let posts = stmt
            .query_map([board_id, limit, offset], Self::row_to_post)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(posts)
    }

    /// List posts by author.
    pub fn list_by_author(&self, author_id: i64) -> Result<Vec<Post>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE author_id = ? ORDER BY created_at DESC",
        )?;

        let posts = stmt
            .query_map([author_id], Self::row_to_post)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(posts)
    }

    /// Count posts in a thread.
    pub fn count_by_thread(&self, thread_id: i64) -> Result<i64> {
        let count: i64 = self.db.conn().query_row(
            "SELECT COUNT(*) FROM posts WHERE thread_id = ?",
            [thread_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Count posts in a flat board.
    pub fn count_by_flat_board(&self, board_id: i64) -> Result<i64> {
        let count: i64 = self.db.conn().query_row(
            "SELECT COUNT(*) FROM posts WHERE board_id = ? AND thread_id IS NULL",
            [board_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Count all posts in a board (both flat and thread posts).
    pub fn count_by_board(&self, board_id: i64) -> Result<i64> {
        let count: i64 = self.db.conn().query_row(
            "SELECT COUNT(*) FROM posts WHERE board_id = ?",
            [board_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get the latest post in a thread.
    pub fn get_latest_in_thread(&self, thread_id: i64) -> Result<Option<Post>> {
        let result = self.db.conn().query_row(
            "SELECT id, board_id, thread_id, author_id, title, body, created_at
             FROM posts WHERE thread_id = ? ORDER BY created_at DESC, id DESC LIMIT 1",
            [thread_id],
            Self::row_to_post,
        );

        match result {
            Ok(post) => Ok(Some(post)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
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
    use crate::board::{BoardRepository, BoardType, NewBoard, NewThread, ThreadRepository};
    use crate::db::{NewUser, UserRepository};

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_board(db: &Database, board_type: BoardType) -> i64 {
        let repo = BoardRepository::new(db);
        let board = repo
            .create(&NewBoard::new("test-board").with_board_type(board_type))
            .unwrap();
        board.id
    }

    fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db);
        let user = repo
            .create(&NewUser::new("testuser", "hash", "Test User"))
            .unwrap();
        user.id
    }

    fn create_test_thread(db: &Database, board_id: i64, author_id: i64) -> i64 {
        let repo = ThreadRepository::new(db);
        let thread = repo
            .create(&NewThread::new(board_id, "Test Thread", author_id))
            .unwrap();
        thread.id
    }

    // Thread post tests
    #[test]
    fn test_create_thread_post() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Hello World");
        let post = repo.create_thread_post(&new_post).unwrap();

        assert_eq!(post.board_id, board_id);
        assert_eq!(post.thread_id, Some(thread_id));
        assert_eq!(post.author_id, author_id);
        assert_eq!(post.body, "Hello World");
        assert!(post.title.is_none());
        assert!(post.is_thread_post());
    }

    #[test]
    fn test_create_flat_post() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Flat);
        let author_id = create_test_user(&db);
        let repo = PostRepository::new(&db);

        let new_post = NewFlatPost::new(board_id, author_id, "Test Title", "Hello World");
        let post = repo.create_flat_post(&new_post).unwrap();

        assert_eq!(post.board_id, board_id);
        assert!(post.thread_id.is_none());
        assert_eq!(post.author_id, author_id);
        assert_eq!(post.title, Some("Test Title".to_string()));
        assert_eq!(post.body, "Hello World");
        assert!(post.is_flat_post());
    }

    #[test]
    fn test_get_by_id() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Hello");
        let created = repo.create_thread_post(&new_post).unwrap();

        let found = repo.get_by_id(created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().body, "Hello");

        let not_found = repo.get_by_id(999).unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_update_post_body() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Original Body");
        let post = repo.create_thread_post(&new_post).unwrap();

        let update = PostUpdate::new().body("Updated Body");
        let updated = repo.update(post.id, &update).unwrap().unwrap();

        assert_eq!(updated.body, "Updated Body");
    }

    #[test]
    fn test_update_post_title() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Flat);
        let author_id = create_test_user(&db);
        let repo = PostRepository::new(&db);

        let new_post = NewFlatPost::new(board_id, author_id, "Original Title", "Body");
        let post = repo.create_flat_post(&new_post).unwrap();

        let update = PostUpdate::new().title(Some("Updated Title".to_string()));
        let updated = repo.update(post.id, &update).unwrap().unwrap();

        assert_eq!(updated.title, Some("Updated Title".to_string()));
    }

    #[test]
    fn test_update_empty() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Hello");
        let post = repo.create_thread_post(&new_post).unwrap();

        let update = PostUpdate::new();
        let result = repo.update(post.id, &update).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().body, "Hello");
    }

    #[test]
    fn test_update_nonexistent_post() {
        let db = setup_db();
        let repo = PostRepository::new(&db);

        let update = PostUpdate::new().body("New Body");
        let result = repo.update(999, &update).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_delete_post() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        let new_post = NewThreadPost::new(board_id, thread_id, author_id, "Hello");
        let post = repo.create_thread_post(&new_post).unwrap();

        let deleted = repo.delete(post.id).unwrap();
        assert!(deleted);

        let found = repo.get_by_id(post.id).unwrap();
        assert!(found.is_none());

        // Deleting again should return false
        let deleted_again = repo.delete(post.id).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_list_by_thread() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        // Create some posts
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 1",
        ))
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 2",
        ))
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 3",
        ))
        .unwrap();

        let posts = repo.list_by_thread(thread_id).unwrap();
        assert_eq!(posts.len(), 3);
        // Should be ordered by created_at ASC
        assert_eq!(posts[0].body, "Post 1");
        assert_eq!(posts[2].body, "Post 3");
    }

    #[test]
    fn test_list_by_thread_paginated() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        // Create some posts
        for i in 1..=5 {
            repo.create_thread_post(&NewThreadPost::new(
                board_id,
                thread_id,
                author_id,
                format!("Post {i}"),
            ))
            .unwrap();
        }

        // Get first page
        let page1 = repo.list_by_thread_paginated(thread_id, 0, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].body, "Post 1");
        assert_eq!(page1[1].body, "Post 2");

        // Get second page
        let page2 = repo.list_by_thread_paginated(thread_id, 2, 2).unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].body, "Post 3");
        assert_eq!(page2[1].body, "Post 4");
    }

    #[test]
    fn test_list_by_flat_board() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Flat);
        let author_id = create_test_user(&db);
        let repo = PostRepository::new(&db);

        // Create some flat posts
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 1", "Body 1"))
            .unwrap();
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 2", "Body 2"))
            .unwrap();
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 3", "Body 3"))
            .unwrap();

        let posts = repo.list_by_flat_board(board_id).unwrap();
        assert_eq!(posts.len(), 3);
        // Should be ordered by created_at DESC (newest first)
        assert_eq!(posts[0].title, Some("Title 3".to_string()));
    }

    #[test]
    fn test_list_by_flat_board_paginated() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Flat);
        let author_id = create_test_user(&db);
        let repo = PostRepository::new(&db);

        // Create some flat posts
        for i in 1..=5 {
            repo.create_flat_post(&NewFlatPost::new(
                board_id,
                author_id,
                format!("Title {i}"),
                format!("Body {i}"),
            ))
            .unwrap();
        }

        // Get first page (newest first)
        let page1 = repo.list_by_flat_board_paginated(board_id, 0, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].title, Some("Title 5".to_string()));
        assert_eq!(page1[1].title, Some("Title 4".to_string()));
    }

    #[test]
    fn test_list_by_author() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Flat);
        let author_id = create_test_user(&db);

        // Create another user
        let user_repo = UserRepository::new(&db);
        let other_author = user_repo
            .create(&NewUser::new("other", "hash", "Other"))
            .unwrap();

        let repo = PostRepository::new(&db);

        // Create posts by different authors
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 1", "Body 1"))
            .unwrap();
        repo.create_flat_post(&NewFlatPost::new(
            board_id,
            other_author.id,
            "Title 2",
            "Body 2",
        ))
        .unwrap();
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 3", "Body 3"))
            .unwrap();

        let posts = repo.list_by_author(author_id).unwrap();
        assert_eq!(posts.len(), 2);
    }

    #[test]
    fn test_count_by_thread() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        assert_eq!(repo.count_by_thread(thread_id).unwrap(), 0);

        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 1",
        ))
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 2",
        ))
        .unwrap();

        assert_eq!(repo.count_by_thread(thread_id).unwrap(), 2);
    }

    #[test]
    fn test_count_by_flat_board() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Flat);
        let author_id = create_test_user(&db);
        let repo = PostRepository::new(&db);

        assert_eq!(repo.count_by_flat_board(board_id).unwrap(), 0);

        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 1", "Body 1"))
            .unwrap();
        repo.create_flat_post(&NewFlatPost::new(board_id, author_id, "Title 2", "Body 2"))
            .unwrap();

        assert_eq!(repo.count_by_flat_board(board_id).unwrap(), 2);
    }

    #[test]
    fn test_get_latest_in_thread() {
        let db = setup_db();
        let board_id = create_test_board(&db, BoardType::Thread);
        let author_id = create_test_user(&db);
        let thread_id = create_test_thread(&db, board_id, author_id);
        let repo = PostRepository::new(&db);

        // No posts yet
        let latest = repo.get_latest_in_thread(thread_id).unwrap();
        assert!(latest.is_none());

        // Add some posts
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 1",
        ))
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 2",
        ))
        .unwrap();
        repo.create_thread_post(&NewThreadPost::new(
            board_id, thread_id, author_id, "Post 3",
        ))
        .unwrap();

        let latest = repo.get_latest_in_thread(thread_id).unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().body, "Post 3");
    }
}
