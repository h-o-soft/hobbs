//! File storage for HOBBS.
//!
//! This module provides physical file storage functionality:
//! - UUID-based file naming
//! - Directory sharding by first 2 characters of UUID
//! - Save, load, and delete operations

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::{HobbsError, Result};

/// File storage service for managing physical files.
///
/// Files are stored in a sharded directory structure:
/// ```text
/// {base_path}/
/// ├── ab/
/// │   └── ab12cd34-5678-90ab-cdef-123456789012.txt
/// ├── cd/
/// │   └── cd90ab12-3456-7890-abcd-ef1234567890.bin
/// └── ...
/// ```
#[derive(Debug, Clone)]
pub struct FileStorage {
    /// Base directory for file storage.
    base_path: PathBuf,
}

impl FileStorage {
    /// Create a new FileStorage with the given base path.
    ///
    /// The base directory will be created if it doesn't exist.
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self> {
        let base_path = base_path.into();
        fs::create_dir_all(&base_path)?;

        Ok(Self { base_path })
    }

    /// Get the base path of this storage.
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Save content to storage with a new UUID-based filename.
    ///
    /// # Arguments
    ///
    /// * `content` - The file content to save
    /// * `original_name` - The original filename (used to extract extension)
    ///
    /// # Returns
    ///
    /// The stored filename (UUID.extension format)
    pub fn save(&self, content: &[u8], original_name: &str) -> Result<String> {
        let uuid = Uuid::new_v4();
        let ext = Self::extract_extension(original_name);
        let stored_name = format!("{uuid}.{ext}");

        self.save_with_name(content, &stored_name)?;
        Ok(stored_name)
    }

    /// Save content with a specific stored name.
    ///
    /// This is useful when you already have a stored name (e.g., from database).
    pub fn save_with_name(&self, content: &[u8], stored_name: &str) -> Result<()> {
        let file_path = self.get_file_path(stored_name);

        // Create the shard directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&file_path, content)?;

        Ok(())
    }

