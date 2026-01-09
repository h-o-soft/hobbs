//! Script loader for scanning and syncing Lua scripts from the file system.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::repository::ScriptRepository;
use super::types::{Script, ScriptMetadata, SyncResult};
use crate::db::DbPool;
use crate::Result;

/// Loader for scanning Lua scripts from the file system.
pub struct ScriptLoader {
    /// Base directory for scripts.
    scripts_dir: PathBuf,
}

impl ScriptLoader {
    /// Create a new ScriptLoader with the given scripts directory.
    pub fn new<P: AsRef<Path>>(scripts_dir: P) -> Self {
        Self {
            scripts_dir: scripts_dir.as_ref().to_path_buf(),
        }
    }

    /// Sync scripts from the file system to the database.
    ///
    /// This will:
    /// 1. Scan the scripts directory for .lua files
    /// 2. Parse metadata from each file
    /// 3. Add new scripts, update changed ones, and remove deleted ones
    pub async fn sync(&self, pool: &DbPool) -> Result<SyncResult> {
        let repo = ScriptRepository::new(pool);
        let mut result = SyncResult::default();

        // Get existing file paths from DB
        let existing_paths: HashSet<String> =
            repo.list_all_file_paths().await?.into_iter().collect();
        let mut found_paths: HashSet<String> = HashSet::new();

        // Scan directory for .lua files
        if self.scripts_dir.exists() {
            self.scan_directory(&self.scripts_dir, &repo, &mut result, &mut found_paths)
                .await?;
        }

        // Remove scripts that no longer exist on disk
        for path in existing_paths.difference(&found_paths) {
            if let Err(e) = repo.delete_by_file_path(path).await {
                result.errors.push((path.clone(), e.to_string()));
            } else {
                result.removed += 1;
            }
        }

        Ok(result)
    }

