//! File metadata types and repository for HOBBS file management.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::Result;

/// Metadata for a file in the file library.
#[derive(Debug, Clone)]
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
    pub created_at: DateTime<Utc>,
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
}

/// Repository for file metadata operations.
pub struct FileRepository;

impl FileRepository {
    /// Create a new file entry.
    pub fn create(conn: &Connection, file: &NewFile) -> Result<FileMetadata> {
        conn.execute(
            "INSERT INTO files (folder_id, filename, stored_name, size, description, uploader_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file.folder_id,
                file.filename,
                file.stored_name,
                file.size,
                file.description,
                file.uploader_id,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?
            .ok_or_else(|| crate::HobbsError::Database(rusqlite::Error::QueryReturnedNoRows))
    }

    /// Get a file by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<FileMetadata>> {
        let file = conn
            .query_row(
                "SELECT id, folder_id, filename, stored_name, size, description, uploader_id, downloads, created_at
                 FROM files WHERE id = ?1",
                [id],
                Self::map_row,
            )
            .optional()?;

        Ok(file)
    }

    /// Get a file by stored name.
    pub fn get_by_stored_name(
        conn: &Connection,
        stored_name: &str,
    ) -> Result<Option<FileMetadata>> {
        let file = conn
            .query_row(
                "SELECT id, folder_id, filename, stored_name, size, description, uploader_id, downloads, created_at
                 FROM files WHERE stored_name = ?1",
                [stored_name],
                Self::map_row,
            )
            .optional()?;

        Ok(file)
    }

    /// List files in a folder (ordered by created_at descending).
    pub fn list_by_folder(conn: &Connection, folder_id: i64) -> Result<Vec<FileMetadata>> {
        let mut stmt = conn.prepare(
            "SELECT id, folder_id, filename, stored_name, size, description, uploader_id, downloads, created_at
             FROM files WHERE folder_id = ?1 ORDER BY created_at DESC, id DESC",
        )?;

        let files: Vec<FileMetadata> = stmt
            .query_map([folder_id], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(files)
    }

    /// List files uploaded by a user.
    pub fn list_by_uploader(conn: &Connection, uploader_id: i64) -> Result<Vec<FileMetadata>> {
        let mut stmt = conn.prepare(
            "SELECT id, folder_id, filename, stored_name, size, description, uploader_id, downloads, created_at
             FROM files WHERE uploader_id = ?1 ORDER BY created_at DESC, id DESC",
        )?;

        let files: Vec<FileMetadata> = stmt
            .query_map([uploader_id], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(files)
    }

    /// Update file metadata.
    pub fn update(conn: &Connection, id: i64, update: &FileUpdate) -> Result<Option<FileMetadata>> {
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref filename) = update.filename {
            updates.push("filename = ?");
            params.push(Box::new(filename.clone()));
        }

        if let Some(ref description) = update.description {
            updates.push("description = ?");
            params.push(Box::new(description.clone()));
        }

        if let Some(downloads) = update.downloads {
            updates.push("downloads = ?");
            params.push(Box::new(downloads));
        }

        if updates.is_empty() {
            return Self::get_by_id(conn, id);
        }

        params.push(Box::new(id));

        let query = format!("UPDATE files SET {} WHERE id = ?", updates.join(", "));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&query, param_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Increment the download count for a file.
    pub fn increment_downloads(conn: &Connection, id: i64) -> Result<i64> {
        conn.execute(
            "UPDATE files SET downloads = downloads + 1 WHERE id = ?1",
            [id],
        )?;

        let downloads: i64 =
            conn.query_row("SELECT downloads FROM files WHERE id = ?1", [id], |row| {
                row.get(0)
            })?;

        Ok(downloads)
    }

    /// Delete a file by ID.
    pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
        let rows = conn.execute("DELETE FROM files WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Count files in a folder.
    pub fn count_by_folder(conn: &Connection, folder_id: i64) -> Result<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE folder_id = ?1",
            [folder_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get total size of files in a folder.
    pub fn total_size_by_folder(conn: &Connection, folder_id: i64) -> Result<i64> {
        let size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(size), 0) FROM files WHERE folder_id = ?1",
            [folder_id],
            |row| row.get(0),
        )?;
        Ok(size)
    }

    /// Map a database row to FileMetadata.
    fn map_row(row: &Row) -> rusqlite::Result<FileMetadata> {
        let created_at_str: String = row.get(8)?;

        Ok(FileMetadata {
            id: row.get(0)?,
            folder_id: row.get(1)?,
            filename: row.get(2)?,
            stored_name: row.get(3)?,
            size: row.get(4)?,
            description: row.get(5)?,
            uploader_id: row.get(6)?,
            downloads: row.get(7)?,
            created_at: DateTime::parse_from_rfc3339(&format!("{created_at_str}Z"))
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, NewUser, UserRepository};
    use crate::file::{FolderRepository, NewFolder};

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_user(db: &Database) -> crate::db::User {
        let repo = UserRepository::new(db);
        let user = NewUser::new("testuser", "password123", "Test User");
        repo.create(&user).unwrap()
    }

    fn create_test_folder(conn: &Connection) -> super::super::folder::Folder {
        FolderRepository::create(conn, &NewFolder::new("Test Folder")).unwrap()
    }

    #[test]
    fn test_create_file() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder = create_test_folder(conn);

        let new_file = NewFile::new(
            folder.id,
            "test.txt",
            "abc12345-1234-5678-90ab-cdef12345678.txt",
            1024,
            user.id,
        )
        .with_description("Test file");

        let file = FileRepository::create(conn, &new_file).unwrap();

        assert_eq!(file.folder_id, folder.id);
        assert_eq!(file.filename, "test.txt");
        assert_eq!(file.stored_name, "abc12345-1234-5678-90ab-cdef12345678.txt");
        assert_eq!(file.size, 1024);
        assert_eq!(file.description, Some("Test file".to_string()));
        assert_eq!(file.uploader_id, user.id);
        assert_eq!(file.downloads, 0);
    }

    #[test]
    fn test_get_file_by_id() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder = create_test_folder(conn);

        let new_file = NewFile::new(folder.id, "file.txt", "stored.txt", 100, user.id);
        let created = FileRepository::create(conn, &new_file).unwrap();

        let found = FileRepository::get_by_id(conn, created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().filename, "file.txt");
    }

    #[test]
    fn test_get_file_not_found() {
        let db = setup_db();
        let conn = db.conn();

        let found = FileRepository::get_by_id(conn, 9999).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_get_by_stored_name() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder = create_test_folder(conn);

        let stored_name = "unique-stored-name.txt";
        let new_file = NewFile::new(folder.id, "file.txt", stored_name, 100, user.id);
        FileRepository::create(conn, &new_file).unwrap();

        let found = FileRepository::get_by_stored_name(conn, stored_name).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().stored_name, stored_name);
    }

    #[test]
    fn test_list_by_folder() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder1 = create_test_folder(conn);
        let folder2 = FolderRepository::create(conn, &NewFolder::new("Other")).unwrap();

        FileRepository::create(
            conn,
            &NewFile::new(folder1.id, "file1.txt", "stored1.txt", 100, user.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder1.id, "file2.txt", "stored2.txt", 200, user.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder2.id, "file3.txt", "stored3.txt", 300, user.id),
        )
        .unwrap();

        let files = FileRepository::list_by_folder(conn, folder1.id).unwrap();
        assert_eq!(files.len(), 2);
        // Should be ordered by created_at DESC, so file2 comes first
        assert_eq!(files[0].filename, "file2.txt");
        assert_eq!(files[1].filename, "file1.txt");
    }

