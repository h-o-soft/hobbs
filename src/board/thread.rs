//! Thread model for HOBBS.
//!
//! This module defines the Thread struct for thread-based boards.

/// Thread entity representing a discussion thread in a board.
#[derive(Debug, Clone)]
pub struct Thread {
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

/// Data for creating a new thread.
#[derive(Debug, Clone)]
pub struct NewThread {
    /// ID of the board to create the thread in.
    pub board_id: i64,
    /// Thread title.
    pub title: String,
    /// ID of the user creating the thread.
    pub author_id: i64,
}

impl NewThread {
    /// Create a new thread with required fields.
    pub fn new(board_id: i64, title: impl Into<String>, author_id: i64) -> Self {
        Self {
            board_id,
            title: title.into(),
            author_id,
        }
    }
}

/// Data for updating an existing thread.
#[derive(Debug, Clone, Default)]
pub struct ThreadUpdate {
    /// New title.
    pub title: Option<String>,
    /// Increment post count by this amount.
    pub post_count_delta: Option<i32>,
    /// Update the updated_at timestamp to now.
    pub touch: bool,
}

impl ThreadUpdate {
    /// Create an empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set new title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Increment post count.
    pub fn increment_post_count(mut self) -> Self {
        self.post_count_delta = Some(1);
        self
    }

    /// Decrement post count.
    pub fn decrement_post_count(mut self) -> Self {
        self.post_count_delta = Some(-1);
        self
    }

    /// Update the updated_at timestamp.
    pub fn touch(mut self) -> Self {
        self.touch = true;
        self
    }

    /// Check if any fields are set.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.post_count_delta.is_none() && !self.touch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_thread() {
        let thread = NewThread::new(1, "Test Thread", 42);
        assert_eq!(thread.board_id, 1);
        assert_eq!(thread.title, "Test Thread");
        assert_eq!(thread.author_id, 42);
    }

    #[test]
    fn test_thread_update_empty() {
        let update = ThreadUpdate::new();
        assert!(update.is_empty());
    }

    #[test]
    fn test_thread_update_title() {
        let update = ThreadUpdate::new().title("New Title");
        assert_eq!(update.title, Some("New Title".to_string()));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_thread_update_increment_post_count() {
        let update = ThreadUpdate::new().increment_post_count();
        assert_eq!(update.post_count_delta, Some(1));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_thread_update_decrement_post_count() {
        let update = ThreadUpdate::new().decrement_post_count();
        assert_eq!(update.post_count_delta, Some(-1));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_thread_update_touch() {
        let update = ThreadUpdate::new().touch();
        assert!(update.touch);
        assert!(!update.is_empty());
    }

    #[test]
    fn test_thread_update_combined() {
        let update = ThreadUpdate::new()
            .title("Updated Title")
            .increment_post_count()
            .touch();
        assert_eq!(update.title, Some("Updated Title".to_string()));
        assert_eq!(update.post_count_delta, Some(1));
        assert!(update.touch);
        assert!(!update.is_empty());
    }
}
