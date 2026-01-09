//! File metadata types and repository for HOBBS file management.

use chrono::{DateTime, Utc};
use sqlx::QueryBuilder;

use crate::db::DbPool;
use crate::Result;

/// Metadata for a file in the file library.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FileMetadata {
    /// Unique file ID.
    pub id: i64,
    /// Folder ID this file belongs to.
    pub folder_id: i64,
    /// Original filename (display name).
    pub filename: String,
    /// Stored filename (UUID.ext format).
    pub stored_name: String,
    /// File size in bytes.
    pub size: i64,
    /// File description.
    pub description: Option<String>,
    /// User ID of the uploader.
    pub uploader_id: i64,
    /// Number of times downloaded.
    pub downloads: i64,
    /// When the file was uploaded.
    pub created_at: String,
}

impl FileMetadata {
    /// Get the created_at as DateTime<Utc>.
    pub fn created_at_datetime(&self) -> DateTime<Utc> {
        use chrono::NaiveDateTime;

        // Try RFC3339 first, then SQLite format (YYYY-MM-DD HH:MM:SS)
        DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&self.created_at, "%Y-%m-%d %H:%M:%S")
                    .map(|naive| naive.and_utc())
            })
            .unwrap_or_else(|_| Utc::now())
    }
}

/// Data for creating a new file entry.
#[derive(Debug, Clone)]
pub struct NewFile {
    /// Folder ID this file belongs to.
    pub folder_id: i64,
    /// Original filename (display name).
    pub filename: String,
    /// Stored filename (UUID.ext format).
    pub stored_name: String,
    /// File size in bytes.
    pub size: i64,
    /// File description.
    pub description: Option<String>,
    /// User ID of the uploader.
    pub uploader_id: i64,
}

