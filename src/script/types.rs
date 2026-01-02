//! Script types and data structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A script stored in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    /// Unique identifier.
    pub id: i64,
    /// File path relative to scripts directory.
    pub file_path: String,
    /// Display name (from metadata).
    pub name: String,
    /// URL-safe identifier (derived from filename).
    pub slug: String,
    /// Description (from metadata).
    pub description: Option<String>,
    /// Author name (from metadata).
    pub author: Option<String>,
    /// File hash for change detection.
    pub file_hash: Option<String>,
    /// Last sync timestamp.
    pub synced_at: Option<DateTime<Utc>>,
    /// Minimum role required to execute.
    pub min_role: i32,
    /// Whether the script is enabled.
    pub enabled: bool,
    /// Maximum instruction count.
    pub max_instructions: i64,
    /// Maximum memory in MB.
    pub max_memory_mb: i32,
    /// Maximum execution time in seconds.
    pub max_execution_seconds: i32,
}

impl Script {
    /// Check if a user with the given role can execute this script.
    pub fn can_execute(&self, user_role: i32) -> bool {
        self.enabled && user_role >= self.min_role
    }
}

/// Script metadata parsed from Lua file comments.
#[derive(Debug, Clone, Default)]
pub struct ScriptMetadata {
    /// Display name (@name).
    pub name: Option<String>,
    /// Description (@description).
    pub description: Option<String>,
    /// Author (@author).
    pub author: Option<String>,
    /// Minimum role (@min_role).
    pub min_role: Option<i32>,
    /// Enabled flag (@enabled).
    pub enabled: Option<bool>,
}

/// Result of syncing scripts from the file system.
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    /// Number of scripts added.
    pub added: usize,
    /// Number of scripts updated.
    pub updated: usize,
    /// Number of scripts removed.
    pub removed: usize,
    /// Errors encountered during sync.
    pub errors: Vec<(String, String)>,
}

impl SyncResult {
    /// Check if any changes were made.
    pub fn has_changes(&self) -> bool {
        self.added > 0 || self.updated > 0 || self.removed > 0
    }

    /// Total number of changes.
    pub fn total_changes(&self) -> usize {
        self.added + self.updated + self.removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_can_execute() {
        let script = Script {
            id: 1,
            file_path: "test.lua".to_string(),
            name: "Test".to_string(),
            slug: "test".to_string(),
            description: None,
            author: None,
            file_hash: None,
            synced_at: None,
            min_role: 1,
            enabled: true,
            max_instructions: 1000000,
            max_memory_mb: 10,
            max_execution_seconds: 30,
        };

        // Guest (0) cannot execute min_role=1 script
        assert!(!script.can_execute(0));
        // Member (1) can execute
        assert!(script.can_execute(1));
        // SysOp (3) can execute
        assert!(script.can_execute(3));
    }

    #[test]
    fn test_script_disabled_cannot_execute() {
        let script = Script {
            id: 1,
            file_path: "test.lua".to_string(),
            name: "Test".to_string(),
            slug: "test".to_string(),
            description: None,
            author: None,
            file_hash: None,
            synced_at: None,
            min_role: 0,
            enabled: false,
            max_instructions: 1000000,
            max_memory_mb: 10,
            max_execution_seconds: 30,
        };

        // Even SysOp cannot execute disabled script
        assert!(!script.can_execute(3));
    }

    #[test]
    fn test_sync_result() {
        let mut result = SyncResult::default();
        assert!(!result.has_changes());
        assert_eq!(result.total_changes(), 0);

        result.added = 2;
        result.updated = 1;
        assert!(result.has_changes());
        assert_eq!(result.total_changes(), 3);
    }
}
