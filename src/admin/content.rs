//! Content management for administrators.
//!
//! This module provides administrative functions for managing content:
//! - Delete posts (SubOp and above)
//! - Delete files (SubOp and above)
//! - Soft delete posts (replace body with deletion message)

use crate::board::{Post, PostRepository, PostUpdate, ThreadRepository};
use crate::db::{DbPool, User};
use crate::file::{FileMetadata, FileRepository, FileStorage};

use super::{require_admin, AdminError};

/// Deletion mode for posts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostDeletionMode {
    /// Soft delete: Replace body with deletion message.
    Soft,
    /// Hard delete: Physically delete the post.
    Hard,
}

/// Default deletion message for soft-deleted posts.
pub const DELETED_POST_MESSAGE: &str = "この投稿は削除されました";

/// Admin service for content management.
pub struct ContentAdminService<'a> {
    pool: &'a DbPool,
    storage: Option<&'a FileStorage>,
}

impl<'a> ContentAdminService<'a> {
    /// Create a new ContentAdminService.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool, storage: None }
    }

    /// Create a new ContentAdminService with file storage.
    pub fn with_storage(pool: &'a DbPool, storage: &'a FileStorage) -> Self {
        Self {
            pool,
            storage: Some(storage),
        }
    }

    /// Get a post by ID.
    ///
    /// Requires SubOp or higher permission.
    pub async fn get_post(&self, post_id: i64, admin: &User) -> Result<Post, AdminError> {
        require_admin(Some(admin))?;

        let repo = PostRepository::new(self.pool);
        let post = repo
            .get_by_id(post_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("投稿".to_string()))?;

        Ok(post)
    }

    /// Delete a post.
    ///
    /// Requires SubOp or higher permission.
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to delete
    /// * `mode` - Whether to soft delete or hard delete
    /// * `admin` - The admin user performing the deletion
    pub async fn delete_post(
        &self,
        post_id: i64,
        mode: PostDeletionMode,
        admin: &User,
    ) -> Result<bool, AdminError> {
        require_admin(Some(admin))?;

        let repo = PostRepository::new(self.pool);

        // Check if post exists
        repo.get_by_id(post_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("投稿".to_string()))?;

        match mode {
            PostDeletionMode::Soft => {
                let update = PostUpdate::new().body(DELETED_POST_MESSAGE);
                repo.update(post_id, &update).await?;
                Ok(true)
            }
            PostDeletionMode::Hard => {
                let deleted = repo.delete(post_id).await?;
                Ok(deleted)
            }
        }
    }

    /// Delete a thread and all its posts.
    ///
    /// Requires SubOp or higher permission.
    pub async fn delete_thread(&self, thread_id: i64, admin: &User) -> Result<bool, AdminError> {
        require_admin(Some(admin))?;

        let repo = ThreadRepository::new(self.pool);

        // Check if thread exists
        repo.get_by_id(thread_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("スレッド".to_string()))?;

        // Delete the thread (cascade will delete posts)
        let deleted = repo.delete(thread_id).await?;
        Ok(deleted)
    }

    /// List posts by author.
    ///
    /// Requires SubOp or higher permission.
    pub async fn list_posts_by_author(
        &self,
        author_id: i64,
        admin: &User,
    ) -> Result<Vec<Post>, AdminError> {
        require_admin(Some(admin))?;

        let repo = PostRepository::new(self.pool);
        let posts = repo.list_by_author(author_id).await?;
        Ok(posts)
    }

    /// Get a file by ID.
    ///
    /// Requires SubOp or higher permission.
    pub async fn get_file(&self, file_id: i64, admin: &User) -> Result<FileMetadata, AdminError> {
        require_admin(Some(admin))?;

        let repo = FileRepository::new(self.pool);
        let file = repo
            .get_by_id(file_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("ファイル".to_string()))?;

        Ok(file)
    }

    /// Delete a file.
    ///
    /// Requires SubOp or higher permission.
    /// Deletes both the database record and the physical file.
    pub async fn delete_file(&self, file_id: i64, admin: &User) -> Result<bool, AdminError> {
        require_admin(Some(admin))?;

        let repo = FileRepository::new(self.pool);

        // Get file info first
        let file = repo
            .get_by_id(file_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("ファイル".to_string()))?;

        // Delete from database
        let deleted = repo.delete(file_id).await?;

        // Delete physical file if storage is available
        if deleted {
            if let Some(storage) = self.storage {
                // Ignore errors when deleting physical file
                let _ = storage.delete(&file.stored_name);
            }
        }

        Ok(deleted)
    }

    /// List files by uploader.
    ///
    /// Requires SubOp or higher permission.
    pub async fn list_files_by_uploader(
        &self,
        uploader_id: i64,
        admin: &User,
    ) -> Result<Vec<FileMetadata>, AdminError> {
        require_admin(Some(admin))?;

        let repo = FileRepository::new(self.pool);
        let files = repo.list_by_uploader(uploader_id).await?;
        Ok(files)
    }

    /// List files in a folder.
    ///
    /// Requires SubOp or higher permission.
    pub async fn list_files_in_folder(
        &self,
        folder_id: i64,
        admin: &User,
    ) -> Result<Vec<FileMetadata>, AdminError> {
        require_admin(Some(admin))?;

        let repo = FileRepository::new(self.pool);
        let files = repo.list_by_folder(folder_id).await?;
        Ok(files)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::board::{BoardRepository, BoardType, NewBoard, NewThread, NewThreadPost};
    use crate::db::{Database, NewUser, Role, UserRepository};
    use crate::file::{FolderRepository, NewFile, NewFolder};
    use crate::server::CharacterEncoding;
    use sqlx::SqlitePool;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_test_user(pool: &SqlitePool, username: &str, role: Role) -> User {
        let repo = UserRepository::new(pool);
        let new_user = NewUser::new(username, "hash", username);
        let user = repo.create(&new_user).await.unwrap();
        if role != Role::Member {
            let update = crate::db::UserUpdate::new().role(role);
            repo.update(user.id, &update).await.unwrap();
        }
        repo.get_by_id(user.id).await.unwrap().unwrap()
    }

    fn create_admin_user(id: i64, role: Role) -> User {
        User {
            id,
            username: format!("admin{id}"),
            password: "hash".to_string(),
            nickname: format!("Admin {id}"),
            email: None,
            role,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            language: "en".to_string(),
            auto_paging: true,
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        }
    }

    async fn create_test_board(pool: &SqlitePool) -> i64 {
        let repo = BoardRepository::new(pool);
        let board = repo
            .create(&NewBoard::new("test-board").with_board_type(BoardType::Thread))
            .await
            .unwrap();
        board.id
    }

    async fn create_test_thread(pool: &SqlitePool, board_id: i64, author_id: i64) -> i64 {
        let repo = ThreadRepository::new(pool);
        let thread = repo
            .create(&NewThread::new(board_id, "Test Thread", author_id))
            .await
            .unwrap();
        thread.id
    }

    async fn create_test_post(
        pool: &SqlitePool,
        board_id: i64,
        thread_id: i64,
        author_id: i64,
    ) -> i64 {
        let repo = PostRepository::new(pool);
        let post = repo
            .create_thread_post(&NewThreadPost::new(
                board_id,
                thread_id,
                author_id,
                "Test post content",
            ))
            .await
            .unwrap();
        post.id
    }

    #[tokio::test]
    async fn test_get_post() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "author", Role::Member).await;
        let board_id = create_test_board(pool).await;
        let thread_id = create_test_thread(pool, board_id, user.id).await;
        let post_id = create_test_post(pool, board_id, thread_id, user.id).await;

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let post = service.get_post(post_id, &subop).await.unwrap();
        assert_eq!(post.body, "Test post content");
    }

    #[tokio::test]
    async fn test_get_post_as_member_fails() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "author", Role::Member).await;
        let board_id = create_test_board(pool).await;
        let thread_id = create_test_thread(pool, board_id, user.id).await;
        let post_id = create_test_post(pool, board_id, thread_id, user.id).await;

        let service = ContentAdminService::new(pool);
        let member = create_admin_user(100, Role::Member);

        let result = service.get_post(post_id, &member).await;
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[tokio::test]
    async fn test_delete_post_soft() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "author", Role::Member).await;
        let board_id = create_test_board(pool).await;
        let thread_id = create_test_thread(pool, board_id, user.id).await;
        let post_id = create_test_post(pool, board_id, thread_id, user.id).await;

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let deleted = service
            .delete_post(post_id, PostDeletionMode::Soft, &subop)
            .await
            .unwrap();
        assert!(deleted);

        // Post should still exist but with deleted message
        let post = service.get_post(post_id, &subop).await.unwrap();
        assert_eq!(post.body, DELETED_POST_MESSAGE);
    }

    #[tokio::test]
    async fn test_delete_post_hard() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "author", Role::Member).await;
        let board_id = create_test_board(pool).await;
        let thread_id = create_test_thread(pool, board_id, user.id).await;
        let post_id = create_test_post(pool, board_id, thread_id, user.id).await;

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let deleted = service
            .delete_post(post_id, PostDeletionMode::Hard, &subop)
            .await
            .unwrap();
        assert!(deleted);

        // Post should not exist
        let result = service.get_post(post_id, &subop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_post_as_member_fails() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "author", Role::Member).await;
        let board_id = create_test_board(pool).await;
        let thread_id = create_test_thread(pool, board_id, user.id).await;
        let post_id = create_test_post(pool, board_id, thread_id, user.id).await;

        let service = ContentAdminService::new(pool);
        let member = create_admin_user(100, Role::Member);

        let result = service
            .delete_post(post_id, PostDeletionMode::Soft, &member)
            .await;
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_post() {
        let db = setup_db().await;
        let pool = db.pool();
        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let result = service
            .delete_post(999, PostDeletionMode::Soft, &subop)
            .await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_thread() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "author", Role::Member).await;
        let board_id = create_test_board(pool).await;
        let thread_id = create_test_thread(pool, board_id, user.id).await;
        create_test_post(pool, board_id, thread_id, user.id).await;
        create_test_post(pool, board_id, thread_id, user.id).await;

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let deleted = service.delete_thread(thread_id, &subop).await.unwrap();
        assert!(deleted);

        // Thread should not exist
        let thread_repo = ThreadRepository::new(pool);
        assert!(thread_repo.get_by_id(thread_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_thread() {
        let db = setup_db().await;
        let pool = db.pool();
        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let result = service.delete_thread(999, &subop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_posts_by_author() {
        let db = setup_db().await;
        let pool = db.pool();
        let user1 = create_test_user(pool, "author1", Role::Member).await;
        let user2 = create_test_user(pool, "author2", Role::Member).await;
        let board_id = create_test_board(pool).await;
        let thread_id = create_test_thread(pool, board_id, user1.id).await;

        create_test_post(pool, board_id, thread_id, user1.id).await;
        create_test_post(pool, board_id, thread_id, user1.id).await;
        create_test_post(pool, board_id, thread_id, user2.id).await;

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let posts = service.list_posts_by_author(user1.id, &subop).await.unwrap();
        assert_eq!(posts.len(), 2);
    }

    #[tokio::test]
    async fn test_get_file() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "uploader", Role::Member).await;

        // Create a folder and file
        let folder_repo = FolderRepository::new(pool);
        let folder = folder_repo
            .create(&NewFolder::new("Test Folder"))
            .await
            .unwrap();
        let file_repo = FileRepository::new(pool);
        let file = file_repo
            .create(&NewFile::new(folder.id, "test.txt", "stored.txt", 100, user.id))
            .await
            .unwrap();

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let result = service.get_file(file.id, &subop).await.unwrap();
        assert_eq!(result.filename, "test.txt");
    }

    #[tokio::test]
    async fn test_get_file_not_found() {
        let db = setup_db().await;
        let pool = db.pool();
        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let result = service.get_file(999, &subop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_file() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "uploader", Role::Member).await;

        // Create a folder and file
        let folder_repo = FolderRepository::new(pool);
        let folder = folder_repo
            .create(&NewFolder::new("Test Folder"))
            .await
            .unwrap();
        let file_repo = FileRepository::new(pool);
        let file = file_repo
            .create(&NewFile::new(folder.id, "test.txt", "stored.txt", 100, user.id))
            .await
            .unwrap();

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let deleted = service.delete_file(file.id, &subop).await.unwrap();
        assert!(deleted);

        let result = service.get_file(file.id, &subop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_file_as_member_fails() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "uploader", Role::Member).await;

        // Create a folder and file
        let folder_repo = FolderRepository::new(pool);
        let folder = folder_repo
            .create(&NewFolder::new("Test Folder"))
            .await
            .unwrap();
        let file_repo = FileRepository::new(pool);
        let file = file_repo
            .create(&NewFile::new(folder.id, "test.txt", "stored.txt", 100, user.id))
            .await
            .unwrap();

        let service = ContentAdminService::new(pool);
        let member = create_admin_user(100, Role::Member);

        let result = service.delete_file(file.id, &member).await;
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[tokio::test]
    async fn test_list_files_by_uploader() {
        let db = setup_db().await;
        let pool = db.pool();
        let user1 = create_test_user(pool, "uploader1", Role::Member).await;
        let user2 = create_test_user(pool, "uploader2", Role::Member).await;

        let folder_repo = FolderRepository::new(pool);
        let folder = folder_repo
            .create(&NewFolder::new("Test Folder"))
            .await
            .unwrap();

        let file_repo = FileRepository::new(pool);
        file_repo
            .create(&NewFile::new(folder.id, "file1.txt", "stored1.txt", 100, user1.id))
            .await
            .unwrap();
        file_repo
            .create(&NewFile::new(folder.id, "file2.txt", "stored2.txt", 100, user1.id))
            .await
            .unwrap();
        file_repo
            .create(&NewFile::new(folder.id, "file3.txt", "stored3.txt", 100, user2.id))
            .await
            .unwrap();

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let files = service.list_files_by_uploader(user1.id, &subop).await.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn test_list_files_in_folder() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool, "uploader", Role::Member).await;

        let folder_repo = FolderRepository::new(pool);
        let folder1 = folder_repo
            .create(&NewFolder::new("Folder 1"))
            .await
            .unwrap();
        let folder2 = folder_repo
            .create(&NewFolder::new("Folder 2"))
            .await
            .unwrap();

        let file_repo = FileRepository::new(pool);
        file_repo
            .create(&NewFile::new(folder1.id, "file1.txt", "stored1.txt", 100, user.id))
            .await
            .unwrap();
        file_repo
            .create(&NewFile::new(folder1.id, "file2.txt", "stored2.txt", 100, user.id))
            .await
            .unwrap();
        file_repo
            .create(&NewFile::new(folder2.id, "file3.txt", "stored3.txt", 100, user.id))
            .await
            .unwrap();

        let service = ContentAdminService::new(pool);
        let subop = create_admin_user(100, Role::SubOp);

        let files = service.list_files_in_folder(folder1.id, &subop).await.unwrap();
        assert_eq!(files.len(), 2);
    }
}