    /// Load content from storage.
    ///
    /// # Arguments
    ///
    /// * `stored_name` - The stored filename (UUID.extension format)
    ///
    /// # Returns
    ///
    /// The file content as bytes
    pub fn load(&self, stored_name: &str) -> Result<Vec<u8>> {
        let file_path = self.get_file_path(stored_name);

        match fs::read(&file_path) {
            Ok(content) => Ok(content),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Err(HobbsError::NotFound(format!("File: {stored_name}")))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a file from storage.
    ///
    /// # Arguments
    ///
    /// * `stored_name` - The stored filename (UUID.extension format)
    ///
    /// # Returns
    ///
    /// `true` if the file was deleted, `false` if it didn't exist
    pub fn delete(&self, stored_name: &str) -> Result<bool> {
        let file_path = self.get_file_path(stored_name);

        match fs::remove_file(&file_path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Check if a file exists in storage.
    pub fn exists(&self, stored_name: &str) -> bool {
        let file_path = self.get_file_path(stored_name);
        file_path.exists()
    }

    /// Get the size of a stored file.
    pub fn file_size(&self, stored_name: &str) -> Result<u64> {
        let file_path = self.get_file_path(stored_name);

        match fs::metadata(&file_path) {
            Ok(m) => Ok(m.len()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Err(HobbsError::NotFound(format!("File: {stored_name}")))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Get the full file path for a stored name.
    ///
    /// The path is constructed as: {base_path}/{shard}/{stored_name}
    /// where shard is the first 2 characters of the stored name (UUID prefix).
    pub fn get_file_path(&self, stored_name: &str) -> PathBuf {
        let shard = Self::get_shard(stored_name);
        self.base_path.join(shard).join(stored_name)
    }

    /// Get the shard directory name for a stored name.
    ///
    /// Returns the first 2 characters of the stored name (UUID prefix).
    fn get_shard(stored_name: &str) -> &str {
        if stored_name.len() >= 2 {
            &stored_name[..2]
        } else {
            stored_name
        }
    }

    /// Extract the file extension from a filename.
    ///
    /// Returns "bin" if no extension is found.
    fn extract_extension(filename: &str) -> &str {
        Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin")
    }

    /// Generate a new UUID-based stored name with the given extension.
    pub fn generate_stored_name(original_name: &str) -> String {
        let uuid = Uuid::new_v4();
        let ext = Self::extract_extension(original_name);
        format!("{uuid}.{ext}")
    }

    /// Clean up empty shard directories.
    ///
    /// This removes any empty subdirectories in the storage.
    pub fn cleanup_empty_dirs(&self) -> Result<usize> {
        let mut removed = 0;

        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(dir_entries) = fs::read_dir(&path) {
                        if dir_entries.count() == 0 && fs::remove_dir(&path).is_ok() {
                            removed += 1;
                        }
                    }
                }
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_storage() -> (TempDir, FileStorage) {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path()).unwrap();
        (temp_dir, storage)
    }

    #[test]
    fn test_new_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("storage");

        assert!(!storage_path.exists());

        let storage = FileStorage::new(&storage_path).unwrap();

        assert!(storage_path.exists());
        assert_eq!(storage.base_path(), storage_path);
    }

    #[test]
    fn test_save_and_load() {
        let (_temp_dir, storage) = setup_storage();
        let content = b"Hello, World!";

        let stored_name = storage.save(content, "test.txt").unwrap();

        assert!(stored_name.ends_with(".txt"));
        assert!(stored_name.len() > 4); // UUID + .txt

        let loaded = storage.load(&stored_name).unwrap();
        assert_eq!(loaded, content);
    }

    #[test]
    fn test_save_extracts_extension() {
        let (_temp_dir, storage) = setup_storage();

        let stored_name = storage.save(b"data", "document.pdf").unwrap();
        assert!(stored_name.ends_with(".pdf"));

        let stored_name = storage.save(b"data", "image.PNG").unwrap();
        assert!(stored_name.ends_with(".PNG"));

        let stored_name = storage.save(b"data", "no_extension").unwrap();
        assert!(stored_name.ends_with(".bin"));
    }

    #[test]
    fn test_save_creates_shard_directory() {
        let (_temp_dir, storage) = setup_storage();

        let stored_name = storage.save(b"data", "test.txt").unwrap();

        let shard = &stored_name[..2];
        let shard_dir = storage.base_path().join(shard);

        assert!(shard_dir.exists());
        assert!(shard_dir.is_dir());
    }

    #[test]
    fn test_load_not_found() {
        let (_temp_dir, storage) = setup_storage();

        let result = storage.load("nonexistent.txt");

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_delete() {
        let (_temp_dir, storage) = setup_storage();

        let stored_name = storage.save(b"to delete", "delete.txt").unwrap();
        assert!(storage.exists(&stored_name));

        let deleted = storage.delete(&stored_name).unwrap();
        assert!(deleted);
        assert!(!storage.exists(&stored_name));
    }

    #[test]
    fn test_delete_not_found() {
        let (_temp_dir, storage) = setup_storage();

        let deleted = storage.delete("nonexistent.txt").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_exists() {
        let (_temp_dir, storage) = setup_storage();

        let stored_name = storage.save(b"data", "test.txt").unwrap();

        assert!(storage.exists(&stored_name));
        assert!(!storage.exists("nonexistent.txt"));
    }

    #[test]
    fn test_file_size() {
        let (_temp_dir, storage) = setup_storage();
        let content = b"Hello, World!";

        let stored_name = storage.save(content, "test.txt").unwrap();

        let size = storage.file_size(&stored_name).unwrap();
        assert_eq!(size, content.len() as u64);
    }

    #[test]
    fn test_file_size_not_found() {
        let (_temp_dir, storage) = setup_storage();

        let result = storage.file_size("nonexistent.txt");
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_get_file_path() {
        let (_temp_dir, storage) = setup_storage();

        let stored_name = "ab12cd34-5678-90ab-cdef-123456789012.txt";
        let path = storage.get_file_path(stored_name);

        assert_eq!(path, storage.base_path().join("ab").join(stored_name));
    }

    #[test]
    fn test_get_shard() {
        assert_eq!(FileStorage::get_shard("abcdef.txt"), "ab");
        assert_eq!(FileStorage::get_shard("12-345.bin"), "12");
        assert_eq!(FileStorage::get_shard("x"), "x");
        assert_eq!(FileStorage::get_shard(""), "");
    }

    #[test]
    fn test_extract_extension() {
        assert_eq!(FileStorage::extract_extension("test.txt"), "txt");
        assert_eq!(FileStorage::extract_extension("document.PDF"), "PDF");
        assert_eq!(FileStorage::extract_extension("no_ext"), "bin");
        assert_eq!(FileStorage::extract_extension("file.tar.gz"), "gz");
        // ".hidden" is a filename without extension, so it defaults to "bin"
        assert_eq!(FileStorage::extract_extension(".hidden"), "bin");
        // "file.hidden" has extension "hidden"
        assert_eq!(FileStorage::extract_extension("file.hidden"), "hidden");
    }

    #[test]
    fn test_generate_stored_name() {
        let name1 = FileStorage::generate_stored_name("test.txt");
        let name2 = FileStorage::generate_stored_name("test.txt");

        // Should generate unique names
        assert_ne!(name1, name2);

        // Should preserve extension
        assert!(name1.ends_with(".txt"));
        assert!(name2.ends_with(".txt"));

        // Should be valid UUID format (36 chars + . + extension)
        assert!(name1.len() > 36);
    }

    #[test]
    fn test_save_with_name() {
        let (_temp_dir, storage) = setup_storage();
        let content = b"specific content";
        let stored_name = "ab123456-7890-abcd-ef12-345678901234.txt";

        storage.save_with_name(content, stored_name).unwrap();

        assert!(storage.exists(stored_name));
        let loaded = storage.load(stored_name).unwrap();
        assert_eq!(loaded, content);
    }

    #[test]
    fn test_cleanup_empty_dirs() {
        let (_temp_dir, storage) = setup_storage();

        // Create a file and then delete it
        let stored_name = storage.save(b"temp", "temp.txt").unwrap();
        storage.delete(&stored_name).unwrap();

        // The shard directory should be empty now
        let removed = storage.cleanup_empty_dirs().unwrap();

        // Should have removed at least one empty directory
        assert!(removed >= 1);
    }

    #[test]
    fn test_binary_content() {
        let (_temp_dir, storage) = setup_storage();

        // Test with binary content
        let content: Vec<u8> = (0..=255).collect();

        let stored_name = storage.save(&content, "binary.bin").unwrap();
        let loaded = storage.load(&stored_name).unwrap();

        assert_eq!(loaded, content);
    }

    #[test]
    fn test_large_file() {
        let (_temp_dir, storage) = setup_storage();

        // Create a 1MB file
        let content: Vec<u8> = vec![0xAB; 1024 * 1024];

        let stored_name = storage.save(&content, "large.bin").unwrap();

        assert_eq!(storage.file_size(&stored_name).unwrap(), 1024 * 1024);

        let loaded = storage.load(&stored_name).unwrap();
        assert_eq!(loaded, content);
    }

    #[test]
    fn test_unicode_original_name() {
        let (_temp_dir, storage) = setup_storage();

        // Japanese filename
        let stored_name = storage.save(b"data", "日本語ファイル.txt").unwrap();
        assert!(stored_name.ends_with(".txt"));

        // Emoji in filename
        let stored_name = storage.save(b"data", "📄document.pdf").unwrap();
        assert!(stored_name.ends_with(".pdf"));
    }
}
