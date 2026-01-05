//! Folder management for administrators.
//!
//! This module provides administrative functions for managing folders:
//! - Create folder (SubOp and above)
//! - Update folder (SubOp and above)
//! - Delete folder (SysOp only)

use crate::auth::require_sysop;
use crate::db::{Database, User};
use crate::file::{Folder, FolderRepository, FolderUpdate, NewFolder, MAX_FOLDER_DEPTH};

use super::{require_admin, AdminError};

/// Admin service for folder management.
pub struct FolderAdminService<'a> {
    db: &'a Database,
}

impl<'a> FolderAdminService<'a> {
    /// Create a new FolderAdminService.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new folder.
    ///
    /// Requires SubOp or higher permission.
    pub fn create_folder(&self, folder: &NewFolder, admin: &User) -> Result<Folder, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();

        // Check parent exists if specified
        if let Some(parent_id) = folder.parent_id {
            FolderRepository::get_by_id(conn, parent_id)?
                .ok_or_else(|| AdminError::NotFound("親フォルダ".to_string()))?;

            // Check max depth
            let parent_depth = FolderRepository::get_depth(conn, parent_id)?;
            if parent_depth >= MAX_FOLDER_DEPTH - 1 {
                return Err(AdminError::InvalidOperation(format!(
                    "フォルダの階層が深すぎます（最大{MAX_FOLDER_DEPTH}階層）"
                )));
            }
        }