impl NewFile {
    /// Create a new NewFile.
    pub fn new(
        folder_id: i64,
        filename: impl Into<String>,
        stored_name: impl Into<String>,
        size: i64,
        uploader_id: i64,
    ) -> Self {
        Self {
            folder_id,
            filename: filename.into(),
            stored_name: stored_name.into(),
            size,
            description: None,
            uploader_id,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Builder for updating file metadata.
#[derive(Debug, Clone, Default)]
pub struct FileUpdate {
    /// New filename.
    pub filename: Option<String>,
    /// New description.
    pub description: Option<Option<String>>,
    /// New download count.
    pub downloads: Option<i64>,
}

impl FileUpdate {
    /// Create a new FileUpdate.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the filename.
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = Some(description.map(|s| s.into()));
        self
    }

    /// Set the download count.
    pub fn downloads(mut self, downloads: i64) -> Self {
        self.downloads = Some(downloads);
        self
    }

    /// Check if the update is empty.
    pub fn is_empty(&self) -> bool {
        self.filename.is_none() && self.description.is_none() && self.downloads.is_none()
    }
}

/// Repository for file metadata operations.
pub struct FileRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> FileRepository<'a> {
    /// Create a new FileRepository with the given database pool reference.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new file entry.
    pub async fn create(&self, file: &NewFile) -> Result<FileMetadata> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO files (folder_id, filename, stored_name, size, description, uploader_id)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id",
        )
        .bind(file.folder_id)
        .bind(&file.filename)
        .bind(&file.stored_name)
        .bind(file.size)
        .bind(&file.description)
        .bind(file.uploader_id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| crate::HobbsError::NotFound("file".to_string()))
    }

    /// Get a file by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<FileMetadata>> {
        let file = sqlx::query_as::<_, FileMetadata>(
            "SELECT id, folder_id, filename, stored_name, size, description, uploader_id, downloads, created_at
             FROM files WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(file)
    }

    /// Get a file by stored name.
    pub async fn get_by_stored_name(&self, stored_name: &str) -> Result<Option<FileMetadata>> {
        let file = sqlx::query_as::<_, FileMetadata>(
            "SELECT id, folder_id, filename, stored_name, size, description, uploader_id, downloads, created_at
             FROM files WHERE stored_name = $1",
        )
        .bind(stored_name)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(file)
    }

    /// List files in a folder (ordered by created_at descending).
    pub async fn list_by_folder(&self, folder_id: i64) -> Result<Vec<FileMetadata>> {
        let files = sqlx::query_as::<_, FileMetadata>(
            "SELECT id, folder_id, filename, stored_name, size, description, uploader_id, downloads, created_at
             FROM files WHERE folder_id = $1 ORDER BY created_at DESC, id DESC",
        )
        .bind(folder_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(files)
    }

    /// List files uploaded by a user.
    pub async fn list_by_uploader(&self, uploader_id: i64) -> Result<Vec<FileMetadata>> {
        let files = sqlx::query_as::<_, FileMetadata>(
            "SELECT id, folder_id, filename, stored_name, size, description, uploader_id, downloads, created_at
             FROM files WHERE uploader_id = $1 ORDER BY created_at DESC, id DESC",
        )
        .bind(uploader_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(files)
    }

    /// Update file metadata.
    pub async fn update(&self, id: i64, update: &FileUpdate) -> Result<Option<FileMetadata>> {
        if update.is_empty() {
            return self.get_by_id(id).await;
        }

        #[cfg(feature = "sqlite")]
        let mut query: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("UPDATE files SET ");
        #[cfg(feature = "postgres")]
        let mut query: QueryBuilder<sqlx::Postgres> = QueryBuilder::new("UPDATE files SET ");
        let mut separated = query.separated(", ");

        if let Some(ref filename) = update.filename {
            separated.push("filename = ");
            separated.push_bind_unseparated(filename);
        }

        if let Some(ref description) = update.description {
            separated.push("description = ");
            separated.push_bind_unseparated(description.clone());
        }

        if let Some(downloads) = update.downloads {
            separated.push("downloads = ");
            separated.push_bind_unseparated(downloads);
        }

        query.push(" WHERE id = ");
        query.push_bind(id);

        let result = query
            .build()
            .execute(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_by_id(id).await
    }

    /// Increment the download count for a file.
    pub async fn increment_downloads(&self, id: i64) -> Result<i64> {
        sqlx::query("UPDATE files SET downloads = downloads + 1 WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        let downloads: (i64,) = sqlx::query_as("SELECT downloads FROM files WHERE id = $1")
            .bind(id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(downloads.0)
    }

    /// Delete a file by ID.
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM files WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Count files in a folder.
    pub async fn count_by_folder(&self, folder_id: i64) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files WHERE folder_id = $1")
            .bind(folder_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }

    /// Get total size of files in a folder.
    pub async fn total_size_by_folder(&self, folder_id: i64) -> Result<i64> {
        let size: (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(size), 0) FROM files WHERE folder_id = $1")
                .bind(folder_id)
                .fetch_one(self.pool)
                .await
                .map_err(|e| crate::HobbsError::Database(e.to_string()))?;

        Ok(size.0)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::db::{NewUser, UserRepository};
    use crate::file::{FolderRepository, NewFolder};
    use crate::Database;
    use sqlx::SqlitePool;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_test_user(pool: &SqlitePool) -> crate::db::User {
        let repo = UserRepository::new(pool);
        let user = NewUser::new("testuser", "password123", "Test User");
        repo.create(&user).await.unwrap()
    }

    async fn create_test_folder(pool: &SqlitePool) -> super::super::folder::Folder {
        let repo = FolderRepository::new(pool);
        repo.create(&NewFolder::new("Test Folder")).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_file() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder = create_test_folder(pool).await;

        let new_file = NewFile::new(
            folder.id,
            "test.txt",
            "abc12345-1234-5678-90ab-cdef12345678.txt",
            1024,
            user.id,
        )
        .with_description("Test file");

        let repo = FileRepository::new(pool);
        let file = repo.create(&new_file).await.unwrap();

        assert_eq!(file.folder_id, folder.id);
        assert_eq!(file.filename, "test.txt");
        assert_eq!(file.stored_name, "abc12345-1234-5678-90ab-cdef12345678.txt");
        assert_eq!(file.size, 1024);
        assert_eq!(file.description, Some("Test file".to_string()));
        assert_eq!(file.uploader_id, user.id);
        assert_eq!(file.downloads, 0);
    }

    #[tokio::test]
    async fn test_get_file_by_id() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder = create_test_folder(pool).await;

        let repo = FileRepository::new(pool);
        let new_file = NewFile::new(folder.id, "file.txt", "stored.txt", 100, user.id);
        let created = repo.create(&new_file).await.unwrap();

        let found = repo.get_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().filename, "file.txt");
    }

    #[tokio::test]
    async fn test_get_file_not_found() {
        let db = setup_db().await;
        let pool = db.pool();

        let repo = FileRepository::new(pool);
        let found = repo.get_by_id(9999).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_by_stored_name() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder = create_test_folder(pool).await;

        let stored_name = "unique-stored-name.txt";
        let repo = FileRepository::new(pool);
        let new_file = NewFile::new(folder.id, "file.txt", stored_name, 100, user.id);
        repo.create(&new_file).await.unwrap();

        let found = repo.get_by_stored_name(stored_name).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().stored_name, stored_name);
    }

    #[tokio::test]
    async fn test_list_by_folder() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder_repo = FolderRepository::new(pool);
        let folder1 = folder_repo
            .create(&NewFolder::new("Folder 1"))
            .await
            .unwrap();
        let folder2 = folder_repo.create(&NewFolder::new("Other")).await.unwrap();

        let repo = FileRepository::new(pool);
        repo.create(&NewFile::new(
            folder1.id,
            "file1.txt",
            "stored1.txt",
            100,
            user.id,
        ))
        .await
        .unwrap();
        repo.create(&NewFile::new(
            folder1.id,
            "file2.txt",
            "stored2.txt",
            200,
            user.id,
        ))
        .await
        .unwrap();
        repo.create(&NewFile::new(
            folder2.id,
            "file3.txt",
            "stored3.txt",
            300,
            user.id,
        ))
        .await
        .unwrap();

        let files = repo.list_by_folder(folder1.id).await.unwrap();
        assert_eq!(files.len(), 2);
        // Should be ordered by created_at DESC, so file2 comes first
        assert_eq!(files[0].filename, "file2.txt");
        assert_eq!(files[1].filename, "file1.txt");
    }

    #[tokio::test]
    async fn test_list_by_uploader() {
        let db = setup_db().await;
        let pool = db.pool();
        let user_repo = UserRepository::new(pool);
        let user1 = user_repo
            .create(&NewUser::new("user1", "password", "User 1"))
            .await
            .unwrap();
        let user2 = user_repo
            .create(&NewUser::new("user2", "password", "User 2"))
            .await
            .unwrap();
        let folder = create_test_folder(pool).await;

        let repo = FileRepository::new(pool);
        repo.create(&NewFile::new(
            folder.id,
            "file1.txt",
            "stored1.txt",
            100,
            user1.id,
        ))
        .await
        .unwrap();
        repo.create(&NewFile::new(
            folder.id,
            "file2.txt",
            "stored2.txt",
            200,
            user2.id,
        ))
        .await
        .unwrap();
        repo.create(&NewFile::new(
            folder.id,
            "file3.txt",
            "stored3.txt",
            300,
            user1.id,
        ))
        .await
        .unwrap();

        let user1_files = repo.list_by_uploader(user1.id).await.unwrap();
        assert_eq!(user1_files.len(), 2);
    }

    #[tokio::test]
    async fn test_update_file() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder = create_test_folder(pool).await;

        let repo = FileRepository::new(pool);
        let file = repo
            .create(&NewFile::new(
                folder.id,
                "old.txt",
                "stored.txt",
                100,
                user.id,
            ))
            .await
            .unwrap();

        let update = FileUpdate::new()
            .filename("new.txt")
            .description(Some("Updated description"));

        let updated = repo.update(file.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.filename, "new.txt");
        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_increment_downloads() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder = create_test_folder(pool).await;

        let repo = FileRepository::new(pool);
        let file = repo
            .create(&NewFile::new(
                folder.id,
                "file.txt",
                "stored.txt",
                100,
                user.id,
            ))
            .await
            .unwrap();

        assert_eq!(file.downloads, 0);

        let downloads = repo.increment_downloads(file.id).await.unwrap();
        assert_eq!(downloads, 1);

        let downloads = repo.increment_downloads(file.id).await.unwrap();
        assert_eq!(downloads, 2);
    }

    #[tokio::test]
    async fn test_delete_file() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder = create_test_folder(pool).await;

        let repo = FileRepository::new(pool);
        let file = repo
            .create(&NewFile::new(
                folder.id,
                "file.txt",
                "stored.txt",
                100,
                user.id,
            ))
            .await
            .unwrap();

        let deleted = repo.delete(file.id).await.unwrap();
        assert!(deleted);

        let found = repo.get_by_id(file.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_file_not_found() {
        let db = setup_db().await;
        let pool = db.pool();

        let repo = FileRepository::new(pool);
        let deleted = repo.delete(9999).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_count_by_folder() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder = create_test_folder(pool).await;

        let repo = FileRepository::new(pool);
        assert_eq!(repo.count_by_folder(folder.id).await.unwrap(), 0);

        repo.create(&NewFile::new(
            folder.id,
            "file1.txt",
            "stored1.txt",
            100,
            user.id,
        ))
        .await
        .unwrap();
        repo.create(&NewFile::new(
            folder.id,
            "file2.txt",
            "stored2.txt",
            200,
            user.id,
        ))
        .await
        .unwrap();

        assert_eq!(repo.count_by_folder(folder.id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_total_size_by_folder() {
        let db = setup_db().await;
        let pool = db.pool();
        let user = create_test_user(pool).await;
        let folder = create_test_folder(pool).await;

        let repo = FileRepository::new(pool);
        assert_eq!(repo.total_size_by_folder(folder.id).await.unwrap(), 0);

        repo.create(&NewFile::new(
            folder.id,
            "file1.txt",
            "stored1.txt",
            100,
            user.id,
        ))
        .await
        .unwrap();
        repo.create(&NewFile::new(
            folder.id,
            "file2.txt",
            "stored2.txt",
            250,
            user.id,
        ))
        .await
        .unwrap();

        assert_eq!(repo.total_size_by_folder(folder.id).await.unwrap(), 350);
    }

    #[tokio::test]
    async fn test_new_file_builder() {
        let new_file =
            NewFile::new(1, "test.txt", "stored.txt", 1024, 5).with_description("Test description");

        assert_eq!(new_file.folder_id, 1);
        assert_eq!(new_file.filename, "test.txt");
        assert_eq!(new_file.stored_name, "stored.txt");
        assert_eq!(new_file.size, 1024);
        assert_eq!(new_file.uploader_id, 5);
        assert_eq!(new_file.description, Some("Test description".to_string()));
    }

    #[tokio::test]
    async fn test_file_update_builder() {
        let update = FileUpdate::new()
            .filename("new.txt")
            .description(Some("New description"))
            .downloads(10);

        assert_eq!(update.filename, Some("new.txt".to_string()));
        assert_eq!(
            update.description,
            Some(Some("New description".to_string()))
        );
        assert_eq!(update.downloads, Some(10));
    }
}
