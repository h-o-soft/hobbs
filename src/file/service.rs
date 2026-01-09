//! File service for HOBBS.
//!
//! This module provides high-level file operations including:
//! - Upload with permission and size checks
//! - Download with access control
//! - File listing and deletion

use crate::db::{DbPool, Role, User};
use crate::{HobbsError, Result};

use super::folder::FolderRepository;
use super::metadata::{FileMetadata, FileRepository, NewFile};
use super::storage::FileStorage;
use super::{DEFAULT_MAX_FILE_SIZE, MAX_DESCRIPTION_LENGTH, MAX_FILENAME_LENGTH};

/// Request data for file upload.
#[derive(Debug, Clone)]
pub struct UploadRequest {
    /// Folder ID to upload to.
    pub folder_id: i64,
    /// Original filename.
    pub filename: String,
    /// File description (optional).
    pub description: Option<String>,
    /// File content.
    pub content: Vec<u8>,
}

impl UploadRequest {
    /// Create a new upload request.
    pub fn new(folder_id: i64, filename: impl Into<String>, content: Vec<u8>) -> Self {
        Self {
            folder_id,
            filename: filename.into(),
            description: None,
            content,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Result of a file download.
#[derive(Debug)]
pub struct DownloadResult {
    /// File metadata.
    pub metadata: FileMetadata,
    /// File content.
    pub content: Vec<u8>,
}

/// File service for managing file uploads and downloads.
pub struct FileService<'a> {
    pool: &'a DbPool,
    storage: &'a FileStorage,
    max_file_size: u64,
}

impl<'a> FileService<'a> {
    /// Create a new FileService.
    pub fn new(pool: &'a DbPool, storage: &'a FileStorage) -> Self {
        Self {
            pool,
            storage,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        }
    }

    /// Create a new FileService with a custom max file size.
    pub fn with_max_file_size(mut self, max_size: u64) -> Self {
        self.max_file_size = max_size;
        self
    }

    /// Upload a file.
    ///
    /// # Permission Check
    /// User must have role >= folder's upload_perm.
    ///
    /// # Validation
    /// - Filename: max 100 characters
    /// - Description: max 500 characters
    /// - File size: max configured size (default 10MB)
    ///
    /// # Returns
    /// The created file metadata.
    pub async fn upload(&self, request: &UploadRequest, user: &User) -> Result<FileMetadata> {
        // Validate filename length
        if request.filename.chars().count() > MAX_FILENAME_LENGTH {
            return Err(HobbsError::Validation(format!(
                "ファイル名は{MAX_FILENAME_LENGTH}文字以内にしてください"
            )));
        }

        // Validate description length
        if let Some(ref desc) = request.description {
            if desc.chars().count() > MAX_DESCRIPTION_LENGTH {
                return Err(HobbsError::Validation(format!(
                    "説明は{MAX_DESCRIPTION_LENGTH}文字以内にしてください"
                )));
            }
        }

        // Validate file size
        if request.content.len() as u64 > self.max_file_size {
            let max_mb = self.max_file_size / 1024 / 1024;
            return Err(HobbsError::Validation(format!(
                "ファイルサイズが大きすぎます（最大 {max_mb}MB）"
            )));
        }

        // Get folder and check permissions
        let folder_repo = FolderRepository::new(self.pool);
        let folder = folder_repo
            .get_by_id(request.folder_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("フォルダ".to_string()))?;

        // Check upload permission
        if user.role < folder.upload_perm {
            return Err(HobbsError::Permission(
                "このフォルダへのアップロード権限がありません".to_string(),
            ));
        }

        // Save the physical file
        let stored_name = self.storage.save(&request.content, &request.filename)?;

        // Create metadata
        let new_file = if let Some(ref desc) = request.description {
            NewFile::new(
                request.folder_id,
                &request.filename,
                &stored_name,
                request.content.len() as i64,
                user.id,
            )
            .with_description(desc)
        } else {
            NewFile::new(
                request.folder_id,
                &request.filename,
                &stored_name,
                request.content.len() as i64,
                user.id,
            )
        };

        let file_repo = FileRepository::new(self.pool);
        let metadata = file_repo.create(&new_file).await?;

        Ok(metadata)
    }

    /// Download a file.
    ///
    /// # Permission Check
    /// User must have role >= folder's permission (read permission).
    ///
    /// # Side Effects
    /// Increments the download count.
    ///
    /// # Returns
    /// The file metadata and content.
    pub async fn download(&self, file_id: i64, user: &User) -> Result<DownloadResult> {
        // Get file metadata
        let file_repo = FileRepository::new(self.pool);
        let metadata = file_repo
            .get_by_id(file_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("ファイル".to_string()))?;

        // Get folder and check permissions
        let folder_repo = FolderRepository::new(self.pool);
        let folder = folder_repo
            .get_by_id(metadata.folder_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("フォルダ".to_string()))?;

        // Check read permission
        if user.role < folder.permission {
            return Err(HobbsError::Permission(
                "このフォルダの閲覧権限がありません".to_string(),
            ));
        }

        // Load the physical file
        let content = self.storage.load(&metadata.stored_name)?;

        // Increment download count
        file_repo.increment_downloads(file_id).await?;

        Ok(DownloadResult { metadata, content })
    }

    /// Get file metadata without downloading content.
    ///
    /// # Permission Check
    /// User must have role >= folder's permission.
    pub async fn get_file(&self, file_id: i64, user: &User) -> Result<FileMetadata> {
        let file_repo = FileRepository::new(self.pool);
        let metadata = file_repo
            .get_by_id(file_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("ファイル".to_string()))?;

        // Get folder and check permissions
        let folder_repo = FolderRepository::new(self.pool);
        let folder = folder_repo
            .get_by_id(metadata.folder_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("フォルダ".to_string()))?;

        if user.role < folder.permission {
            return Err(HobbsError::Permission(
                "このフォルダの閲覧権限がありません".to_string(),
            ));
        }

        Ok(metadata)
    }

    /// List files in a folder.
    ///
    /// # Permission Check
    /// User must have role >= folder's permission.
    pub async fn list_files(&self, folder_id: i64, user: &User) -> Result<Vec<FileMetadata>> {
        // Get folder and check permissions
        let folder_repo = FolderRepository::new(self.pool);
        let folder = folder_repo
            .get_by_id(folder_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("フォルダ".to_string()))?;

        if user.role < folder.permission {
            return Err(HobbsError::Permission(
                "このフォルダの閲覧権限がありません".to_string(),
            ));
        }

        let file_repo = FileRepository::new(self.pool);
        let files = file_repo.list_by_folder(folder_id).await?;
        Ok(files)
    }

    /// Delete a file.
    ///
    /// # Permission Check
    /// - Uploader can delete their own files
    /// - SubOp or higher can delete any file
    ///
    /// # Returns
    /// `true` if the file was deleted.
    pub async fn delete_file(&self, file_id: i64, user: &User) -> Result<bool> {
        // Get file metadata
        let file_repo = FileRepository::new(self.pool);
        let metadata = file_repo
            .get_by_id(file_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("ファイル".to_string()))?;

        // Check delete permission
        let can_delete = metadata.uploader_id == user.id || user.role >= Role::SubOp;

        if !can_delete {
            return Err(HobbsError::Permission(
                "このファイルを削除する権限がありません".to_string(),
            ));
        }

        // Delete physical file first
        self.storage.delete(&metadata.stored_name)?;

        // Delete metadata
        let deleted = file_repo.delete(file_id).await?;

        Ok(deleted)
    }

    /// List files uploaded by the current user.
    pub async fn list_my_files(&self, user: &User) -> Result<Vec<FileMetadata>> {
        let file_repo = FileRepository::new(self.pool);
        let files = file_repo.list_by_uploader(user.id).await?;
        Ok(files)
    }

    /// Get the storage used by this service.
    pub fn storage(&self) -> &FileStorage {
        self.storage
    }

    /// Get the configured max file size.
    pub fn max_file_size(&self) -> u64 {
        self.max_file_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{NewUser, UserRepository};
    use crate::file::NewFolder;
    use crate::Database;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    async fn setup() -> (Database, TempDir, FileStorage) {
        let db = Database::open_in_memory().await.unwrap();
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path()).unwrap();
        (db, temp_dir, storage)
    }

    async fn create_user(pool: &SqlitePool, username: &str, role: Role) -> User {
        let repo = UserRepository::new(pool);
        let mut new_user = NewUser::new(username, "password123", username);
        new_user.role = role;
        repo.create(&new_user).await.unwrap()
    }

    async fn create_folder(
        pool: &SqlitePool,
        name: &str,
        permission: Role,
        upload_perm: Role,
    ) -> super::super::folder::Folder {
        let repo = FolderRepository::new(pool);
        repo.create(
            &NewFolder::new(name)
                .with_permission(permission)
                .with_upload_perm(upload_perm),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_upload_success() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "uploader", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Guest, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(folder.id, "test.txt", b"Hello, World!".to_vec())
            .with_description("A test file");

        let result = service.upload(&request, &user).await.unwrap();

        assert_eq!(result.filename, "test.txt");
        assert_eq!(result.description, Some("A test file".to_string()));
        assert_eq!(result.size, 13);
        assert_eq!(result.uploader_id, user.id);
        assert_eq!(result.downloads, 0);
    }

    #[tokio::test]
    async fn test_upload_permission_denied() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "guest", Role::Guest).await;
        let folder = create_folder(pool, "Test", Role::Guest, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(folder.id, "test.txt", b"data".to_vec());

        let result = service.upload(&request, &user).await;

        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[tokio::test]
    async fn test_upload_file_too_large() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "uploader", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Guest, Role::Member).await;

        let service = FileService::new(pool, &storage).with_max_file_size(100);
        let request = UploadRequest::new(folder.id, "large.txt", vec![0u8; 200]);

        let result = service.upload(&request, &user).await;

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[tokio::test]
    async fn test_upload_filename_too_long() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "uploader", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Guest, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let long_name = "a".repeat(101);
        let request = UploadRequest::new(folder.id, long_name, b"data".to_vec());

        let result = service.upload(&request, &user).await;

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[tokio::test]
    async fn test_upload_description_too_long() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "uploader", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Guest, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let long_desc = "a".repeat(501);
        let request =
            UploadRequest::new(folder.id, "test.txt", b"data".to_vec()).with_description(long_desc);

        let result = service.upload(&request, &user).await;

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[tokio::test]
    async fn test_upload_folder_not_found() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "uploader", Role::Member).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(9999, "test.txt", b"data".to_vec());

        let result = service.upload(&request, &user).await;

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_download_success() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "user", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let content = b"Download test content".to_vec();
        let request = UploadRequest::new(folder.id, "download.txt", content.clone());
        let uploaded = service.upload(&request, &user).await.unwrap();

        let result = service.download(uploaded.id, &user).await.unwrap();

        assert_eq!(result.content, content);
        assert_eq!(result.metadata.filename, "download.txt");

        // Verify download count was incremented
        let file_repo = FileRepository::new(pool);
        let updated = file_repo.get_by_id(uploaded.id).await.unwrap().unwrap();
        assert_eq!(updated.downloads, 1);
    }