    /// Scan a directory recursively for .lua files.
    async fn scan_directory(
        &self,
        dir: &Path,
        repo: &ScriptRepository<'_>,
        result: &mut SyncResult,
        found_paths: &mut HashSet<String>,
    ) -> Result<()> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                result
                    .errors
                    .push((dir.display().to_string(), e.to_string()));
                return Ok(());
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                Box::pin(self.scan_directory(&path, repo, result, found_paths)).await?;
            } else if path.extension().is_some_and(|ext| ext == "lua") {
                // Process .lua file
                if let Err(e) = self
                    .process_script_file(&path, repo, result, found_paths)
                    .await
                {
                    result
                        .errors
                        .push((path.display().to_string(), e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Process a single script file.
    async fn process_script_file(
        &self,
        path: &Path,
        repo: &ScriptRepository<'_>,
        result: &mut SyncResult,
        found_paths: &mut HashSet<String>,
    ) -> Result<()> {
        // Get relative path
        let rel_path = path
            .strip_prefix(&self.scripts_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        found_paths.insert(rel_path.clone());

        // Read file content
        let content = fs::read_to_string(path)?;

        // Calculate hash
        let file_hash = Self::calculate_hash(&content);

        // Check if script exists and is unchanged
        if let Some(existing) = repo.get_by_file_path(&rel_path).await? {
            if existing.file_hash.as_deref() == Some(&file_hash) {
                // Unchanged, skip
                return Ok(());
            }
        }

        // Parse metadata
        let metadata = Self::parse_metadata(&content);

        // Generate slug from filename
        let slug = Self::generate_slug(path);

        // Create script
        let script = Script {
            id: 0,
            file_path: rel_path.clone(),
            name: metadata
                .name
                .unwrap_or_else(|| Self::filename_to_name(path)),
            slug,
            description: metadata.description,
            author: metadata.author,
            name_i18n: metadata.name_i18n,
            description_i18n: metadata.description_i18n,
            file_hash: Some(file_hash),
            synced_at: Some(Utc::now()),
            min_role: metadata.min_role.unwrap_or(0),
            enabled: metadata.enabled.unwrap_or(true),
            max_instructions: 1_000_000,
            max_memory_mb: 10,
            max_execution_seconds: 30,
        };

        // Check if this is an update or insert
        let is_update = repo.get_by_file_path(&rel_path).await?.is_some();

        repo.upsert(&script).await?;

        if is_update {
            result.updated += 1;
        } else {
            result.added += 1;
        }

        Ok(())
    }

    /// Parse metadata from Lua file comments.
    ///
    /// Looks for comments like:
    /// ```lua
    /// -- @name Script Name
    /// -- @name.ja スクリプト名
    /// -- @description Description text
    /// -- @description.ja 説明文
    /// -- @author Author Name
    /// -- @min_role 0
    /// -- @enabled true
    /// ```
    pub fn parse_metadata(content: &str) -> ScriptMetadata {
        let mut metadata = ScriptMetadata::default();

        for line in content.lines() {
            let line = line.trim();
            if !line.starts_with("--") {
                // Stop at first non-comment line
                if !line.is_empty() {
                    break;
                }
                continue;
            }

            let comment = line.trim_start_matches("--").trim();

            // Try to parse localized metadata first (e.g., @name.ja, @description.en)
            if let Some(rest) = comment.strip_prefix("@name.") {
                if let Some((lang, value)) = Self::parse_localized_value(rest) {
                    metadata.name_i18n.insert(lang, value);
                }
            } else if let Some(rest) = comment.strip_prefix("@description.") {
                if let Some((lang, value)) = Self::parse_localized_value(rest) {
                    metadata.description_i18n.insert(lang, value);
                }
            } else if let Some(value) = comment.strip_prefix("@name ") {
                metadata.name = Some(value.trim().to_string());
            } else if let Some(value) = comment.strip_prefix("@description ") {
                metadata.description = Some(value.trim().to_string());
            } else if let Some(value) = comment.strip_prefix("@author ") {
                metadata.author = Some(value.trim().to_string());
            } else if let Some(value) = comment.strip_prefix("@min_role ") {
                metadata.min_role = value.trim().parse().ok();
            } else if let Some(value) = comment.strip_prefix("@enabled ") {
                metadata.enabled = value.trim().parse().ok();
            }
        }

        metadata
    }

    /// Parse a localized value like "ja スクリプト名" into ("ja", "スクリプト名").
    fn parse_localized_value(s: &str) -> Option<(String, String)> {
        let s = s.trim();
        // Find the first space to separate language code from value
        let space_pos = s.find(' ')?;
        let lang = &s[..space_pos];
        let value = s[space_pos + 1..].trim();

        // Validate language code (2-10 chars: alphanumeric, underscore, hyphen)
        // Examples: "ja", "en", "zh_CN", "pt-BR"
        if lang.len() >= 2
            && lang.len() <= 10
            && lang
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            && !value.is_empty()
        {
            Some((lang.to_string(), value.to_string()))
        } else {
            None
        }
    }

    /// Calculate a simple hash of the file content.
    fn calculate_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Generate a URL-safe slug from the file path.
    fn generate_slug(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect()
    }

    /// Convert filename to a display name.
    fn filename_to_name(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }

    /// Get the scripts directory path.
    pub fn scripts_dir(&self) -> &Path {
        &self.scripts_dir
    }

    /// Check if the scripts directory exists.
    pub fn scripts_dir_exists(&self) -> bool {
        self.scripts_dir.exists()
    }

    /// Create the scripts directory if it doesn't exist.
    pub fn ensure_scripts_dir(&self) -> Result<()> {
        if !self.scripts_dir.exists() {
            fs::create_dir_all(&self.scripts_dir)?;
        }
        Ok(())
    }

    /// Read the source code of a script from the file system.
    pub fn read_script_source(&self, file_path: &str) -> Result<String> {
        let full_path = self.scripts_dir.join(file_path);
        Ok(fs::read_to_string(full_path)?)
    }

    /// Load translations from a sidecar .i18n.toml file.
    ///
    /// Given a script file path like "game.lua", this looks for "game.i18n.toml"
    /// in the same directory and parses it.
    ///
    /// # Returns
    ///
    /// A HashMap where keys are language codes (e.g., "ja", "en") and values are
    /// HashMaps of translation keys to translated strings.
    ///
    /// Returns an empty HashMap if the file doesn't exist or can't be parsed.
    pub fn load_translations(&self, script_path: &str) -> HashMap<String, HashMap<String, String>> {
        let script_full_path = self.scripts_dir.join(script_path);

        // Build the i18n file path by replacing .lua with .i18n.toml
        let i18n_path = if let Some(stem) = script_full_path.file_stem() {
            script_full_path.with_file_name(format!("{}.i18n.toml", stem.to_string_lossy()))
        } else {
            return HashMap::new();
        };

        Self::parse_translations_file(&i18n_path)
    }

    /// Parse a translations file.
    ///
    /// # Format
    ///
    /// ```toml
    /// [ja]
    /// title = "じゃんけん"
    /// rock = "グー"
    ///
    /// [en]
    /// title = "Rock-Paper-Scissors"
    /// rock = "Rock"
    /// ```
    fn parse_translations_file(path: &Path) -> HashMap<String, HashMap<String, String>> {
        if !path.exists() {
            return HashMap::new();
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };

        let table: toml::Table = match toml::from_str(&content) {
            Ok(t) => t,
            Err(_) => return HashMap::new(),
        };

        let mut translations = HashMap::new();

        for (lang, value) in table {
            if let toml::Value::Table(lang_table) = value {
                let mut lang_translations = HashMap::new();
                for (key, val) in lang_table {
                    if let toml::Value::String(s) = val {
                        lang_translations.insert(key, s);
                    }
                }
                if !lang_translations.is_empty() {
                    translations.insert(lang, lang_translations);
                }
            }
        }

        translations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use tempfile::tempdir;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    #[test]
    fn test_parse_metadata_full() {
        let content = r#"-- @name じゃんけん
-- @description じゃんけんゲーム。勝敗記録付き。
-- @author SysOp
-- @min_role 0
-- @enabled true

bbs.println("Hello")
"#;

        let metadata = ScriptLoader::parse_metadata(content);
        assert_eq!(metadata.name, Some("じゃんけん".to_string()));
        assert_eq!(
            metadata.description,
            Some("じゃんけんゲーム。勝敗記録付き。".to_string())
        );
        assert_eq!(metadata.author, Some("SysOp".to_string()));
        assert_eq!(metadata.min_role, Some(0));
        assert_eq!(metadata.enabled, Some(true));
    }

    #[test]
    fn test_parse_metadata_partial() {
        let content = r#"-- @name Test Script
-- @author Test

bbs.println("Hello")
"#;

        let metadata = ScriptLoader::parse_metadata(content);
        assert_eq!(metadata.name, Some("Test Script".to_string()));
        assert_eq!(metadata.author, Some("Test".to_string()));
        assert!(metadata.description.is_none());
        assert!(metadata.min_role.is_none());
        assert!(metadata.enabled.is_none());
    }

    #[test]
    fn test_parse_metadata_i18n() {
        let content = r#"-- @name Rock-Paper-Scissors
-- @name.ja じゃんけん
-- @name.de Schere-Stein-Papier
-- @description Play rock-paper-scissors game
-- @description.ja じゃんけんゲームで遊ぼう
-- @author SysOp
-- @min_role 0

bbs.println("Hello")
"#;

        let metadata = ScriptLoader::parse_metadata(content);
        assert_eq!(metadata.name, Some("Rock-Paper-Scissors".to_string()));
        assert_eq!(
            metadata.name_i18n.get("ja"),
            Some(&"じゃんけん".to_string())
        );
        assert_eq!(
            metadata.name_i18n.get("de"),
            Some(&"Schere-Stein-Papier".to_string())
        );
        assert_eq!(
            metadata.description,
            Some("Play rock-paper-scissors game".to_string())
        );
        assert_eq!(
            metadata.description_i18n.get("ja"),
            Some(&"じゃんけんゲームで遊ぼう".to_string())
        );
        assert_eq!(metadata.author, Some("SysOp".to_string()));
        assert_eq!(metadata.min_role, Some(0));
    }

    #[test]
    fn test_parse_metadata_i18n_only() {
        // Test with only localized versions (no default)
        let content = r#"-- @name.en English Name
-- @name.ja 日本語名
-- @description.en English description
-- @description.ja 日本語説明

bbs.println("Hello")
"#;

        let metadata = ScriptLoader::parse_metadata(content);
        assert!(metadata.name.is_none()); // No default name
        assert_eq!(
            metadata.name_i18n.get("en"),
            Some(&"English Name".to_string())
        );
        assert_eq!(metadata.name_i18n.get("ja"), Some(&"日本語名".to_string()));
        assert!(metadata.description.is_none()); // No default description
        assert_eq!(
            metadata.description_i18n.get("en"),
            Some(&"English description".to_string())
        );
        assert_eq!(
            metadata.description_i18n.get("ja"),
            Some(&"日本語説明".to_string())
        );
    }

    #[test]
    fn test_parse_localized_value() {
        // Valid cases
        assert_eq!(
            ScriptLoader::parse_localized_value("ja テスト"),
            Some(("ja".to_string(), "テスト".to_string()))
        );
        assert_eq!(
            ScriptLoader::parse_localized_value("en Test Value"),
            Some(("en".to_string(), "Test Value".to_string()))
        );
        assert_eq!(
            ScriptLoader::parse_localized_value("zh_CN 中文"),
            Some(("zh_CN".to_string(), "中文".to_string()))
        );

        // Invalid cases
        assert_eq!(ScriptLoader::parse_localized_value("j テスト"), None); // Too short lang
        assert_eq!(
            ScriptLoader::parse_localized_value("waytoolong1 テスト"),
            None
        ); // Too long lang (11 chars)
        assert_eq!(ScriptLoader::parse_localized_value("ja "), None); // Empty value
        assert_eq!(ScriptLoader::parse_localized_value("ja"), None); // No space/value
    }

    #[test]
    fn test_parse_metadata_empty() {
        let content = "bbs.println(\"Hello\")";

        let metadata = ScriptLoader::parse_metadata(content);
        assert!(metadata.name.is_none());
        assert!(metadata.description.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.min_role.is_none());
        assert!(metadata.enabled.is_none());
    }

    #[test]
    fn test_generate_slug() {
        assert_eq!(ScriptLoader::generate_slug(Path::new("test.lua")), "test");
        assert_eq!(
            ScriptLoader::generate_slug(Path::new("My Script.lua")),
            "my_script"
        );
        assert_eq!(
            ScriptLoader::generate_slug(Path::new("game-v2.lua")),
            "game_v2"
        );
    }

    #[test]
    fn test_calculate_hash() {
        let hash1 = ScriptLoader::calculate_hash("content1");
        let hash2 = ScriptLoader::calculate_hash("content2");
        let hash3 = ScriptLoader::calculate_hash("content1");

        assert_ne!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_sync_empty_directory() {
        let db = setup_db().await;
        let dir = tempdir().unwrap();

        let loader = ScriptLoader::new(dir.path());
        let result = loader.sync(db.pool()).await.unwrap();

        assert_eq!(result.added, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.removed, 0);
        assert!(!result.has_changes());
    }

    #[tokio::test]
    async fn test_sync_adds_new_scripts() {
        let db = setup_db().await;
        let dir = tempdir().unwrap();

        // Create test script
        let script_content = r#"-- @name Test Script
-- @description A test
bbs.println("Hello")
"#;
        fs::write(dir.path().join("test.lua"), script_content).unwrap();

        let loader = ScriptLoader::new(dir.path());
        let result = loader.sync(db.pool()).await.unwrap();

        assert_eq!(result.added, 1);
        assert_eq!(result.updated, 0);
        assert_eq!(result.removed, 0);

        // Verify script was added
        let repo = ScriptRepository::new(db.pool());
        let script = repo.get_by_slug("test").await.unwrap().unwrap();
        assert_eq!(script.name, "Test Script");
        assert_eq!(script.description, Some("A test".to_string()));
    }

    #[tokio::test]
    async fn test_sync_updates_changed_scripts() {
        let db = setup_db().await;
        let dir = tempdir().unwrap();

        let script_path = dir.path().join("test.lua");

        // Create initial script
        fs::write(
            &script_path,
            r#"-- @name Version 1
bbs.println("v1")
"#,
        )
        .unwrap();

        let loader = ScriptLoader::new(dir.path());
        loader.sync(db.pool()).await.unwrap();

        // Update script
        fs::write(
            &script_path,
            r#"-- @name Version 2
bbs.println("v2")
"#,
        )
        .unwrap();

        let result = loader.sync(db.pool()).await.unwrap();

        assert_eq!(result.added, 0);
        assert_eq!(result.updated, 1);
        assert_eq!(result.removed, 0);

        // Verify script was updated
        let repo = ScriptRepository::new(db.pool());
        let script = repo.get_by_slug("test").await.unwrap().unwrap();
        assert_eq!(script.name, "Version 2");
    }

    #[tokio::test]
    async fn test_sync_removes_deleted_scripts() {
        let db = setup_db().await;
        let dir = tempdir().unwrap();

        let script_path = dir.path().join("test.lua");

        // Create and sync script
        fs::write(&script_path, "bbs.println(\"Hello\")").unwrap();
        let loader = ScriptLoader::new(dir.path());
        loader.sync(db.pool()).await.unwrap();

        // Delete script file
        fs::remove_file(&script_path).unwrap();

        let result = loader.sync(db.pool()).await.unwrap();

        assert_eq!(result.added, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.removed, 1);

        // Verify script was removed
        let repo = ScriptRepository::new(db.pool());
        let script = repo.get_by_slug("test").await.unwrap();
        assert!(script.is_none());
    }

    #[tokio::test]
    async fn test_sync_handles_subdirectories() {
        let db = setup_db().await;
        let dir = tempdir().unwrap();

        // Create subdirectory
        let subdir = dir.path().join("games");
        fs::create_dir(&subdir).unwrap();

        // Create scripts in root and subdirectory
        fs::write(dir.path().join("root.lua"), "bbs.println(\"root\")").unwrap();
        fs::write(subdir.join("game.lua"), "bbs.println(\"game\")").unwrap();

        let loader = ScriptLoader::new(dir.path());
        let result = loader.sync(db.pool()).await.unwrap();

        assert_eq!(result.added, 2);

        // Verify both scripts were added
        let repo = ScriptRepository::new(db.pool());
        assert!(repo.get_by_slug("root").await.unwrap().is_some());
        assert!(repo.get_by_slug("game").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_sync_skips_unchanged_scripts() {
        let db = setup_db().await;
        let dir = tempdir().unwrap();

        // Create script
        fs::write(dir.path().join("test.lua"), "bbs.println(\"Hello\")").unwrap();

        let loader = ScriptLoader::new(dir.path());

        // First sync
        let result1 = loader.sync(db.pool()).await.unwrap();
        assert_eq!(result1.added, 1);

        // Second sync (no changes)
        let result2 = loader.sync(db.pool()).await.unwrap();
        assert_eq!(result2.added, 0);
        assert_eq!(result2.updated, 0);
        assert_eq!(result2.removed, 0);
    }

    #[test]
    fn test_read_script_source() {
        let dir = tempdir().unwrap();

        let content = "bbs.println(\"Hello, World!\")";
        fs::write(dir.path().join("test.lua"), content).unwrap();

        let loader = ScriptLoader::new(dir.path());
        let source = loader.read_script_source("test.lua").unwrap();

        assert_eq!(source, content);
    }

    #[test]
    fn test_ensure_scripts_dir() {
        let dir = tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts").join("games");

        let loader = ScriptLoader::new(&scripts_dir);
        assert!(!loader.scripts_dir_exists());

        loader.ensure_scripts_dir().unwrap();
        assert!(loader.scripts_dir_exists());
    }

    #[test]
    fn test_load_translations() {
        let dir = tempdir().unwrap();

        // Create script
        fs::write(dir.path().join("game.lua"), "bbs.println('hello')").unwrap();

        // Create translation file
        let i18n_content = r#"
[ja]
title = "じゃんけん"
rock = "グー"

[en]
title = "Rock-Paper-Scissors"
rock = "Rock"
"#;
        fs::write(dir.path().join("game.i18n.toml"), i18n_content).unwrap();

        let loader = ScriptLoader::new(dir.path());
        let translations = loader.load_translations("game.lua");

        // Check Japanese translations
        assert!(translations.contains_key("ja"));
        let ja = translations.get("ja").unwrap();
        assert_eq!(ja.get("title"), Some(&"じゃんけん".to_string()));
        assert_eq!(ja.get("rock"), Some(&"グー".to_string()));

        // Check English translations
        assert!(translations.contains_key("en"));
        let en = translations.get("en").unwrap();
        assert_eq!(en.get("title"), Some(&"Rock-Paper-Scissors".to_string()));
        assert_eq!(en.get("rock"), Some(&"Rock".to_string()));
    }

    #[test]
    fn test_load_translations_no_file() {
        let dir = tempdir().unwrap();

        // Create script without translation file
        fs::write(dir.path().join("game.lua"), "bbs.println('hello')").unwrap();

        let loader = ScriptLoader::new(dir.path());
        let translations = loader.load_translations("game.lua");

        // Should return empty HashMap
        assert!(translations.is_empty());
    }

    #[test]
    fn test_load_translations_invalid_file() {
        let dir = tempdir().unwrap();

        // Create script
        fs::write(dir.path().join("game.lua"), "bbs.println('hello')").unwrap();

        // Create invalid translation file
        fs::write(dir.path().join("game.i18n.toml"), "not valid toml {{{").unwrap();

        let loader = ScriptLoader::new(dir.path());
        let translations = loader.load_translations("game.lua");

        // Should return empty HashMap on parse error
        assert!(translations.is_empty());
    }
}
