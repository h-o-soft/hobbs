//! Permission checking for HOBBS.
//!
//! This module provides role-based access control (RBAC) functions
//! for checking user permissions.

use thiserror::Error;

use crate::db::{Role, User};

/// Permission-related errors.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PermissionError {
    /// User does not have sufficient permission.
    #[error("この操作には{0}以上の権限が必要です")]
    InsufficientRole(String),

    /// User is not authenticated.
    #[error("この操作にはログインが必要です")]
    NotAuthenticated,

    /// User account is not active.
    #[error("アカウントが無効化されています")]
    AccountInactive,
}

/// Check if a user has the required permission level.
///
/// This function checks:
/// 1. If authentication is required (required >= Member), the user must be Some
/// 2. If the user is Some, their account must be active
/// 3. The user's role must be >= the required role
///
/// # Arguments
///
/// * `user` - Optional reference to the current user
/// * `required` - The minimum role required for the operation
///
/// # Returns
///
/// `Ok(())` if the user has sufficient permission, or `PermissionError` otherwise.
///
/// # Examples
///
/// ```
/// use hobbs::auth::permission::{check_permission, PermissionError};
/// use hobbs::db::{Role, User};
///
/// // Guest access (no user required)
/// assert!(check_permission(None, Role::Guest).is_ok());
///
/// // Member required but no user
/// assert!(matches!(
///     check_permission(None, Role::Member),
///     Err(PermissionError::NotAuthenticated)
/// ));
/// ```
pub fn check_permission(user: Option<&User>, required: Role) -> Result<(), PermissionError> {
    // Guest-level operations don't require authentication
    if required == Role::Guest {
        // Even guests can be blocked if they have an inactive account
        if let Some(u) = user {
            if !u.is_active {
                return Err(PermissionError::AccountInactive);
            }
        }
        return Ok(());
    }

    // For non-guest operations, user must be authenticated
    let user = user.ok_or(PermissionError::NotAuthenticated)?;

    // Check if account is active
    if !user.is_active {
        return Err(PermissionError::AccountInactive);
    }

    // Check role level
    if !user.role.can_access(required) {
        return Err(PermissionError::InsufficientRole(
            required.display_name().to_string(),
        ));
    }

    Ok(())
}

/// Require at least Member role.
///
/// Convenience function that checks if the user is at least a Member.
///
/// # Examples
///
/// ```
/// use hobbs::auth::permission::{require_member, PermissionError};
/// use hobbs::db::{Role, User};
///
/// // No user -> NotAuthenticated
/// assert!(matches!(require_member(None), Err(PermissionError::NotAuthenticated)));
/// ```
pub fn require_member(user: Option<&User>) -> Result<(), PermissionError> {
    check_permission(user, Role::Member)
}

/// Require at least SubOp role.
///
/// Convenience function that checks if the user is at least a SubOp.
///
/// # Examples
///
/// ```
/// use hobbs::auth::permission::require_subop;
/// use hobbs::db::{Role, User};
///
/// // No user -> error
/// assert!(require_subop(None).is_err());
/// ```
pub fn require_subop(user: Option<&User>) -> Result<(), PermissionError> {
    check_permission(user, Role::SubOp)
}

/// Require SysOp role.
///
/// Convenience function that checks if the user is a SysOp.
///
/// # Examples
///
/// ```
/// use hobbs::auth::permission::require_sysop;
/// use hobbs::db::{Role, User};
///
/// // No user -> error
/// assert!(require_sysop(None).is_err());
/// ```
pub fn require_sysop(user: Option<&User>) -> Result<(), PermissionError> {
    check_permission(user, Role::SysOp)
}

