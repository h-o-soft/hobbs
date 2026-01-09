//! Folder types and repository for HOBBS file management.

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, SqlitePool};

use crate::db::Role;
use crate::{HobbsError, Result};

/// A folder in the file library.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Folder {
    /// Unique folder ID.
    pub id: i64,
    /// Folder name.
    pub name: String,
    /// Folder description.
    pub description: Option<String>,
    /// Parent folder ID (None for root folders).
    pub parent_id: Option<i64>,
    /// Minimum role required to view the folder.
    #[sqlx(try_from = "String")]
    pub permission: Role,
    /// Minimum role required to upload to the folder.
    #[sqlx(try_from = "String")]
    pub upload_perm: Role,
    /// Display order.
    pub order_num: i32,
    /// When the folder was created.
    pub created_at: String,
}

impl Folder {
    /// Get the created_at as DateTime<Utc>.
    pub fn created_at_datetime(&self) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!("{}Z", self.created_at))
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    }
}

/// Data for creating a new folder.
#[derive(Debug, Clone)]
pub struct NewFolder {
    /// Folder name.
    pub name: String,
    /// Folder description.
    pub description: Option<String>,
    /// Parent folder ID (None for root folders).
    pub parent_id: Option<i64>,
    /// Minimum role required to view the folder.
    pub permission: Role,
    /// Minimum role required to upload to the folder.
    pub upload_perm: Role,
    /// Display order.
    pub order_num: i32,
}

impl NewFolder {
    /// Create a new NewFolder with default permissions.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            parent_id: None,
            permission: Role::Member,
            upload_perm: Role::SubOp,
            order_num: 0,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the parent folder.
    pub fn with_parent(mut self, parent_id: i64) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the view permission.
    pub fn with_permission(mut self, permission: Role) -> Self {
        self.permission = permission;
        self
    }

    /// Set the upload permission.
    pub fn with_upload_perm(mut self, upload_perm: Role) -> Self {
        self.upload_perm = upload_perm;
        self
    }

    /// Set the display order.
    pub fn with_order(mut self, order_num: i32) -> Self {
        self.order_num = order_num;
        self
    }
}

/// Builder for updating a folder.
#[derive(Debug, Clone, Default)]
pub struct FolderUpdate {
    /// New folder name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<Option<String>>,
    /// New parent folder ID.
    pub parent_id: Option<Option<i64>>,
    /// New view permission.
    pub permission: Option<Role>,
    /// New upload permission.
    pub upload_perm: Option<Role>,
    /// New display order.
    pub order_num: Option<i32>,
}

impl FolderUpdate {
    /// Create a new FolderUpdate.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = Some(description.map(|s| s.into()));
        self
    }

    /// Set the parent folder ID.
    pub fn parent_id(mut self, parent_id: Option<i64>) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the view permission.
    pub fn permission(mut self, permission: Role) -> Self {
        self.permission = Some(permission);
        self
    }

    /// Set the upload permission.
    pub fn upload_perm(mut self, upload_perm: Role) -> Self {
        self.upload_perm = Some(upload_perm);
        self
    }

    /// Set the display order.
    pub fn order_num(mut self, order_num: i32) -> Self {
        self.order_num = Some(order_num);
        self
    }

    /// Check if any fields are set.
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.description.is_none()
            && self.parent_id.is_none()
            && self.permission.is_none()
            && self.upload_perm.is_none()
            && self.order_num.is_none()
    }
}