    #[test]
    fn test_list_by_uploader() {
        let db = setup_db();
        let conn = db.conn();
        let user1 = create_test_user(&db);
        let user2 = {
            let repo = UserRepository::new(&db);
            repo.create(&NewUser::new("user2", "password", "User 2"))
                .unwrap()
        };
        let folder = create_test_folder(conn);

        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file1.txt", "stored1.txt", 100, user1.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file2.txt", "stored2.txt", 200, user2.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file3.txt", "stored3.txt", 300, user1.id),
        )
        .unwrap();

        let user1_files = FileRepository::list_by_uploader(conn, user1.id).unwrap();
        assert_eq!(user1_files.len(), 2);
    }

    #[test]
    fn test_update_file() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder = create_test_folder(conn);

        let file = FileRepository::create(
            conn,
            &NewFile::new(folder.id, "old.txt", "stored.txt", 100, user.id),
        )
        .unwrap();

        let update = FileUpdate::new()
            .filename("new.txt")
            .description(Some("Updated description"));

        let updated = FileRepository::update(conn, file.id, &update)
            .unwrap()
            .unwrap();

        assert_eq!(updated.filename, "new.txt");
        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[test]
    fn test_increment_downloads() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder = create_test_folder(conn);

        let file = FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file.txt", "stored.txt", 100, user.id),
        )
        .unwrap();

        assert_eq!(file.downloads, 0);

        let downloads = FileRepository::increment_downloads(conn, file.id).unwrap();
        assert_eq!(downloads, 1);

        let downloads = FileRepository::increment_downloads(conn, file.id).unwrap();
        assert_eq!(downloads, 2);
    }

    #[test]
    fn test_delete_file() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder = create_test_folder(conn);

        let file = FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file.txt", "stored.txt", 100, user.id),
        )
        .unwrap();

        let deleted = FileRepository::delete(conn, file.id).unwrap();
        assert!(deleted);

        let found = FileRepository::get_by_id(conn, file.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_delete_file_not_found() {
        let db = setup_db();
        let conn = db.conn();

        let deleted = FileRepository::delete(conn, 9999).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_count_by_folder() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder = create_test_folder(conn);

        assert_eq!(FileRepository::count_by_folder(conn, folder.id).unwrap(), 0);

        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file1.txt", "stored1.txt", 100, user.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file2.txt", "stored2.txt", 200, user.id),
        )
        .unwrap();

        assert_eq!(FileRepository::count_by_folder(conn, folder.id).unwrap(), 2);
    }

    #[test]
    fn test_total_size_by_folder() {
        let db = setup_db();
        let conn = db.conn();
        let user = create_test_user(&db);
        let folder = create_test_folder(conn);

        assert_eq!(
            FileRepository::total_size_by_folder(conn, folder.id).unwrap(),
            0
        );

        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file1.txt", "stored1.txt", 100, user.id),
        )
        .unwrap();
        FileRepository::create(
            conn,
            &NewFile::new(folder.id, "file2.txt", "stored2.txt", 250, user.id),
        )
        .unwrap();

        assert_eq!(
            FileRepository::total_size_by_folder(conn, folder.id).unwrap(),
            350
        );
    }

    #[test]
    fn test_new_file_builder() {
        let new_file =
            NewFile::new(1, "test.txt", "stored.txt", 1024, 5).with_description("Test description");

        assert_eq!(new_file.folder_id, 1);
        assert_eq!(new_file.filename, "test.txt");
        assert_eq!(new_file.stored_name, "stored.txt");
        assert_eq!(new_file.size, 1024);
        assert_eq!(new_file.uploader_id, 5);
        assert_eq!(new_file.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_file_update_builder() {
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