/// Check if a user can perform an action on a resource owned by another user.
///
/// Rules:
/// - Users can always act on their own resources (if they have base permission)
/// - SubOp and above can act on other users' resources
///
/// # Arguments
///
/// * `actor` - The user trying to perform the action
/// * `owner_id` - The ID of the resource owner
/// * `base_permission` - The minimum role required for the operation on own resources
///
/// # Examples
///
/// ```ignore
/// use hobbs::auth::permission::can_modify_resource;
/// use hobbs::db::Role;
///
/// // User can modify their own resource
/// assert!(can_modify_resource(Some(&user), user.id, Role::Member).is_ok());
///
/// // SubOp can modify others' resources
/// assert!(can_modify_resource(Some(&subop), other_user.id, Role::Member).is_ok());
/// ```
pub fn can_modify_resource(
    actor: Option<&User>,
    owner_id: i64,
    base_permission: Role,
) -> Result<(), PermissionError> {
    let actor = actor.ok_or(PermissionError::NotAuthenticated)?;

    if !actor.is_active {
        return Err(PermissionError::AccountInactive);
    }

    // Check base permission first
    if !actor.role.can_access(base_permission) {
        return Err(PermissionError::InsufficientRole(
            base_permission.display_name().to_string(),
        ));
    }

    // User can modify their own resources
    if actor.id == owner_id {
        return Ok(());
    }

    // Otherwise, need SubOp or higher to modify others' resources
    if !actor.role.can_access(Role::SubOp) {
        return Err(PermissionError::InsufficientRole(
            Role::SubOp.display_name().to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::CharacterEncoding;

    fn create_test_user(role: Role, is_active: bool) -> User {
        User {
            id: 1,
            username: "testuser".to_string(),
            password: "hash".to_string(),
            nickname: "Test User".to_string(),
            email: None,
            role,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active,
        }
    }

    // check_permission tests
    #[test]
    fn test_check_permission_guest_no_user() {
        assert!(check_permission(None, Role::Guest).is_ok());
    }

    #[test]
    fn test_check_permission_guest_with_user() {
        let user = create_test_user(Role::Member, true);
        assert!(check_permission(Some(&user), Role::Guest).is_ok());
    }

    #[test]
    fn test_check_permission_guest_inactive_user() {
        let user = create_test_user(Role::Member, false);
        assert!(matches!(
            check_permission(Some(&user), Role::Guest),
            Err(PermissionError::AccountInactive)
        ));
    }

    #[test]
    fn test_check_permission_member_no_user() {
        assert!(matches!(
            check_permission(None, Role::Member),
            Err(PermissionError::NotAuthenticated)
        ));
    }

    #[test]
    fn test_check_permission_member_with_guest() {
        let user = create_test_user(Role::Guest, true);
        assert!(matches!(
            check_permission(Some(&user), Role::Member),
            Err(PermissionError::InsufficientRole(_))
        ));
    }

    #[test]
    fn test_check_permission_member_with_member() {
        let user = create_test_user(Role::Member, true);
        assert!(check_permission(Some(&user), Role::Member).is_ok());
    }

    #[test]
    fn test_check_permission_member_with_subop() {
        let user = create_test_user(Role::SubOp, true);
        assert!(check_permission(Some(&user), Role::Member).is_ok());
    }

    #[test]
    fn test_check_permission_member_with_sysop() {
        let user = create_test_user(Role::SysOp, true);
        assert!(check_permission(Some(&user), Role::Member).is_ok());
    }

    #[test]
    fn test_check_permission_subop_with_member() {
        let user = create_test_user(Role::Member, true);
        assert!(matches!(
            check_permission(Some(&user), Role::SubOp),
            Err(PermissionError::InsufficientRole(_))
        ));
    }

    #[test]
    fn test_check_permission_subop_with_subop() {
        let user = create_test_user(Role::SubOp, true);
        assert!(check_permission(Some(&user), Role::SubOp).is_ok());
    }

    #[test]
    fn test_check_permission_sysop_with_subop() {
        let user = create_test_user(Role::SubOp, true);
        assert!(matches!(
            check_permission(Some(&user), Role::SysOp),
            Err(PermissionError::InsufficientRole(_))
        ));
    }

    #[test]
    fn test_check_permission_sysop_with_sysop() {
        let user = create_test_user(Role::SysOp, true);
        assert!(check_permission(Some(&user), Role::SysOp).is_ok());
    }

    #[test]
    fn test_check_permission_inactive_user() {
        let user = create_test_user(Role::SysOp, false);
        assert!(matches!(
            check_permission(Some(&user), Role::Member),
            Err(PermissionError::AccountInactive)
        ));
    }

    // Convenience function tests
    #[test]
    fn test_require_member() {
        assert!(require_member(None).is_err());

        let guest = create_test_user(Role::Guest, true);
        assert!(require_member(Some(&guest)).is_err());

        let member = create_test_user(Role::Member, true);
        assert!(require_member(Some(&member)).is_ok());
    }

    #[test]
    fn test_require_subop() {
        assert!(require_subop(None).is_err());

        let member = create_test_user(Role::Member, true);
        assert!(require_subop(Some(&member)).is_err());

        let subop = create_test_user(Role::SubOp, true);
        assert!(require_subop(Some(&subop)).is_ok());
    }

    #[test]
    fn test_require_sysop() {
        assert!(require_sysop(None).is_err());

        let subop = create_test_user(Role::SubOp, true);
        assert!(require_sysop(Some(&subop)).is_err());

        let sysop = create_test_user(Role::SysOp, true);
        assert!(require_sysop(Some(&sysop)).is_ok());
    }

    // can_modify_resource tests
    #[test]
    fn test_can_modify_resource_own_resource() {
        let user = create_test_user(Role::Member, true);
        assert!(can_modify_resource(Some(&user), user.id, Role::Member).is_ok());
    }

    #[test]
    fn test_can_modify_resource_other_resource_as_member() {
        let user = create_test_user(Role::Member, true);
        assert!(matches!(
            can_modify_resource(Some(&user), 999, Role::Member),
            Err(PermissionError::InsufficientRole(_))
        ));
    }

    #[test]
    fn test_can_modify_resource_other_resource_as_subop() {
        let mut user = create_test_user(Role::SubOp, true);
        user.id = 1;
        assert!(can_modify_resource(Some(&user), 999, Role::Member).is_ok());
    }

    #[test]
    fn test_can_modify_resource_other_resource_as_sysop() {
        let mut user = create_test_user(Role::SysOp, true);
        user.id = 1;
        assert!(can_modify_resource(Some(&user), 999, Role::Member).is_ok());
    }

    #[test]
    fn test_can_modify_resource_no_user() {
        assert!(matches!(
            can_modify_resource(None, 1, Role::Member),
            Err(PermissionError::NotAuthenticated)
        ));
    }

    #[test]
    fn test_can_modify_resource_inactive_user() {
        let user = create_test_user(Role::SysOp, false);
        assert!(matches!(
            can_modify_resource(Some(&user), user.id, Role::Member),
            Err(PermissionError::AccountInactive)
        ));
    }

    #[test]
    fn test_can_modify_resource_insufficient_base_permission() {
        let user = create_test_user(Role::Guest, true);
        assert!(matches!(
            can_modify_resource(Some(&user), user.id, Role::Member),
            Err(PermissionError::InsufficientRole(_))
        ));
    }

    // Error display tests
    #[test]
    fn test_permission_error_display() {
        let err = PermissionError::InsufficientRole("メンバー".to_string());
        assert!(err.to_string().contains("メンバー"));

        let err = PermissionError::NotAuthenticated;
        assert!(err.to_string().contains("ログイン"));

        let err = PermissionError::AccountInactive;
        assert!(err.to_string().contains("無効化"));
    }

    // Role.can_access tests (moved from user.rs tests to permission module)
    #[test]
    fn test_role_can_access() {
        // Guest can only access Guest level
        assert!(Role::Guest.can_access(Role::Guest));
        assert!(!Role::Guest.can_access(Role::Member));
        assert!(!Role::Guest.can_access(Role::SubOp));
        assert!(!Role::Guest.can_access(Role::SysOp));

        // Member can access Guest and Member
        assert!(Role::Member.can_access(Role::Guest));
        assert!(Role::Member.can_access(Role::Member));
        assert!(!Role::Member.can_access(Role::SubOp));
        assert!(!Role::Member.can_access(Role::SysOp));

        // SubOp can access Guest, Member, SubOp
        assert!(Role::SubOp.can_access(Role::Guest));
        assert!(Role::SubOp.can_access(Role::Member));
        assert!(Role::SubOp.can_access(Role::SubOp));
        assert!(!Role::SubOp.can_access(Role::SysOp));

        // SysOp can access all
        assert!(Role::SysOp.can_access(Role::Guest));
        assert!(Role::SysOp.can_access(Role::Member));
        assert!(Role::SysOp.can_access(Role::SubOp));
        assert!(Role::SysOp.can_access(Role::SysOp));
    }

    #[test]
    fn test_role_display_name() {
        assert_eq!(Role::Guest.display_name(), "ゲスト");
        assert_eq!(Role::Member.display_name(), "メンバー");
        assert_eq!(Role::SubOp.display_name(), "副管理者");
        assert_eq!(Role::SysOp.display_name(), "管理者");
    }
}
