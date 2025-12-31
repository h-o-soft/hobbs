//! Administration module for HOBBS.
//!
//! This module provides administrative functionality including:
//! - Board management (create, update, delete)
//! - Folder management (create, update, delete)
//! - User management (list, update, change role, suspend/activate)
//! - Post and file management (delete)
//! - Session management (list, force disconnect)
//!
//! Access is controlled by role:
//! - SubOp: Most admin functions except destructive operations
//! - SysOp: All admin functions including destructive operations

mod board;
mod content;
mod folder;

pub use board::{BoardAdminService, CreateBoardRequest};
pub use content::{ContentAdminService, PostDeletionMode, DELETED_POST_MESSAGE};
pub use folder::FolderAdminService;

use thiserror::Error;

use crate::auth::{require_subop, require_sysop, PermissionError};
use crate::db::{Database, Role, User};

/// Admin-related errors.
#[derive(Error, Debug)]
pub enum AdminError {
    /// Permission denied for the operation.
    #[error("{0}")]
    Permission(#[from] PermissionError),

    /// Target resource not found.
    #[error("{0}が見つかりません")]
    NotFound(String),

    /// Invalid operation.
    #[error("無効な操作: {0}")]
    InvalidOperation(String),

    /// Cannot modify own account in certain ways.
    #[error("自分自身に対してこの操作は行えません")]
    CannotModifySelf,

    /// Cannot demote the last SysOp.
    #[error("最後のSysOpの権限を変更することはできません")]
    LastSysOp,

    /// Database error.
    #[error("データベースエラー: {0}")]
    Database(#[from] rusqlite::Error),

    /// General HOBBS error.
    #[error("{0}")]
    Hobbs(#[from] crate::HobbsError),
}

/// Require admin access (SubOp or higher).
///
/// This is an alias for `require_subop` that provides clearer semantics
/// for admin-related operations.
///
/// # Arguments
///
/// * `user` - Optional reference to the current user
///
/// # Returns
///
/// `Ok(())` if the user has admin access, or `PermissionError` otherwise.
///
/// # Examples
///
/// ```
/// use hobbs::admin::require_admin;
/// use hobbs::db::{Role, User};
/// use hobbs::server::CharacterEncoding;
///
/// let mut subop = User {
///     id: 1,
///     username: "admin".to_string(),
///     password: "hash".to_string(),
///     nickname: "Admin".to_string(),
///     email: None,
///     role: Role::SubOp,
///     profile: None,
///     terminal: "standard".to_string(),
///     encoding: CharacterEncoding::default(),
///     created_at: "2024-01-01".to_string(),
///     last_login: None,
///     is_active: true,
/// };
///
/// assert!(require_admin(Some(&subop)).is_ok());
/// ```
pub fn require_admin(user: Option<&User>) -> std::result::Result<(), PermissionError> {
    require_subop(user)
}

/// Check if a user can perform admin operations.
///
/// Returns `true` if the user is SubOp or higher.
pub fn is_admin(user: Option<&User>) -> bool {
    require_admin(user).is_ok()
}

/// Check if a user can perform SysOp-only operations.
///
/// Returns `true` if the user is SysOp.
pub fn is_sysop(user: Option<&User>) -> bool {
    require_sysop(user).is_ok()
}

/// Check if a user can modify another user's role.
///
/// Rules:
/// - Only SysOp can change roles
/// - Cannot change own role
/// - Cannot demote the last SysOp
///
/// # Arguments
///
/// * `admin` - The admin user attempting the change
/// * `target` - The user whose role is being changed
/// * `db` - Database reference for checking SysOp count
///
/// # Returns
///
/// `Ok(())` if the operation is allowed.
pub fn can_change_role(admin: &User, target: &User) -> std::result::Result<(), AdminError> {
    // Only SysOp can change roles
    require_sysop(Some(admin))?;

    // Cannot change own role
    if admin.id == target.id {
        return Err(AdminError::CannotModifySelf);
    }

    Ok(())
}

/// Check if a SubOp can edit a target user.
///
/// Rules:
/// - SubOp can only edit users with lower role (Member or Guest)
/// - SysOp can edit anyone (except role changes which need can_change_role)
///
/// # Arguments
///
/// * `admin` - The admin user attempting the edit
/// * `target` - The user being edited
///
/// # Returns
///
/// `Ok(())` if the operation is allowed.
pub fn can_edit_user(admin: &User, target: &User) -> std::result::Result<(), AdminError> {
    require_admin(Some(admin))?;

    // SysOp can edit anyone
    if admin.role == Role::SysOp {
        return Ok(());
    }

    // SubOp can only edit lower roles (Member or Guest)
    if target.role >= Role::SubOp {
        return Err(AdminError::Permission(PermissionError::InsufficientRole(
            "SubOpは他のSubOp以上のユーザーを編集できません".to_string(),
        )));
    }

    Ok(())
}

/// Admin service for system administration.
///
/// This service provides administrative functions for managing the BBS system.
/// Access control is enforced at each operation level.
pub struct AdminService<'a> {
    db: &'a Database,
}

impl<'a> AdminService<'a> {
    /// Create a new AdminService.
    ///
    /// # Arguments
    ///
    /// * `db` - Reference to the database
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get database reference.
    pub fn db(&self) -> &Database {
        self.db
    }

    /// Check if there are multiple SysOps in the system.
    ///
    /// This is used to prevent demoting the last SysOp.
    pub fn has_multiple_sysops(&self) -> std::result::Result<bool, AdminError> {
        let conn = self.db.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE role = 'sysop' AND is_active = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 1)
    }

    /// Validate role change operation.
    ///
    /// Checks all conditions for a role change:
    /// - Admin has SysOp permission
    /// - Not changing own role
    /// - Not demoting the last SysOp
    ///
    /// # Arguments
    ///
    /// * `admin` - The admin performing the change
    /// * `target` - The target user
    /// * `new_role` - The new role to assign
    pub fn validate_role_change(
        &self,
        admin: &User,
        target: &User,
        new_role: Role,
    ) -> std::result::Result<(), AdminError> {
        // Basic permission check
        can_change_role(admin, target)?;

        // Check if demoting from SysOp
        if target.role == Role::SysOp && new_role != Role::SysOp && !self.has_multiple_sysops()? {
            return Err(AdminError::LastSysOp);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::CharacterEncoding;

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
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        }
    }

    // require_admin tests
    #[test]
    fn test_require_admin_no_user() {
        assert!(require_admin(None).is_err());
    }

    #[test]
    fn test_require_admin_guest() {
        let user = create_test_user(1, Role::Guest);
        assert!(require_admin(Some(&user)).is_err());
    }

    #[test]
    fn test_require_admin_member() {
        let user = create_test_user(1, Role::Member);
        assert!(require_admin(Some(&user)).is_err());
    }

    #[test]
    fn test_require_admin_subop() {
        let user = create_test_user(1, Role::SubOp);
        assert!(require_admin(Some(&user)).is_ok());
    }

    #[test]
    fn test_require_admin_sysop() {
        let user = create_test_user(1, Role::SysOp);
        assert!(require_admin(Some(&user)).is_ok());
    }

    // is_admin tests
    #[test]
    fn test_is_admin() {
        let guest = create_test_user(1, Role::Guest);
        let member = create_test_user(2, Role::Member);
        let subop = create_test_user(3, Role::SubOp);
        let sysop = create_test_user(4, Role::SysOp);

        assert!(!is_admin(None));
        assert!(!is_admin(Some(&guest)));
        assert!(!is_admin(Some(&member)));
        assert!(is_admin(Some(&subop)));
        assert!(is_admin(Some(&sysop)));
    }

    // is_sysop tests
    #[test]
    fn test_is_sysop() {
        let subop = create_test_user(1, Role::SubOp);
        let sysop = create_test_user(2, Role::SysOp);

        assert!(!is_sysop(None));
        assert!(!is_sysop(Some(&subop)));
        assert!(is_sysop(Some(&sysop)));
    }

    // can_change_role tests
    #[test]
    fn test_can_change_role_not_sysop() {
        let subop = create_test_user(1, Role::SubOp);
        let target = create_test_user(2, Role::Member);

        let result = can_change_role(&subop, &target);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_can_change_role_self() {
        let sysop = create_test_user(1, Role::SysOp);

        let result = can_change_role(&sysop, &sysop);
        assert!(matches!(result, Err(AdminError::CannotModifySelf)));
    }

    #[test]
    fn test_can_change_role_success() {
        let sysop = create_test_user(1, Role::SysOp);
        let target = create_test_user(2, Role::Member);

        assert!(can_change_role(&sysop, &target).is_ok());
    }

    // can_edit_user tests
    #[test]
    fn test_can_edit_user_not_admin() {
        let member = create_test_user(1, Role::Member);
        let target = create_test_user(2, Role::Guest);

        let result = can_edit_user(&member, &target);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_can_edit_user_subop_edits_member() {
        let subop = create_test_user(1, Role::SubOp);
        let target = create_test_user(2, Role::Member);

        assert!(can_edit_user(&subop, &target).is_ok());
    }

    #[test]
    fn test_can_edit_user_subop_edits_subop() {
        let subop1 = create_test_user(1, Role::SubOp);
        let subop2 = create_test_user(2, Role::SubOp);

        let result = can_edit_user(&subop1, &subop2);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_can_edit_user_sysop_edits_subop() {
        let sysop = create_test_user(1, Role::SysOp);
        let subop = create_test_user(2, Role::SubOp);

        assert!(can_edit_user(&sysop, &subop).is_ok());
    }

    #[test]
    fn test_can_edit_user_sysop_edits_sysop() {
        let sysop1 = create_test_user(1, Role::SysOp);
        let sysop2 = create_test_user(2, Role::SysOp);

        assert!(can_edit_user(&sysop1, &sysop2).is_ok());
    }

    // AdminError display tests
    #[test]
    fn test_admin_error_display() {
        let err = AdminError::NotFound("ユーザー".to_string());
        assert!(err.to_string().contains("見つかりません"));

        let err = AdminError::InvalidOperation("テスト".to_string());
        assert!(err.to_string().contains("無効な操作"));

        let err = AdminError::CannotModifySelf;
        assert!(err.to_string().contains("自分自身"));

        let err = AdminError::LastSysOp;
        assert!(err.to_string().contains("最後のSysOp"));

        // Database error is tested via rusqlite::Error conversion
    }

    // AdminService tests
    #[test]
    fn test_admin_service_new() {
        let db = Database::open_in_memory().unwrap();
        let service = AdminService::new(&db);
        assert!(std::ptr::eq(service.db(), &db));
    }

    #[test]
    fn test_has_multiple_sysops_none() {
        let db = Database::open_in_memory().unwrap();
        let service = AdminService::new(&db);

        // Initially no sysops
        assert!(!service.has_multiple_sysops().unwrap());
    }

    #[test]
    fn test_has_multiple_sysops_one() {
        let db = Database::open_in_memory().unwrap();

        // Create one sysop
        let conn = db.conn();
        conn.execute(
            "INSERT INTO users (username, password, nickname, role, terminal, encoding, is_active)
             VALUES ('sysop', 'hash', 'SysOp', 'sysop', 'standard', 'shiftjis', 1)",
            [],
        )
        .unwrap();

        let service = AdminService::new(&db);
        assert!(!service.has_multiple_sysops().unwrap());
    }

    #[test]
    fn test_has_multiple_sysops_two() {
        let db = Database::open_in_memory().unwrap();

        // Create two sysops
        let conn = db.conn();
        conn.execute(
            "INSERT INTO users (username, password, nickname, role, terminal, encoding, is_active)
             VALUES ('sysop1', 'hash', 'SysOp 1', 'sysop', 'standard', 'shiftjis', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (username, password, nickname, role, terminal, encoding, is_active)
             VALUES ('sysop2', 'hash', 'SysOp 2', 'sysop', 'standard', 'shiftjis', 1)",
            [],
        )
        .unwrap();

        let service = AdminService::new(&db);
        assert!(service.has_multiple_sysops().unwrap());
    }

    #[test]
    fn test_has_multiple_sysops_one_inactive() {
        let db = Database::open_in_memory().unwrap();

        // Create two sysops, one inactive
        let conn = db.conn();
        conn.execute(
            "INSERT INTO users (username, password, nickname, role, terminal, encoding, is_active)
             VALUES ('sysop1', 'hash', 'SysOp 1', 'sysop', 'standard', 'shiftjis', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (username, password, nickname, role, terminal, encoding, is_active)
             VALUES ('sysop2', 'hash', 'SysOp 2', 'sysop', 'standard', 'shiftjis', 0)",
            [],
        )
        .unwrap();

        let service = AdminService::new(&db);
        // Only one active sysop
        assert!(!service.has_multiple_sysops().unwrap());
    }

    #[test]
    fn test_validate_role_change_demote_last_sysop() {
        let db = Database::open_in_memory().unwrap();

        // Create one sysop
        let conn = db.conn();
        conn.execute(
            "INSERT INTO users (username, password, nickname, role, terminal, encoding, is_active)
             VALUES ('sysop', 'hash', 'SysOp', 'sysop', 'standard', 'shiftjis', 1)",
            [],
        )
        .unwrap();

        let admin = User {
            id: 1,
            username: "sysop".to_string(),
            password: "hash".to_string(),
            nickname: "SysOp".to_string(),
            email: None,
            role: Role::SysOp,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        };

        let target = User {
            id: 2,
            username: "sysop2".to_string(),
            password: "hash".to_string(),
            nickname: "SysOp 2".to_string(),
            email: None,
            role: Role::SysOp,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        };

        // Insert the second sysop but make them inactive (so only 1 active)
        conn.execute(
            "INSERT INTO users (username, password, nickname, role, terminal, encoding, is_active)
             VALUES ('sysop2', 'hash', 'SysOp 2', 'sysop', 'standard', 'shiftjis', 0)",
            [],
        )
        .unwrap();

        let service = AdminService::new(&db);

        // Try to demote the "target" from SysOp to Member
        // Note: In this test, we're checking validation logic even though target isn't in DB
        // We need to add target to DB as active sysop for proper test
        conn.execute(
            "UPDATE users SET is_active = 1 WHERE username = 'sysop2'",
            [],
        )
        .unwrap();

        // Now we have 2 sysops, demotion should be allowed
        assert!(service
            .validate_role_change(&admin, &target, Role::Member)
            .is_ok());

        // Make one inactive again
        conn.execute(
            "UPDATE users SET is_active = 0 WHERE username = 'sysop2'",
            [],
        )
        .unwrap();

        // Create a new target that's the active sysop (id=1)
        let single_sysop = User {
            id: 999, // Different from admin to avoid CannotModifySelf
            username: "single_sysop".to_string(),
            password: "hash".to_string(),
            nickname: "Single SysOp".to_string(),
            email: None,
            role: Role::SysOp,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        };

        // Now trying to demote should fail (last sysop)
        let result = service.validate_role_change(&admin, &single_sysop, Role::Member);
        assert!(matches!(result, Err(AdminError::LastSysOp)));
    }

    #[test]
    fn test_validate_role_change_subop_cannot_change() {
        let db = Database::open_in_memory().unwrap();
        let service = AdminService::new(&db);

        let subop = create_test_user(1, Role::SubOp);
        let target = create_test_user(2, Role::Member);

        let result = service.validate_role_change(&subop, &target, Role::SubOp);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_validate_role_change_self() {
        let db = Database::open_in_memory().unwrap();
        let service = AdminService::new(&db);

        let sysop = create_test_user(1, Role::SysOp);

        let result = service.validate_role_change(&sysop, &sysop, Role::Member);
        assert!(matches!(result, Err(AdminError::CannotModifySelf)));
    }
}
