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
    pub async fn create_folder(
        &self,
        folder: &NewFolder,
        admin: &User,
    ) -> Result<Folder, AdminError> {
        require_admin(Some(admin))?;

        let repo = FolderRepository::new(self.db.pool());

        // Check parent exists if specified
        if let Some(parent_id) = folder.parent_id {
            repo.get_by_id(parent_id)
                .await?
                .ok_or_else(|| AdminError::NotFound("親フォルダ".to_string()))?;

            // Check max depth
            let parent_depth = repo.get_depth(parent_id).await?;
            if parent_depth >= MAX_FOLDER_DEPTH - 1 {
                return Err(AdminError::InvalidOperation(format!(
                    "フォルダの階層が深すぎます（最大{MAX_FOLDER_DEPTH}階層）"
                )));
            }
        }

        let created = repo.create(folder).await?;
        Ok(created)
    }

    /// Update an existing folder.
    ///
    /// Requires SubOp or higher permission.
    pub async fn update_folder(
        &self,
        folder_id: i64,
        update: &FolderUpdate,
        admin: &User,
    ) -> Result<Folder, AdminError> {
        require_admin(Some(admin))?;

        let repo = FolderRepository::new(self.db.pool());

        // Check if folder exists
        repo.get_by_id(folder_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        // If changing parent, validate
        if let Some(Some(parent_id)) = update.parent_id {
            // Check parent exists
            repo.get_by_id(parent_id)
                .await?
                .ok_or_else(|| AdminError::NotFound("親フォルダ".to_string()))?;

            // Check for circular reference
            if parent_id == folder_id {
                return Err(AdminError::InvalidOperation(
                    "フォルダを自身の子として設定することはできません".to_string(),
                ));
            }

            // Check if parent is a descendant of this folder
            let parent_path = repo.get_path(parent_id).await?;
            if parent_path.iter().any(|f| f.id == folder_id) {
                return Err(AdminError::InvalidOperation(
                    "フォルダを自身の子孫に移動することはできません".to_string(),
                ));
            }

            // Check max depth after move
            let parent_depth = repo.get_depth(parent_id).await?;
            if parent_depth >= MAX_FOLDER_DEPTH - 1 {
                return Err(AdminError::InvalidOperation(format!(
                    "移動先のフォルダ階層が深すぎます（最大{MAX_FOLDER_DEPTH}階層）"
                )));
            }
        }

        let updated = repo
            .update(folder_id, update)
            .await?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        Ok(updated)
    }

    /// Delete a folder.
    ///
    /// Requires SysOp permission.
    /// This will also delete all files and subfolders in the folder.
    pub async fn delete_folder(&self, folder_id: i64, admin: &User) -> Result<bool, AdminError> {
        require_sysop(Some(admin))?;

        let repo = FolderRepository::new(self.db.pool());

        // Check if folder exists
        repo.get_by_id(folder_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        let deleted = repo.delete(folder_id).await?;
        Ok(deleted)
    }

    /// Get a folder by ID.
    ///
    /// Requires SubOp or higher permission.
    pub async fn get_folder(&self, folder_id: i64, admin: &User) -> Result<Folder, AdminError> {
        require_admin(Some(admin))?;

        let repo = FolderRepository::new(self.db.pool());
        let folder = repo
            .get_by_id(folder_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        Ok(folder)
    }

    /// List all root folders.
    ///
    /// Requires SubOp or higher permission.
    pub async fn list_root_folders(&self, admin: &User) -> Result<Vec<Folder>, AdminError> {
        require_admin(Some(admin))?;

        let repo = FolderRepository::new(self.db.pool());
        let folders = repo.list_root().await?;
        Ok(folders)
    }

    /// List child folders of a parent folder.
    ///
    /// Requires SubOp or higher permission.
    pub async fn list_child_folders(
        &self,
        parent_id: i64,
        admin: &User,
    ) -> Result<Vec<Folder>, AdminError> {
        require_admin(Some(admin))?;

        let repo = FolderRepository::new(self.db.pool());

        // Check if parent exists
        repo.get_by_id(parent_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        let folders = repo.list_by_parent(parent_id).await?;
        Ok(folders)
    }

    /// Get folder path from root.
    ///
    /// Requires SubOp or higher permission.
    pub async fn get_folder_path(
        &self,
        folder_id: i64,
        admin: &User,
    ) -> Result<Vec<Folder>, AdminError> {
        require_admin(Some(admin))?;

        let repo = FolderRepository::new(self.db.pool());

        // Check if folder exists
        repo.get_by_id(folder_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        let path = repo.get_path(folder_id).await?;
        Ok(path)
    }

    /// Count files in a folder.
    ///
    /// Requires SubOp or higher permission.
    pub async fn count_files(&self, folder_id: i64, admin: &User) -> Result<i64, AdminError> {
        require_admin(Some(admin))?;

        let repo = FolderRepository::new(self.db.pool());

        // Check if folder exists
        repo.get_by_id(folder_id)
            .await?
            .ok_or_else(|| AdminError::NotFound("フォルダ".to_string()))?;

        let count = repo.count_files(folder_id).await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Role;
    use crate::server::CharacterEncoding;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
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

    #[tokio::test]
    async fn test_create_folder_as_subop() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let new_folder = NewFolder::new("共有フォルダ").with_description("共有用");

        let folder = service.create_folder(&new_folder, &subop).await.unwrap();

        assert_eq!(folder.name, "共有フォルダ");
        assert_eq!(folder.description, Some("共有用".to_string()));
    }

    #[tokio::test]
    async fn test_create_folder_with_parent() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let parent = service
            .create_folder(&NewFolder::new("親フォルダ"), &subop)
            .await
            .unwrap();

        let child_folder = NewFolder::new("子フォルダ").with_parent(parent.id);
        let child = service.create_folder(&child_folder, &subop).await.unwrap();

        assert_eq!(child.name, "子フォルダ");
        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[tokio::test]
    async fn test_create_folder_as_member_fails() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let member = create_test_user(1, Role::Member);

        let new_folder = NewFolder::new("テスト");
        let result = service.create_folder(&new_folder, &member).await;

        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[tokio::test]
    async fn test_create_folder_nonexistent_parent() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let new_folder = NewFolder::new("テスト").with_parent(999);
        let result = service.create_folder(&new_folder, &subop).await;

        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_update_folder() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let folder = service
            .create_folder(&NewFolder::new("元の名前"), &subop)
            .await
            .unwrap();

        let update = FolderUpdate::new()
            .name("新しい名前")
            .description(Some("新しい説明"));

        let updated = service
            .update_folder(folder.id, &update, &subop)
            .await
            .unwrap();

        assert_eq!(updated.name, "新しい名前");
        assert_eq!(updated.description, Some("新しい説明".to_string()));
    }

    #[tokio::test]
    async fn test_update_folder_move_to_new_parent() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let folder1 = service
            .create_folder(&NewFolder::new("フォルダ1"), &subop)
            .await
            .unwrap();
        let folder2 = service
            .create_folder(&NewFolder::new("フォルダ2"), &subop)
            .await
            .unwrap();

        let update = FolderUpdate::new().parent_id(Some(folder1.id));
        let updated = service
            .update_folder(folder2.id, &update, &subop)
            .await
            .unwrap();

        assert_eq!(updated.parent_id, Some(folder1.id));
    }

    #[tokio::test]
    async fn test_update_folder_circular_reference() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let parent = service
            .create_folder(&NewFolder::new("親"), &subop)
            .await
            .unwrap();
        let child = service
            .create_folder(&NewFolder::new("子").with_parent(parent.id), &subop)
            .await
            .unwrap();

        // Try to make parent a child of child (circular)
        let update = FolderUpdate::new().parent_id(Some(child.id));
        let result = service.update_folder(parent.id, &update, &subop).await;

        assert!(matches!(result, Err(AdminError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_update_folder_self_as_parent() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let folder = service
            .create_folder(&NewFolder::new("テスト"), &subop)
            .await
            .unwrap();

        let update = FolderUpdate::new().parent_id(Some(folder.id));
        let result = service.update_folder(folder.id, &update, &subop).await;

        assert!(matches!(result, Err(AdminError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_update_nonexistent_folder() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let update = FolderUpdate::new().name("新しい名前");
        let result = service.update_folder(999, &update, &subop).await;

        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_folder_as_sysop() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);

        let folder = service
            .create_folder(&NewFolder::new("削除対象"), &sysop)
            .await
            .unwrap();

        let deleted = service.delete_folder(folder.id, &sysop).await.unwrap();
        assert!(deleted);

        let result = service.get_folder(folder.id, &sysop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_folder_as_subop_fails() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);
        let subop = create_test_user(2, Role::SubOp);

        let folder = service
            .create_folder(&NewFolder::new("削除対象"), &sysop)
            .await
            .unwrap();

        let result = service.delete_folder(folder.id, &subop).await;
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_folder() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);

        let result = service.delete_folder(999, &sysop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_root_folders() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        service
            .create_folder(&NewFolder::new("ルート1").with_order(2), &subop)
            .await
            .unwrap();
        service
            .create_folder(&NewFolder::new("ルート2").with_order(1), &subop)
            .await
            .unwrap();

        let roots = service.list_root_folders(&subop).await.unwrap();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].name, "ルート2"); // order = 1
        assert_eq!(roots[1].name, "ルート1"); // order = 2
    }

    #[tokio::test]
    async fn test_list_child_folders() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let parent = service
            .create_folder(&NewFolder::new("親"), &subop)
            .await
            .unwrap();
        service
            .create_folder(&NewFolder::new("子1").with_parent(parent.id), &subop)
            .await
            .unwrap();
        service
            .create_folder(&NewFolder::new("子2").with_parent(parent.id), &subop)
            .await
            .unwrap();

        let children = service.list_child_folders(parent.id, &subop).await.unwrap();
        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_get_folder_path() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let root = service
            .create_folder(&NewFolder::new("ルート"), &subop)
            .await
            .unwrap();
        let level1 = service
            .create_folder(&NewFolder::new("レベル1").with_parent(root.id), &subop)
            .await
            .unwrap();
        let level2 = service
            .create_folder(&NewFolder::new("レベル2").with_parent(level1.id), &subop)
            .await
            .unwrap();

        let path = service.get_folder_path(level2.id, &subop).await.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].name, "ルート");
        assert_eq!(path[1].name, "レベル1");
        assert_eq!(path[2].name, "レベル2");
    }

    #[tokio::test]
    async fn test_get_folder() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let created = service
            .create_folder(&NewFolder::new("テスト"), &subop)
            .await
            .unwrap();

        let folder = service.get_folder(created.id, &subop).await.unwrap();
        assert_eq!(folder.name, "テスト");
    }

    #[tokio::test]
    async fn test_get_folder_not_found() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let result = service.get_folder(999, &subop).await;
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_count_files() {
        let db = setup_db().await;
        let service = FolderAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let folder = service
            .create_folder(&NewFolder::new("テスト"), &subop)
            .await
            .unwrap();

        // Initially no files
        let count = service.count_files(folder.id, &subop).await.unwrap();
        assert_eq!(count, 0);
    }
}
