//! File management module for HOBBS.
//!
//! This module provides file upload/download functionality including:
//! - Hierarchical folder structure
//! - Folder and file metadata management
//! - Permission-based access control
//! - File storage with UUID naming

mod folder;
mod metadata;

pub use folder::{Folder, FolderRepository, FolderUpdate, NewFolder};
pub use metadata::{FileMetadata, FileRepository, FileUpdate, NewFile};

/// Maximum length for filename (in characters).
pub const MAX_FILENAME_LENGTH: usize = 100;

/// Maximum length for file/folder description (in characters).
pub const MAX_DESCRIPTION_LENGTH: usize = 500;

/// Maximum folder depth (levels).
pub const MAX_FOLDER_DEPTH: usize = 10;

/// Default maximum file size (10MB).
pub const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
