//! Content management for administrators.
//!
//! This module provides administrative functions for managing content:
//! - Delete posts (SubOp and above)
//! - Delete files (SubOp and above)
//! - Soft delete posts (replace body with deletion message)

use crate::board::{Post, PostRepository, PostUpdate, ThreadRepository};
use crate::db::{Database, User};
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
    db: &'a Database,
    storage: Option<&'a FileStorage>,
}

impl<'a> ContentAdminService<'a> {
    /// Create a new ContentAdminService.
    pub fn new(db: &'a Database) -> Self {
        Self { db, storage: None }
    }

    /// Create a new ContentAdminService with file storage.
    pub fn with_storage(db: &'a Database, storage: &'a FileStorage) -> Self {
        Self {
            db,
            storage: Some(storage),
        }
    }

    /// Get a post by ID.
    ///
    /// Requires SubOp or higher permission.
    pub fn get_post(&self, post_id: i64, admin: &User) -> Result<Post, AdminError> {
        require_admin(Some(admin))?;

        let repo = PostRepository::new(self.db);
        let post = repo
            .get_by_id(post_id)?
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
    pub fn delete_post(
        &self,
        post_id: i64,
        mode: PostDeletionMode,
        admin: &User,
    ) -> Result<bool, AdminError> {
        require_admin(Some(admin))?;

        let repo = PostRepository::new(self.db);

        // Check if post exists
        repo.get_by_id(post_id)?
            .ok_or_else(|| AdminError::NotFound("投稿".to_string()))?;

        match mode {
            PostDeletionMode::Soft => {
                let update = PostUpdate::new().body(DELETED_POST_MESSAGE);
                repo.update(post_id, &update)?;
                Ok(true)
            }
            PostDeletionMode::Hard => {
                let deleted = repo.delete(post_id)?;
                Ok(deleted)
            }
        }
    }

    /// Delete a thread and all its posts.
    ///
    /// Requires SubOp or higher permission.
    pub fn delete_thread(&self, thread_id: i64, admin: &User) -> Result<bool, AdminError> {
        require_admin(Some(admin))?;

        let repo = ThreadRepository::new(self.db);

        // Check if thread exists
        repo.get_by_id(thread_id)?
            .ok_or_else(|| AdminError::NotFound("スレッド".to_string()))?;

        // Delete the thread (cascade will delete posts)
        let deleted = repo.delete(thread_id)?;
        Ok(deleted)
    }

    /// List posts by author.
    ///
    /// Requires SubOp or higher permission.
    pub fn list_posts_by_author(
        &self,
        author_id: i64,
        admin: &User,
    ) -> Result<Vec<Post>, AdminError> {
        require_admin(Some(admin))?;

        let repo = PostRepository::new(self.db);
        let posts = repo.list_by_author(author_id)?;
        Ok(posts)
    }

    /// Get a file by ID.
    ///
    /// Requires SubOp or higher permission.
    pub fn get_file(&self, file_id: i64, admin: &User) -> Result<FileMetadata, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();
        let file = FileRepository::get_by_id(conn, file_id)?
            .ok_or_else(|| AdminError::NotFound("ファイル".to_string()))?;

        Ok(file)
    }

    /// Delete a file.
    ///
    /// Requires SubOp or higher permission.
    /// Deletes both the database record and the physical file.
    pub fn delete_file(&self, file_id: i64, admin: &User) -> Result<bool, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();

        // Get file info first
        let file = FileRepository::get_by_id(conn, file_id)?
            .ok_or_else(|| AdminError::NotFound("ファイル".to_string()))?;

