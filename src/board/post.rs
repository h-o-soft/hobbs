//! Post model for HOBBS.
//!
//! This module defines the Post struct for both thread-based and flat boards.

/// Post entity representing a message in a board or thread.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Post {
    /// Unique post ID.
    pub id: i64,
    /// ID of the board this post belongs to.
    pub board_id: i64,
    /// ID of the thread this post belongs to (None for flat boards).
    pub thread_id: Option<i64>,
    /// ID of the user who created the post.
    pub author_id: i64,
    /// Post title (used for flat boards, None for thread posts).
    pub title: Option<String>,
    /// Post body/content.
    pub body: String,
    /// Post creation timestamp.
    pub created_at: String,
}

impl Post {
    /// Check if this post is in a thread (thread-based board).
    pub fn is_thread_post(&self) -> bool {
        self.thread_id.is_some()
    }

    /// Check if this post is a flat post (flat board).
    pub fn is_flat_post(&self) -> bool {
        self.thread_id.is_none()
    }
}

/// Data for creating a new post in a thread.
#[derive(Debug, Clone)]
pub struct NewThreadPost {
    /// ID of the board.
    pub board_id: i64,
    /// ID of the thread to post in.
    pub thread_id: i64,
    /// ID of the user creating the post.
    pub author_id: i64,
    /// Post body/content.
    pub body: String,
}

impl NewThreadPost {
    /// Create a new thread post with required fields.
    pub fn new(board_id: i64, thread_id: i64, author_id: i64, body: impl Into<String>) -> Self {
        Self {
            board_id,
            thread_id,
            author_id,
            body: body.into(),
        }
    }
}

/// Data for creating a new post in a flat board.
#[derive(Debug, Clone)]
pub struct NewFlatPost {
    /// ID of the board.
    pub board_id: i64,
    /// ID of the user creating the post.
    pub author_id: i64,
    /// Post title.
    pub title: String,
    /// Post body/content.
    pub body: String,
}

impl NewFlatPost {
    /// Create a new flat post with required fields.
    pub fn new(
        board_id: i64,
        author_id: i64,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            board_id,
            author_id,
            title: title.into(),
            body: body.into(),
        }
    }
}

/// Data for updating an existing post.
#[derive(Debug, Clone, Default)]
pub struct PostUpdate {
    /// New title (for flat posts).
    pub title: Option<Option<String>>,
    /// New body.
    pub body: Option<String>,
}

impl PostUpdate {
    /// Create an empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set new title.
    pub fn title(mut self, title: Option<String>) -> Self {
        self.title = Some(title);
        self
    }

    /// Set new body.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Check if any fields are set.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.body.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_is_thread_post() {
        let post = Post {
            id: 1,
            board_id: 1,
            thread_id: Some(1),
            author_id: 1,
            title: None,
            body: "Test".to_string(),
            created_at: "2024-01-01".to_string(),
        };
        assert!(post.is_thread_post());
        assert!(!post.is_flat_post());
    }

    #[test]
    fn test_post_is_flat_post() {
        let post = Post {
            id: 1,
            board_id: 1,
            thread_id: None,
            author_id: 1,
            title: Some("Title".to_string()),
            body: "Test".to_string(),
            created_at: "2024-01-01".to_string(),
        };
        assert!(!post.is_thread_post());
        assert!(post.is_flat_post());
    }

    #[test]
    fn test_new_thread_post() {
        let post = NewThreadPost::new(1, 2, 3, "Hello World");
        assert_eq!(post.board_id, 1);
        assert_eq!(post.thread_id, 2);
        assert_eq!(post.author_id, 3);
        assert_eq!(post.body, "Hello World");
    }

    #[test]
    fn test_new_flat_post() {
        let post = NewFlatPost::new(1, 2, "Title", "Body");
        assert_eq!(post.board_id, 1);
        assert_eq!(post.author_id, 2);
        assert_eq!(post.title, "Title");
        assert_eq!(post.body, "Body");
    }

    #[test]
    fn test_post_update_empty() {
        let update = PostUpdate::new();
        assert!(update.is_empty());
    }

    #[test]
    fn test_post_update_body() {
        let update = PostUpdate::new().body("New Body");
        assert_eq!(update.body, Some("New Body".to_string()));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_post_update_title() {
        let update = PostUpdate::new().title(Some("New Title".to_string()));
        assert_eq!(update.title, Some(Some("New Title".to_string())));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_post_update_clear_title() {
        let update = PostUpdate::new().title(None);
        assert_eq!(update.title, Some(None));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_post_update_combined() {
        let update = PostUpdate::new()
            .title(Some("New Title".to_string()))
            .body("New Body");
        assert_eq!(update.title, Some(Some("New Title".to_string())));
        assert_eq!(update.body, Some("New Body".to_string()));
        assert!(!update.is_empty());
    }
}
