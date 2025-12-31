//! File service for HOBBS.
//!
//! This module provides high-level file operations including:
//! - Upload with permission and size checks
//! - Download with access control
//! - File listing and deletion

use crate::db::{Database, Role, User};
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
    db: &'a Database,
    storage: &'a FileStorage,
    max_file_size: u64,
}

impl<'a> FileService<'a> {
    /// Create a new FileService.
    pub fn new(db: &'a Database, storage: &'a FileStorage) -> Self {
        Self {
            db,
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
    pub fn upload(&self, request: &UploadRequest, user: &User) -> Result<FileMetadata> {
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
        let folder = FolderRepository::get_by_id(self.db.conn(), request.folder_id)?
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

        let metadata = FileRepository::create(self.db.conn(), &new_file)?;

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
    pub fn download(&self, file_id: i64, user: &User) -> Result<DownloadResult> {
        // Get file metadata
        let metadata = FileRepository::get_by_id(self.db.conn(), file_id)?
            .ok_or_else(|| HobbsError::NotFound("ファイル".to_string()))?;

        // Get folder and check permissions
        let folder = FolderRepository::get_by_id(self.db.conn(), metadata.folder_id)?
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
        FileRepository::increment_downloads(self.db.conn(), file_id)?;

        Ok(DownloadResult { metadata, content })
    }

    /// Get file metadata without downloading content.
    ///
    /// # Permission Check
    /// User must have role >= folder's permission.
    pub fn get_file(&self, file_id: i64, user: &User) -> Result<FileMetadata> {
        let metadata = FileRepository::get_by_id(self.db.conn(), file_id)?
            .ok_or_else(|| HobbsError::NotFound("ファイル".to_string()))?;

        // Get folder and check permissions
        let folder = FolderRepository::get_by_id(self.db.conn(), metadata.folder_id)?
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
    pub fn list_files(&self, folder_id: i64, user: &User) -> Result<Vec<FileMetadata>> {
        // Get folder and check permissions
        let folder = FolderRepository::get_by_id(self.db.conn(), folder_id)?
            .ok_or_else(|| HobbsError::NotFound("フォルダ".to_string()))?;

        if user.role < folder.permission {
            return Err(HobbsError::Permission(
                "このフォルダの閲覧権限がありません".to_string(),
            ));
        }

        let files = FileRepository::list_by_folder(self.db.conn(), folder_id)?;
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
    pub fn delete_file(&self, file_id: i64, user: &User) -> Result<bool> {
        // Get file metadata
        let metadata = FileRepository::get_by_id(self.db.conn(), file_id)?
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
        let deleted = FileRepository::delete(self.db.conn(), file_id)?;

        Ok(deleted)
    }

    /// List files uploaded by the current user.
    pub fn list_my_files(&self, user: &User) -> Result<Vec<FileMetadata>> {
        let files = FileRepository::list_by_uploader(self.db.conn(), user.id)?;
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
    use crate::file::{FolderRepository, NewFolder};
    use tempfile::TempDir;

    fn setup() -> (Database, TempDir, FileStorage) {
        let db = Database::open_in_memory().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path()).unwrap();
        (db, temp_dir, storage)
    }

    fn create_user(db: &Database, username: &str, role: Role) -> User {
        let repo = UserRepository::new(db);
        let mut new_user = NewUser::new(username, "password123", username);
        new_user.role = role;
        repo.create(&new_user).unwrap()
    }

    fn create_folder(
        db: &Database,
        name: &str,
        permission: Role,
        upload_perm: Role,
    ) -> super::super::folder::Folder {
        FolderRepository::create(
            db.conn(),
            &NewFolder::new(name)
                .with_permission(permission)
                .with_upload_perm(upload_perm),
        )
        .unwrap()
    }

    #[test]
    fn test_upload_success() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "uploader", Role::Member);
        let folder = create_folder(&db, "Test", Role::Guest, Role::Member);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(folder.id, "test.txt", b"Hello, World!".to_vec())
            .with_description("A test file");

        let result = service.upload(&request, &user).unwrap();

        assert_eq!(result.filename, "test.txt");
        assert_eq!(result.description, Some("A test file".to_string()));
        assert_eq!(result.size, 13);
        assert_eq!(result.uploader_id, user.id);
        assert_eq!(result.downloads, 0);
    }

    #[test]
    fn test_upload_permission_denied() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "guest", Role::Guest);
        let folder = create_folder(&db, "Test", Role::Guest, Role::Member);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(folder.id, "test.txt", b"data".to_vec());

        let result = service.upload(&request, &user);

        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[test]
    fn test_upload_file_too_large() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "uploader", Role::Member);
        let folder = create_folder(&db, "Test", Role::Guest, Role::Member);

        let service = FileService::new(&db, &storage).with_max_file_size(100);
        let request = UploadRequest::new(folder.id, "large.txt", vec![0u8; 200]);

        let result = service.upload(&request, &user);

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[test]
    fn test_upload_filename_too_long() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "uploader", Role::Member);
        let folder = create_folder(&db, "Test", Role::Guest, Role::Member);

        let service = FileService::new(&db, &storage);
        let long_name = "a".repeat(101);
        let request = UploadRequest::new(folder.id, long_name, b"data".to_vec());