/// Repository for folder operations.
pub struct FolderRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> FolderRepository<'a> {
    /// Create a new FolderRepository with the given database pool reference.
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new folder.
    pub async fn create(&self, folder: &NewFolder) -> Result<Folder> {
        let result = sqlx::query(
            "INSERT INTO folders (name, description, parent_id, permission, upload_perm, order_num)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&folder.name)
        .bind(&folder.description)
        .bind(folder.parent_id)
        .bind(folder.permission.as_str())
        .bind(folder.upload_perm.as_str())
        .bind(folder.order_num)
        .execute(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        let id = result.last_insert_rowid();
        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("folder".to_string()))
    }

    /// Get a folder by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Folder>> {
        let folder = sqlx::query_as::<_, Folder>(
            "SELECT id, name, description, parent_id, permission, upload_perm, order_num, created_at
             FROM folders WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(folder)
    }

    /// List all root folders (parent_id is NULL).
    pub async fn list_root(&self) -> Result<Vec<Folder>> {
        let folders = sqlx::query_as::<_, Folder>(
            "SELECT id, name, description, parent_id, permission, upload_perm, order_num, created_at
             FROM folders WHERE parent_id IS NULL ORDER BY order_num, id",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(folders)
    }

    /// List child folders of a parent folder.
    pub async fn list_by_parent(&self, parent_id: i64) -> Result<Vec<Folder>> {
        let folders = sqlx::query_as::<_, Folder>(
            "SELECT id, name, description, parent_id, permission, upload_perm, order_num, created_at
             FROM folders WHERE parent_id = ? ORDER BY order_num, id",
        )
        .bind(parent_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(folders)
    }

    /// List folders accessible by a given role.
    pub async fn list_accessible(&self, role: Role) -> Result<Vec<Folder>> {
        // Get all roles that are at or below the user's role
        let accessible_roles: Vec<String> = [Role::Guest, Role::Member, Role::SubOp, Role::SysOp]
            .into_iter()
            .filter(|r| *r <= role)
            .map(|r| r.as_str().to_string())
            .collect();

        let placeholders: String = accessible_roles
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT id, name, description, parent_id, permission, upload_perm, order_num, created_at
             FROM folders WHERE permission IN ({placeholders}) ORDER BY order_num, id"
        );

        let mut query_builder = sqlx::query_as::<_, Folder>(&query);
        for role_str in &accessible_roles {
            query_builder = query_builder.bind(role_str);
        }

        let folders = query_builder
            .fetch_all(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(folders)
    }

    /// Update a folder.
    pub async fn update(&self, id: i64, update: &FolderUpdate) -> Result<Option<Folder>> {
        if update.is_empty() {
            return self.get_by_id(id).await;
        }

        let mut query: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("UPDATE folders SET ");
        let mut separated = query.separated(", ");

        if let Some(ref name) = update.name {
            separated.push("name = ");
            separated.push_bind_unseparated(name);
        }

        if let Some(ref description) = update.description {
            separated.push("description = ");
            separated.push_bind_unseparated(description.clone());
        }

        if let Some(parent_id) = update.parent_id {
            separated.push("parent_id = ");
            separated.push_bind_unseparated(parent_id);
        }

        if let Some(ref permission) = update.permission {
            separated.push("permission = ");
            separated.push_bind_unseparated(permission.as_str().to_string());
        }

        if let Some(ref upload_perm) = update.upload_perm {
            separated.push("upload_perm = ");
            separated.push_bind_unseparated(upload_perm.as_str().to_string());
        }

        if let Some(order_num) = update.order_num {
            separated.push("order_num = ");
            separated.push_bind_unseparated(order_num);
        }

        query.push(" WHERE id = ");
        query.push_bind(id);

        let result = query
            .build()
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_by_id(id).await
    }

    /// Delete a folder by ID.
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM folders WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get the depth of a folder (0 for root).
    pub async fn get_depth(&self, id: i64) -> Result<usize> {
        let mut depth = 0;
        let mut current_id = Some(id);

        while let Some(folder_id) = current_id {
            let folder = self.get_by_id(folder_id).await?;
            match folder {
                Some(f) => {
                    current_id = f.parent_id;
                    if current_id.is_some() {
                        depth += 1;
                    }
                }
                None => break,
            }
        }

        Ok(depth)
    }

    /// Get the path from root to a folder.
    pub async fn get_path(&self, id: i64) -> Result<Vec<Folder>> {
        let mut path = Vec::new();
        let mut current_id = Some(id);

        while let Some(folder_id) = current_id {
            if let Some(folder) = self.get_by_id(folder_id).await? {
                current_id = folder.parent_id;
                path.push(folder);
            } else {
                break;
            }
        }

        path.reverse();
        Ok(path)
    }

    /// Count files in a folder.
    pub async fn count_files(&self, folder_id: i64) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files WHERE folder_id = ?")
            .bind(folder_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_create_folder() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let new_folder = NewFolder::new("共有ファイル")
            .with_description("会員向けファイル")
            .with_permission(Role::Member)
            .with_upload_perm(Role::SubOp);

        let folder = repo.create(&new_folder).await.unwrap();

        assert_eq!(folder.name, "共有ファイル");
        assert_eq!(folder.description, Some("会員向けファイル".to_string()));
        assert!(folder.parent_id.is_none());
        assert_eq!(folder.permission, Role::Member);
        assert_eq!(folder.upload_perm, Role::SubOp);
    }

    #[tokio::test]
    async fn test_get_folder_by_id() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let new_folder = NewFolder::new("Test Folder");
        let created = repo.create(&new_folder).await.unwrap();

        let found = repo.get_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Folder");
    }

    #[tokio::test]
    async fn test_get_folder_not_found() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let found = repo.get_by_id(9999).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_root_folders() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        // Create root folders
        repo.create(&NewFolder::new("Root A").with_order(2))
            .await
            .unwrap();
        repo.create(&NewFolder::new("Root B").with_order(1))
            .await
            .unwrap();

        let roots = repo.list_root().await.unwrap();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].name, "Root B"); // order_num = 1
        assert_eq!(roots[1].name, "Root A"); // order_num = 2
    }

    #[tokio::test]
    async fn test_list_child_folders() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let parent = repo.create(&NewFolder::new("Parent")).await.unwrap();

        repo.create(
            &NewFolder::new("Child 1")
                .with_parent(parent.id)
                .with_order(2),
        )
        .await
        .unwrap();
        repo.create(
            &NewFolder::new("Child 2")
                .with_parent(parent.id)
                .with_order(1),
        )
        .await
        .unwrap();

        let children = repo.list_by_parent(parent.id).await.unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "Child 2");
        assert_eq!(children[1].name, "Child 1");
    }

    #[tokio::test]
    async fn test_update_folder() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let folder = repo.create(&NewFolder::new("Original")).await.unwrap();

        let update = FolderUpdate::new()
            .name("Updated")
            .description(Some("New description"))
            .permission(Role::SubOp);

        let updated = repo.update(folder.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.description, Some("New description".to_string()));
        assert_eq!(updated.permission, Role::SubOp);
    }

    #[tokio::test]
    async fn test_delete_folder() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let folder = repo.create(&NewFolder::new("ToDelete")).await.unwrap();

        let deleted = repo.delete(folder.id).await.unwrap();
        assert!(deleted);

        let found = repo.get_by_id(folder.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_folder_not_found() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let deleted = repo.delete(9999).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_get_depth() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let root = repo.create(&NewFolder::new("Root")).await.unwrap();
        let level1 = repo
            .create(&NewFolder::new("Level1").with_parent(root.id))
            .await
            .unwrap();
        let level2 = repo
            .create(&NewFolder::new("Level2").with_parent(level1.id))
            .await
            .unwrap();

        assert_eq!(repo.get_depth(root.id).await.unwrap(), 0);
        assert_eq!(repo.get_depth(level1.id).await.unwrap(), 1);
        assert_eq!(repo.get_depth(level2.id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_path() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        let root = repo.create(&NewFolder::new("Root")).await.unwrap();
        let level1 = repo
            .create(&NewFolder::new("Level1").with_parent(root.id))
            .await
            .unwrap();
        let level2 = repo
            .create(&NewFolder::new("Level2").with_parent(level1.id))
            .await
            .unwrap();

        let path = repo.get_path(level2.id).await.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].name, "Root");
        assert_eq!(path[1].name, "Level1");
        assert_eq!(path[2].name, "Level2");
    }

    #[tokio::test]
    async fn test_list_accessible() {
        let db = setup_db().await;
        let repo = FolderRepository::new(db.pool());

        repo.create(&NewFolder::new("Guest Folder").with_permission(Role::Guest))
            .await
            .unwrap();
        repo.create(&NewFolder::new("Member Folder").with_permission(Role::Member))
            .await
            .unwrap();
        repo.create(&NewFolder::new("SubOp Folder").with_permission(Role::SubOp))
            .await
            .unwrap();

        // Guest can only see guest folders
        let guest_folders = repo.list_accessible(Role::Guest).await.unwrap();
        assert_eq!(guest_folders.len(), 1);

        // Member can see guest and member folders
        let member_folders = repo.list_accessible(Role::Member).await.unwrap();
        assert_eq!(member_folders.len(), 2);

        // SubOp can see all
        let subop_folders = repo.list_accessible(Role::SubOp).await.unwrap();
        assert_eq!(subop_folders.len(), 3);
    }

    #[test]
    fn test_new_folder_builder() {
        let folder = NewFolder::new("Test")
            .with_description("Description")
            .with_parent(5)
            .with_permission(Role::SubOp)
            .with_upload_perm(Role::SysOp)
            .with_order(10);

        assert_eq!(folder.name, "Test");
        assert_eq!(folder.description, Some("Description".to_string()));
        assert_eq!(folder.parent_id, Some(5));
        assert_eq!(folder.permission, Role::SubOp);
        assert_eq!(folder.upload_perm, Role::SysOp);
        assert_eq!(folder.order_num, 10);
    }

    #[test]
    fn test_folder_update_builder() {
        let update = FolderUpdate::new()
            .name("New Name")
            .description(Some("New Desc"))
            .parent_id(Some(3))
            .permission(Role::Member)
            .upload_perm(Role::SubOp)
            .order_num(5);

        assert_eq!(update.name, Some("New Name".to_string()));
        assert_eq!(update.description, Some(Some("New Desc".to_string())));
        assert_eq!(update.parent_id, Some(Some(3)));
        assert_eq!(update.permission, Some(Role::Member));
        assert_eq!(update.upload_perm, Some(Role::SubOp));
        assert_eq!(update.order_num, Some(5));
    }
}