        let created = FolderRepository::create(conn, folder)?;
        Ok(created)
    }

    /// Update an existing folder.
    ///
    /// Requires SubOp or higher permission.
    pub fn update_folder(
        &self,
        folder_id: i64,
        update: &FolderUpdate,
        admin: &User,
    ) -> Result<Folder, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();

        // Check if folder exists
        FolderRepository::get_by_id(conn, folder_id)?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        // If changing parent, validate
        if let Some(Some(parent_id)) = update.parent_id {
            // Check parent exists
            FolderRepository::get_by_id(conn, parent_id)?
                .ok_or_else(|| AdminError::NotFound("親フォルダ".to_string()))?;

            // Check for circular reference
            if parent_id == folder_id {
                return Err(AdminError::InvalidOperation(
                    "フォルダを自身の子として設定することはできません".to_string(),
                ));
            }

            // Check if parent is a descendant of this folder
            let parent_path = FolderRepository::get_path(conn, parent_id)?;
            if parent_path.iter().any(|f| f.id == folder_id) {
                return Err(AdminError::InvalidOperation(
                    "フォルダを自身の子孫に移動することはできません".to_string(),
                ));
            }

            // Check max depth after move
            let parent_depth = FolderRepository::get_depth(conn, parent_id)?;
            if parent_depth >= MAX_FOLDER_DEPTH - 1 {
                return Err(AdminError::InvalidOperation(format!(
                    "移動先のフォルダ階層が深すぎます（最大{MAX_FOLDER_DEPTH}階層）"
                )));
            }
        }

        let updated = FolderRepository::update(conn, folder_id, update)?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        Ok(updated)
    }

    /// Delete a folder.
    ///
    /// Requires SysOp permission.
    /// This will also delete all files and subfolders in the folder.
    pub fn delete_folder(&self, folder_id: i64, admin: &User) -> Result<bool, AdminError> {
        require_sysop(Some(admin))?;

        let conn = self.db.conn();

        // Check if folder exists
        FolderRepository::get_by_id(conn, folder_id)?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        let deleted = FolderRepository::delete(conn, folder_id)?;
        Ok(deleted)
    }

    /// Get a folder by ID.
    ///
    /// Requires SubOp or higher permission.
    pub fn get_folder(&self, folder_id: i64, admin: &User) -> Result<Folder, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();
        let folder = FolderRepository::get_by_id(conn, folder_id)?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        Ok(folder)
    }

    /// List all root folders.
    ///
    /// Requires SubOp or higher permission.
    pub fn list_root_folders(&self, admin: &User) -> Result<Vec<Folder>, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();
        let folders = FolderRepository::list_root(conn)?;
        Ok(folders)
    }

    /// List child folders of a parent folder.
    ///
    /// Requires SubOp or higher permission.
    pub fn list_child_folders(
        &self,
        parent_id: i64,
        admin: &User,
    ) -> Result<Vec<Folder>, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();

        // Check if parent exists
        FolderRepository::get_by_id(conn, parent_id)?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        let folders = FolderRepository::list_by_parent(conn, parent_id)?;
        Ok(folders)
    }

    /// Get folder path from root.
    ///
    /// Requires SubOp or higher permission.
    pub fn get_folder_path(&self, folder_id: i64, admin: &User) -> Result<Vec<Folder>, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();

        // Check if folder exists
        FolderRepository::get_by_id(conn, folder_id)?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        let path = FolderRepository::get_path(conn, folder_id)?;
        Ok(path)
    }

    /// Count files in a folder.
    ///
    /// Requires SubOp or higher permission.
    pub fn count_files(&self, folder_id: i64, admin: &User) -> Result<i64, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();

        // Check if folder exists
        FolderRepository::get_by_id(conn, folder_id)?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        let count = FolderRepository::count_files(conn, folder_id)?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Role;
    use crate::server::CharacterEncoding;

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_user(id: i64, role: Role) -> User {
        User {
            id,
            username: format!("user{id}"),
            password: "hash".to_string(),
            nickname: format!("User {id}"),
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

    #[test]
    fn test_create_folder_as_subop() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let new_folder = NewFolder::new("共有フォルダ").with_description("共有用");

        let folder = service.create_folder(&new_folder, &subop).unwrap();

        assert_eq!(folder.name, "共有フォルダ");
        assert_eq!(folder.description, Some("共有用".to_string()));
    }

    #[test]
    fn test_create_folder_with_parent() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let parent = service
            .create_folder(&NewFolder::new("親フォルダ"), &subop)
            .unwrap();

        let child_folder = NewFolder::new("子フォルダ").with_parent(parent.id);
        let child = service.create_folder(&child_folder, &subop).unwrap();

        assert_eq!(child.name, "子フォルダ");
        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[test]
    fn test_create_folder_as_member_fails() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let member = create_test_user(1, Role::Member);

        let new_folder = NewFolder::new("テスト");
        let result = service.create_folder(&new_folder, &member);

        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_create_folder_nonexistent_parent() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let new_folder = NewFolder::new("テスト").with_parent(999);
        let result = service.create_folder(&new_folder, &subop);

        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_update_folder() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let folder = service
            .create_folder(&NewFolder::new("元の名前"), &subop)
            .unwrap();

        let update = FolderUpdate::new()
            .name("新しい名前")
            .description(Some("新しい説明"));

        let updated = service.update_folder(folder.id, &update, &subop).unwrap();

        assert_eq!(updated.name, "新しい名前");
        assert_eq!(updated.description, Some("新しい説明".to_string()));
    }

    #[test]
    fn test_update_folder_move_to_new_parent() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let folder1 = service
            .create_folder(&NewFolder::new("フォルダ1"), &subop)
            .unwrap();
        let folder2 = service
            .create_folder(&NewFolder::new("フォルダ2"), &subop)
            .unwrap();

        let update = FolderUpdate::new().parent_id(Some(folder1.id));
        let updated = service.update_folder(folder2.id, &update, &subop).unwrap();

        assert_eq!(updated.parent_id, Some(folder1.id));
    }

    #[test]
    fn test_update_folder_circular_reference() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let parent = service
            .create_folder(&NewFolder::new("親"), &subop)
            .unwrap();
        let child = service
            .create_folder(&NewFolder::new("子").with_parent(parent.id), &subop)
            .unwrap();

        // Try to make parent a child of child (circular)
        let update = FolderUpdate::new().parent_id(Some(child.id));
        let result = service.update_folder(parent.id, &update, &subop);

        assert!(matches!(result, Err(AdminError::InvalidOperation(_))));
    }

    #[test]
    fn test_update_folder_self_as_parent() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let folder = service
            .create_folder(&NewFolder::new("テスト"), &subop)
            .unwrap();

        let update = FolderUpdate::new().parent_id(Some(folder.id));
        let result = service.update_folder(folder.id, &update, &subop);

        assert!(matches!(result, Err(AdminError::InvalidOperation(_))));
    }

    #[test]
    fn test_update_nonexistent_folder() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let update = FolderUpdate::new().name("新しい名前");
        let result = service.update_folder(999, &update, &subop);

        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_delete_folder_as_sysop() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);

        let folder = service
            .create_folder(&NewFolder::new("削除対象"), &sysop)
            .unwrap();

        let deleted = service.delete_folder(folder.id, &sysop).unwrap();
        assert!(deleted);

        let result = service.get_folder(folder.id, &sysop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_delete_folder_as_subop_fails() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);
        let subop = create_test_user(2, Role::SubOp);

        let folder = service
            .create_folder(&NewFolder::new("削除対象"), &sysop)
            .unwrap();

        let result = service.delete_folder(folder.id, &subop);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_delete_nonexistent_folder() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);

        let result = service.delete_folder(999, &sysop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_list_root_folders() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        service
            .create_folder(&NewFolder::new("ルート1").with_order(2), &subop)
            .unwrap();
        service
            .create_folder(&NewFolder::new("ルート2").with_order(1), &subop)
            .unwrap();

        let roots = service.list_root_folders(&subop).unwrap();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].name, "ルート2"); // order = 1
        assert_eq!(roots[1].name, "ルート1"); // order = 2
    }

    #[test]
    fn test_list_child_folders() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let parent = service
            .create_folder(&NewFolder::new("親"), &subop)
            .unwrap();
        service
            .create_folder(&NewFolder::new("子1").with_parent(parent.id), &subop)
            .unwrap();
        service
            .create_folder(&NewFolder::new("子2").with_parent(parent.id), &subop)
            .unwrap();

        let children = service.list_child_folders(parent.id, &subop).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_get_folder_path() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let root = service
            .create_folder(&NewFolder::new("ルート"), &subop)
            .unwrap();
        let level1 = service
            .create_folder(&NewFolder::new("レベル1").with_parent(root.id), &subop)
            .unwrap();
        let level2 = service
            .create_folder(&NewFolder::new("レベル2").with_parent(level1.id), &subop)
            .unwrap();

        let path = service.get_folder_path(level2.id, &subop).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].name, "ルート");
        assert_eq!(path[1].name, "レベル1");
        assert_eq!(path[2].name, "レベル2");
    }

    #[test]
    fn test_get_folder() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let created = service
            .create_folder(&NewFolder::new("テスト"), &subop)
            .unwrap();

        let folder = service.get_folder(created.id, &subop).unwrap();
        assert_eq!(folder.name, "テスト");
    }

    #[test]
    fn test_get_folder_not_found() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let result = service.get_folder(999, &subop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_count_files() {
        let db = setup_db();
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let folder = service
            .create_folder(&NewFolder::new("テスト"), &subop)
            .unwrap();

        // Initially no files
        let count = service.count_files(folder.id, &subop).unwrap();
        assert_eq!(count, 0);
    }
}