        let result = service.upload(&request, &user);

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[test]
    fn test_upload_description_too_long() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "uploader", Role::Member);
        let folder = create_folder(&db, "Test", Role::Guest, Role::Member);

        let service = FileService::new(&db, &storage);
        let long_desc = "a".repeat(501);
        let request =
            UploadRequest::new(folder.id, "test.txt", b"data".to_vec()).with_description(long_desc);

        let result = service.upload(&request, &user);

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[test]
    fn test_upload_folder_not_found() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "uploader", Role::Member);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(9999, "test.txt", b"data".to_vec());

        let result = service.upload(&request, &user);

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_download_success() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "user", Role::Member);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);
        let content = b"Download test content".to_vec();
        let request = UploadRequest::new(folder.id, "download.txt", content.clone());
        let uploaded = service.upload(&request, &user).unwrap();

        let result = service.download(uploaded.id, &user).unwrap();

        assert_eq!(result.content, content);
        assert_eq!(result.metadata.filename, "download.txt");

        // Verify download count was incremented
        let updated = FileRepository::get_by_id(db.conn(), uploaded.id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.downloads, 1);
    }

    #[test]
    fn test_download_permission_denied() {
        let (db, _temp_dir, storage) = setup();
        let uploader = create_user(&db, "uploader", Role::Member);
        let guest = create_user(&db, "guest", Role::Guest);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(folder.id, "test.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &uploader).unwrap();

        let result = service.download(uploaded.id, &guest);

        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[test]
    fn test_download_file_not_found() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "user", Role::Member);

        let service = FileService::new(&db, &storage);
        let result = service.download(9999, &user);

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_get_file() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "user", Role::Member);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(folder.id, "info.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &user).unwrap();

        let result = service.get_file(uploaded.id, &user).unwrap();

        assert_eq!(result.id, uploaded.id);
        assert_eq!(result.filename, "info.txt");
    }

    #[test]
    fn test_list_files() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "user", Role::Member);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);

        // Upload multiple files
        service
            .upload(
                &UploadRequest::new(folder.id, "file1.txt", b"1".to_vec()),
                &user,
            )
            .unwrap();
        service
            .upload(
                &UploadRequest::new(folder.id, "file2.txt", b"2".to_vec()),
                &user,
            )
            .unwrap();

        let files = service.list_files(folder.id, &user).unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_list_files_permission_denied() {
        let (db, _temp_dir, storage) = setup();
        let guest = create_user(&db, "guest", Role::Guest);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);
        let result = service.list_files(folder.id, &guest);

        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[test]
    fn test_delete_by_uploader() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "uploader", Role::Member);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(folder.id, "delete.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &user).unwrap();

        let deleted = service.delete_file(uploaded.id, &user).unwrap();

        assert!(deleted);

        // Verify file is gone
        let result = FileRepository::get_by_id(db.conn(), uploaded.id).unwrap();
        assert!(result.is_none());

        // Verify physical file is gone
        assert!(!storage.exists(&uploaded.stored_name));
    }

    #[test]
    fn test_delete_by_subop() {
        let (db, _temp_dir, storage) = setup();
        let uploader = create_user(&db, "uploader", Role::Member);
        let subop = create_user(&db, "subop", Role::SubOp);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(folder.id, "delete.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &uploader).unwrap();

        // SubOp can delete other's files
        let deleted = service.delete_file(uploaded.id, &subop).unwrap();

        assert!(deleted);
    }

    #[test]
    fn test_delete_permission_denied() {
        let (db, _temp_dir, storage) = setup();
        let uploader = create_user(&db, "uploader", Role::Member);
        let other = create_user(&db, "other", Role::Member);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(folder.id, "delete.txt", b"data".to_vec());
        let uploaded = service.upload(&request, &uploader).unwrap();

        // Other member cannot delete
        let result = service.delete_file(uploaded.id, &other);

        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[test]
    fn test_delete_file_not_found() {
        let (db, _temp_dir, storage) = setup();
        let user = create_user(&db, "user", Role::Member);

        let service = FileService::new(&db, &storage);
        let result = service.delete_file(9999, &user);

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_list_my_files() {
        let (db, _temp_dir, storage) = setup();
        let user1 = create_user(&db, "user1", Role::Member);
        let user2 = create_user(&db, "user2", Role::Member);
        let folder = create_folder(&db, "Test", Role::Member, Role::Member);

        let service = FileService::new(&db, &storage);

        // Upload files from different users
        service
            .upload(
                &UploadRequest::new(folder.id, "user1_file.txt", b"1".to_vec()),
                &user1,
            )
            .unwrap();
        service
            .upload(
                &UploadRequest::new(folder.id, "user2_file.txt", b"2".to_vec()),
                &user2,
            )
            .unwrap();

        let user1_files = service.list_my_files(&user1).unwrap();
        let user2_files = service.list_my_files(&user2).unwrap();

        assert_eq!(user1_files.len(), 1);
        assert_eq!(user2_files.len(), 1);
        assert_eq!(user1_files[0].filename, "user1_file.txt");
        assert_eq!(user2_files[0].filename, "user2_file.txt");
    }

    #[test]
    fn test_sysop_can_upload_anywhere() {
        let (db, _temp_dir, storage) = setup();
        let sysop = create_user(&db, "sysop", Role::SysOp);
        let folder = create_folder(&db, "Restricted", Role::SysOp, Role::SysOp);

        let service = FileService::new(&db, &storage);
        let request = UploadRequest::new(folder.id, "test.txt", b"data".to_vec());

        let result = service.upload(&request, &sysop);

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

    #[test]
    fn test_with_max_file_size() {
        let (db, _temp_dir, storage) = setup();
        let service = FileService::new(&db, &storage).with_max_file_size(1024);

        assert_eq!(service.max_file_size(), 1024);
    }
}