    #[tokio::test]
    async fn test_download_permission_denied() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let uploader = create_user(pool, "uploader", Role::Member).await;
        let guest = create_user(pool, "guest", Role::Guest).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(folder.id, "test.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &uploader).await.unwrap();

        let result = service.download(uploaded.id, &guest).await;

        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[tokio::test]
    async fn test_download_file_not_found() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "user", Role::Member).await;

        let service = FileService::new(pool, &storage);
        let result = service.download(9999, &user).await;

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_file() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "user", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(folder.id, "info.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &user).await.unwrap();

        let result = service.get_file(uploaded.id, &user).await.unwrap();

        assert_eq!(result.id, uploaded.id);
        assert_eq!(result.filename, "info.txt");
    }

    #[tokio::test]
    async fn test_list_files() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "user", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);

        // Upload multiple files
        service
            .upload(
                &UploadRequest::new(folder.id, "file1.txt", b"1".to_vec()),
                &user,
            )
            .await
            .unwrap();
        service
            .upload(
                &UploadRequest::new(folder.id, "file2.txt", b"2".to_vec()),
                &user,
            )
            .await
            .unwrap();

        let files = service.list_files(folder.id, &user).await.unwrap();

        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn test_list_files_permission_denied() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let guest = create_user(pool, "guest", Role::Guest).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let result = service.list_files(folder.id, &guest).await;

        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[tokio::test]
    async fn test_delete_by_uploader() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "uploader", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(folder.id, "delete.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &user).await.unwrap();

        let deleted = service.delete_file(uploaded.id, &user).await.unwrap();

        assert!(deleted);

        // Verify file is gone
        let file_repo = FileRepository::new(pool);
        let result = file_repo.get_by_id(uploaded.id).await.unwrap();
        assert!(result.is_none());

        // Verify physical file is gone
        assert!(!storage.exists(&uploaded.stored_name));
    }

    #[tokio::test]
    async fn test_delete_by_subop() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let uploader = create_user(pool, "uploader", Role::Member).await;
        let subop = create_user(pool, "subop", Role::SubOp).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(folder.id, "delete.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &uploader).await.unwrap();

        // SubOp can delete other's files
        let deleted = service.delete_file(uploaded.id, &subop).await.unwrap();

        assert!(deleted);
    }

    #[tokio::test]
    async fn test_delete_permission_denied() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let uploader = create_user(pool, "uploader", Role::Member).await;
        let other = create_user(pool, "other", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(folder.id, "delete.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &uploader).await.unwrap();

        // Other member cannot delete
        let result = service.delete_file(uploaded.id, &other).await;

        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[tokio::test]
    async fn test_delete_file_not_found() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user = create_user(pool, "user", Role::Member).await;

        let service = FileService::new(pool, &storage);
        let result = service.delete_file(9999, &user).await;

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_my_files() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let user1 = create_user(pool, "user1", Role::Member).await;
        let user2 = create_user(pool, "user2", Role::Member).await;
        let folder = create_folder(pool, "Test", Role::Member, Role::Member).await;

        let service = FileService::new(pool, &storage);

        // Upload files from different users
        service
            .upload(
                &UploadRequest::new(folder.id, "user1_file.txt", b"1".to_vec()),
                &user1,
            )
            .await
            .unwrap();
        service
            .upload(
                &UploadRequest::new(folder.id, "user2_file.txt", b"2".to_vec()),
                &user2,
            )
            .await
            .unwrap();

        let user1_files = service.list_my_files(&user1).await.unwrap();
        let user2_files = service.list_my_files(&user2).await.unwrap();

        assert_eq!(user1_files.len(), 1);
        assert_eq!(user2_files.len(), 1);
        assert_eq!(user1_files[0].filename, "user1_file.txt");
        assert_eq!(user2_files[0].filename, "user2_file.txt");
    }

    #[tokio::test]
    async fn test_sysop_can_upload_anywhere() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let sysop = create_user(pool, "sysop", Role::SysOp).await;
        let folder = create_folder(pool, "Restricted", Role::SysOp, Role::SysOp).await;

        let service = FileService::new(pool, &storage);
        let request = UploadRequest::new(folder.id, "test.txt", b"data".to_vec());

        let result = service.upload(&request, &sysop).await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_upload_request_builder() {
        let request =
            UploadRequest::new(1, "test.txt", b"data".to_vec()).with_description("Description");

        assert_eq!(request.folder_id, 1);
        assert_eq!(request.filename, "test.txt");
        assert_eq!(request.description, Some("Description".to_string()));
        assert_eq!(request.content, b"data".to_vec());
    }

    #[tokio::test]
    async fn test_with_max_file_size() {
        let (db, _temp_dir, storage) = setup().await;
        let pool = db.pool();
        let service = FileService::new(pool, &storage).with_max_file_size(1024);

        assert_eq!(service.max_file_size(), 1024);
    }
}
