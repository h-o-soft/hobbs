//! Folder types and repository for HOBBS file management.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::db::Role;
use crate::Result;

/// A folder in the file library.
#[derive(Debug, Clone)]
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
    pub permission: Role,
    /// Minimum role required to upload to the folder.
    pub upload_perm: Role,
    /// Display order.
    pub order_num: i32,
    /// When the folder was created.
    pub created_at: DateTime<Utc>,
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
}

/// Repository for folder operations.
pub struct FolderRepository;

impl FolderRepository {
    /// Create a new folder.
    pub fn create(conn: &Connection, folder: &NewFolder) -> Result<Folder> {
        conn.execute(
            "INSERT INTO folders (name, description, parent_id, permission, upload_perm, order_num)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                folder.name,
                folder.description,
                folder.parent_id,
                folder.permission.as_str(),
                folder.upload_perm.as_str(),
                folder.order_num,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?
            .ok_or_else(|| crate::HobbsError::Database(rusqlite::Error::QueryReturnedNoRows))
    }

    /// Get a folder by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Folder>> {
        let folder = conn
            .query_row(
                "SELECT id, name, description, parent_id, permission, upload_perm, order_num, created_at
                 FROM folders WHERE id = ?1",
                [id],
                Self::map_row,
            )
            .optional()?;

        Ok(folder)
    }