        // Delete from database
        let deleted = FileRepository::delete(conn, file_id)?;

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
    pub fn list_files_by_uploader(
        &self,
        uploader_id: i64,
        admin: &User,
    ) -> Result<Vec<FileMetadata>, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();
        let files = FileRepository::list_by_uploader(conn, uploader_id)?;
        Ok(files)
    }

    /// List files in a folder.
    ///
    /// Requires SubOp or higher permission.
    pub fn list_files_in_folder(
        &self,
        folder_id: i64,
        admin: &User,
    ) -> Result<Vec<FileMetadata>, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();
        let files = FileRepository::list_by_folder(conn, folder_id)?;
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardRepository, BoardType, NewBoard, NewThread, NewThreadPost};
    use crate::db::{NewUser, Role, UserRepository};
    use crate::file::{NewFile, NewFolder};
    use crate::server::CharacterEncoding;

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_user(db: &Database, username: &str, role: Role) -> User {
        let repo = UserRepository::new(db);
        let new_user = NewUser::new(username, "hash", username);
        let user = repo.create(&new_user).unwrap();
        if role != Role::Member {
            let update = crate::db::UserUpdate::new().role(role);
            repo.update(user.id, &update).unwrap();
        }
        repo.get_by_id(user.id).unwrap().unwrap()
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

    fn create_test_board(db: &Database) -> i64 {
        let repo = BoardRepository::new(db);
        let board = repo
            .create(&NewBoard::new("test-board").with_board_type(BoardType::Thread))
            .unwrap();
        board.id
    }

    fn create_test_thread(db: &Database, board_id: i64, author_id: i64) -> i64 {
        let repo = ThreadRepository::new(db);
        let thread = repo
            .create(&NewThread::new(board_id, "Test Thread", author_id))
            .unwrap();
        thread.id
    }

    fn create_test_post(db: &Database, board_id: i64, thread_id: i64, author_id: i64) -> i64 {
        let repo = PostRepository::new(db);
        let post = repo
            .create_thread_post(&NewThreadPost::new(
                board_id,
                thread_id,
                author_id,
                "Test post content",
            ))
            .unwrap();
        post.id
    }

    #[test]
    fn test_get_post() {
        let db = setup_db();
        let user = create_test_user(&db, "author", Role::Member);
        let board_id = create_test_board(&db);
        let thread_id = create_test_thread(&db, board_id, user.id);
        let post_id = create_test_post(&db, board_id, thread_id, user.id);

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let post = service.get_post(post_id, &subop).unwrap();
        assert_eq!(post.body, "Test post content");
    }

    #[test]
    fn test_get_post_as_member_fails() {
        let db = setup_db();
        let user = create_test_user(&db, "author", Role::Member);
        let board_id = create_test_board(&db);
        let thread_id = create_test_thread(&db, board_id, user.id);
        let post_id = create_test_post(&db, board_id, thread_id, user.id);

        let service = ContentAdminService::new(&db);
        let member = create_admin_user(100, Role::Member);

        let result = service.get_post(post_id, &member);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_delete_post_soft() {
        let db = setup_db();
        let user = create_test_user(&db, "author", Role::Member);
        let board_id = create_test_board(&db);
        let thread_id = create_test_thread(&db, board_id, user.id);
        let post_id = create_test_post(&db, board_id, thread_id, user.id);

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let deleted = service
            .delete_post(post_id, PostDeletionMode::Soft, &subop)
            .unwrap();
        assert!(deleted);

        // Post should still exist but with deleted message
        let post = service.get_post(post_id, &subop).unwrap();
        assert_eq!(post.body, DELETED_POST_MESSAGE);
    }

    #[test]
    fn test_delete_post_hard() {
        let db = setup_db();
        let user = create_test_user(&db, "author", Role::Member);
        let board_id = create_test_board(&db);
        let thread_id = create_test_thread(&db, board_id, user.id);
        let post_id = create_test_post(&db, board_id, thread_id, user.id);

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let deleted = service
            .delete_post(post_id, PostDeletionMode::Hard, &subop)
            .unwrap();
        assert!(deleted);

        // Post should not exist
        let result = service.get_post(post_id, &subop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_delete_post_as_member_fails() {
        let db = setup_db();
        let user = create_test_user(&db, "author", Role::Member);
        let board_id = create_test_board(&db);
        let thread_id = create_test_thread(&db, board_id, user.id);
        let post_id = create_test_post(&db, board_id, thread_id, user.id);

        let service = ContentAdminService::new(&db);
        let member = create_admin_user(100, Role::Member);

        let result = service.delete_post(post_id, PostDeletionMode::Soft, &member);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_delete_nonexistent_post() {
        let db = setup_db();
        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let result = service.delete_post(999, PostDeletionMode::Soft, &subop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_delete_thread() {
        let db = setup_db();
        let user = create_test_user(&db, "author", Role::Member);
        let board_id = create_test_board(&db);
        let thread_id = create_test_thread(&db, board_id, user.id);
        create_test_post(&db, board_id, thread_id, user.id);
        create_test_post(&db, board_id, thread_id, user.id);

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let deleted = service.delete_thread(thread_id, &subop).unwrap();
        assert!(deleted);

        // Thread should not exist
        let thread_repo = ThreadRepository::new(&db);
        assert!(thread_repo.get_by_id(thread_id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_thread() {
        let db = setup_db();
        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let result = service.delete_thread(999, &subop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_list_posts_by_author() {
        let db = setup_db();
        let user1 = create_test_user(&db, "author1", Role::Member);
        let user2 = create_test_user(&db, "author2", Role::Member);
        let board_id = create_test_board(&db);
        let thread_id = create_test_thread(&db, board_id, user1.id);

        create_test_post(&db, board_id, thread_id, user1.id);
        create_test_post(&db, board_id, thread_id, user1.id);
        create_test_post(&db, board_id, thread_id, user2.id);

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let posts = service.list_posts_by_author(user1.id, &subop).unwrap();
        assert_eq!(posts.len(), 2);
    }

    #[test]
    fn test_get_file() {
        let db = setup_db();
        let user = create_test_user(&db, "uploader", Role::Member);

        // Create a folder and file
        let conn = db.conn();
        let folder =
            crate::file::FolderRepository::create(conn, &NewFolder::new("Test Folder")).unwrap();
        let file = FileRepository::create(
            conn,
            &NewFile::new(folder.id, "test.txt", "stored.txt", 100, user.id),
        )
        .unwrap();

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let result = service.get_file(file.id, &subop).unwrap();
        assert_eq!(result.filename, "test.txt");
    }

    #[test]
    fn test_get_file_not_found() {
        let db = setup_db();
        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let result = service.get_file(999, &subop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_delete_file() {
        let db = setup_db();
        let user = create_test_user(&db, "uploader", Role::Member);

        // Create a folder and file
        let conn = db.conn();
        let folder =
            crate::file::FolderRepository::create(conn, &NewFolder::new("Test Folder")).unwrap();
        let file = FileRepository::create(
            conn,
            &NewFile::new(folder.id, "test.txt", "stored.txt", 100, user.id),
        )
        .unwrap();

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let deleted = service.delete_file(file.id, &subop).unwrap();
        assert!(deleted);

        let result = service.get_file(file.id, &subop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_delete_file_as_member_fails() {
        let db = setup_db();
        let user = create_test_user(&db, "uploader", Role::Member);

        // Create a folder and file
        let conn = db.conn();
        let folder =
            crate::file::FolderRepository::create(conn, &NewFolder::new("Test Folder")).unwrap();
        let file = FileRepository::create(
            conn,
            &NewFile::new(folder.id, "test.txt", "stored.txt", 100, user.id),
        )
        .unwrap();

        let service = ContentAdminService::new(&db);
        let member = create_admin_user(100, Role::Member);

        let result = service.delete_file(file.id, &member);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_list_files_by_uploader() {
        let db = setup_db();
        let user1 = create_test_user(&db, "uploader1", Role::Member);
        let user2 = create_test_user(&db, "uploader2", Role::Member);

        let conn = db.conn();
        let folder =
            crate::file::FolderRepository::create(conn, &NewFolder::new("Test Folder")).unwrap();

        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file1.txt", "stored1.txt", 100, user1.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file2.txt", "stored2.txt", 100, user1.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file3.txt", "stored3.txt", 100, user2.id),
        )
        .unwrap();

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let files = service.list_files_by_uploader(user1.id, &subop).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_list_files_in_folder() {
        let db = setup_db();
        let user = create_test_user(&db, "uploader", Role::Member);

        let conn = db.conn();
        let folder1 =
            crate::file::FolderRepository::create(conn, &NewFolder::new("Folder 1")).unwrap();
        let folder2 =
            crate::file::FolderRepository::create(conn, &NewFolder::new("Folder 2")).unwrap();

        FileRepository::create(
            conn,
            &NewFile::new(folder1.id, "file1.txt", "stored1.txt", 100, user.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder1.id, "file2.txt", "stored2.txt", 100, user.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder2.id, "file3.txt", "stored3.txt", 100, user.id),
        )
        .unwrap();

        let service = ContentAdminService::new(&db);
        let subop = create_admin_user(100, Role::SubOp);

        let files = service.list_files_in_folder(folder1.id, &subop).unwrap();
        assert_eq!(files.len(), 2);
    }
}