    /// List all root folders (parent_id is NULL).
    pub fn list_root(conn: &Connection) -> Result<Vec<Folder>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, parent_id, permission, upload_perm, order_num, created_at
             FROM folders WHERE parent_id IS NULL ORDER BY order_num, id",
        )?;

        let folders: Vec<Folder> = stmt
            .query_map([], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(folders)
    }

    /// List child folders of a parent folder.
    pub fn list_by_parent(conn: &Connection, parent_id: i64) -> Result<Vec<Folder>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, parent_id, permission, upload_perm, order_num, created_at
             FROM folders WHERE parent_id = ?1 ORDER BY order_num, id",
        )?;

        let folders: Vec<Folder> = stmt
            .query_map([parent_id], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(folders)
    }

    /// List folders accessible by a given role.
    pub fn list_accessible(conn: &Connection, role: Role) -> Result<Vec<Folder>> {
        // Get all roles that are at or below the user's role
        let accessible_roles: Vec<&str> = [Role::Guest, Role::Member, Role::SubOp, Role::SysOp]
            .into_iter()
            .filter(|r| *r <= role)
            .map(|r| r.as_str())
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

        let mut stmt = conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> = accessible_roles
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let folders: Vec<Folder> = stmt
            .query_map(params.as_slice(), Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(folders)
    }

    /// Update a folder.
    pub fn update(conn: &Connection, id: i64, update: &FolderUpdate) -> Result<Option<Folder>> {
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref name) = update.name {
            updates.push("name = ?");
            params.push(Box::new(name.clone()));
        }

        if let Some(ref description) = update.description {
            updates.push("description = ?");
            params.push(Box::new(description.clone()));
        }

        if let Some(parent_id) = update.parent_id {
            updates.push("parent_id = ?");
            params.push(Box::new(parent_id));
        }

        if let Some(ref permission) = update.permission {
            updates.push("permission = ?");
            params.push(Box::new(permission.as_str().to_string()));
        }

        if let Some(ref upload_perm) = update.upload_perm {
            updates.push("upload_perm = ?");
            params.push(Box::new(upload_perm.as_str().to_string()));
        }

        if let Some(order_num) = update.order_num {
            updates.push("order_num = ?");
            params.push(Box::new(order_num));
        }

        if updates.is_empty() {
            return Self::get_by_id(conn, id);
        }

        params.push(Box::new(id));

        let query = format!("UPDATE folders SET {} WHERE id = ?", updates.join(", "));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&query, param_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Delete a folder by ID.
    pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
        let rows = conn.execute("DELETE FROM folders WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Get the depth of a folder (0 for root).
    pub fn get_depth(conn: &Connection, id: i64) -> Result<usize> {
        let mut depth = 0;
        let mut current_id = Some(id);

        while let Some(folder_id) = current_id {
            let folder = Self::get_by_id(conn, folder_id)?;
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
    pub fn get_path(conn: &Connection, id: i64) -> Result<Vec<Folder>> {
        let mut path = Vec::new();
        let mut current_id = Some(id);

        while let Some(folder_id) = current_id {
            if let Some(folder) = Self::get_by_id(conn, folder_id)? {
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
    pub fn count_files(conn: &Connection, folder_id: i64) -> Result<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE folder_id = ?1",
            [folder_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Map a database row to a Folder.
    fn map_row(row: &Row) -> rusqlite::Result<Folder> {
        let permission_str: String = row.get(4)?;
        let upload_perm_str: String = row.get(5)?;
        let created_at_str: String = row.get(7)?;

        Ok(Folder {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            parent_id: row.get(3)?,
            permission: Role::from_str(&permission_str).unwrap_or(Role::Member),
            upload_perm: Role::from_str(&upload_perm_str).unwrap_or(Role::SubOp),
            order_num: row.get(6)?,
            created_at: DateTime::parse_from_rfc3339(&format!("{created_at_str}Z"))
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    #[test]
    fn test_create_folder() {
        let db = setup_db();
        let conn = db.conn();

        let new_folder = NewFolder::new("共有ファイル")
            .with_description("会員向けファイル")
            .with_permission(Role::Member)
            .with_upload_perm(Role::SubOp);

        let folder = FolderRepository::create(conn, &new_folder).unwrap();

        assert_eq!(folder.name, "共有ファイル");
        assert_eq!(folder.description, Some("会員向けファイル".to_string()));
        assert!(folder.parent_id.is_none());
        assert_eq!(folder.permission, Role::Member);
        assert_eq!(folder.upload_perm, Role::SubOp);
    }

    #[test]
    fn test_get_folder_by_id() {
        let db = setup_db();
        let conn = db.conn();

        let new_folder = NewFolder::new("Test Folder");
        let created = FolderRepository::create(conn, &new_folder).unwrap();

        let found = FolderRepository::get_by_id(conn, created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Folder");
    }

    #[test]
    fn test_get_folder_not_found() {
        let db = setup_db();
        let conn = db.conn();

        let found = FolderRepository::get_by_id(conn, 9999).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_list_root_folders() {
        let db = setup_db();
        let conn = db.conn();

        // Create root folders
        FolderRepository::create(conn, &NewFolder::new("Root A").with_order(2)).unwrap();
        FolderRepository::create(conn, &NewFolder::new("Root B").with_order(1)).unwrap();

        let roots = FolderRepository::list_root(conn).unwrap();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].name, "Root B"); // order_num = 1
        assert_eq!(roots[1].name, "Root A"); // order_num = 2
    }

    #[test]
    fn test_list_child_folders() {
        let db = setup_db();
        let conn = db.conn();

        let parent = FolderRepository::create(conn, &NewFolder::new("Parent")).unwrap();

        FolderRepository::create(
            conn,
            &NewFolder::new("Child 1")
                .with_parent(parent.id)
                .with_order(2),
        )
        .unwrap();
        FolderRepository::create(
            conn,
            &NewFolder::new("Child 2")
                .with_parent(parent.id)
                .with_order(1),
        )
        .unwrap();

        let children = FolderRepository::list_by_parent(conn, parent.id).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "Child 2");
        assert_eq!(children[1].name, "Child 1");
    }

    #[test]
    fn test_update_folder() {
        let db = setup_db();
        let conn = db.conn();

        let folder = FolderRepository::create(conn, &NewFolder::new("Original")).unwrap();

        let update = FolderUpdate::new()
            .name("Updated")
            .description(Some("New description"))
            .permission(Role::SubOp);

        let updated = FolderRepository::update(conn, folder.id, &update)
            .unwrap()
            .unwrap();

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.description, Some("New description".to_string()));
        assert_eq!(updated.permission, Role::SubOp);
    }

    #[test]
    fn test_delete_folder() {
        let db = setup_db();
        let conn = db.conn();

        let folder = FolderRepository::create(conn, &NewFolder::new("ToDelete")).unwrap();

        let deleted = FolderRepository::delete(conn, folder.id).unwrap();
        assert!(deleted);

        let found = FolderRepository::get_by_id(conn, folder.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_delete_folder_not_found() {
        let db = setup_db();
        let conn = db.conn();

        let deleted = FolderRepository::delete(conn, 9999).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_get_depth() {
        let db = setup_db();
        let conn = db.conn();

        let root = FolderRepository::create(conn, &NewFolder::new("Root")).unwrap();
        let level1 =
            FolderRepository::create(conn, &NewFolder::new("Level1").with_parent(root.id)).unwrap();
        let level2 =
            FolderRepository::create(conn, &NewFolder::new("Level2").with_parent(level1.id))
                .unwrap();

        assert_eq!(FolderRepository::get_depth(conn, root.id).unwrap(), 0);
        assert_eq!(FolderRepository::get_depth(conn, level1.id).unwrap(), 1);
        assert_eq!(FolderRepository::get_depth(conn, level2.id).unwrap(), 2);
    }

    #[test]
    fn test_get_path() {
        let db = setup_db();
        let conn = db.conn();

        let root = FolderRepository::create(conn, &NewFolder::new("Root")).unwrap();
        let level1 =
            FolderRepository::create(conn, &NewFolder::new("Level1").with_parent(root.id)).unwrap();
        let level2 =
            FolderRepository::create(conn, &NewFolder::new("Level2").with_parent(level1.id))
                .unwrap();

        let path = FolderRepository::get_path(conn, level2.id).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].name, "Root");
        assert_eq!(path[1].name, "Level1");
        assert_eq!(path[2].name, "Level2");
    }

    #[test]
    fn test_list_accessible() {
        let db = setup_db();
        let conn = db.conn();

        FolderRepository::create(
            conn,
            &NewFolder::new("Guest Folder").with_permission(Role::Guest),
        )
        .unwrap();
        FolderRepository::create(
            conn,
            &NewFolder::new("Member Folder").with_permission(Role::Member),
        )
        .unwrap();
        FolderRepository::create(
            conn,
            &NewFolder::new("SubOp Folder").with_permission(Role::SubOp),
        )
        .unwrap();

        // Guest can only see guest folders
        let guest_folders = FolderRepository::list_accessible(conn, Role::Guest).unwrap();
        assert_eq!(guest_folders.len(), 1);

        // Member can see guest and member folders
        let member_folders = FolderRepository::list_accessible(conn, Role::Member).unwrap();
        assert_eq!(member_folders.len(), 2);

        // SubOp can see all
        let subop_folders = FolderRepository::list_accessible(conn, Role::SubOp).unwrap();
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
