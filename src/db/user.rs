//! User model for HOBBS.
//!
//! This module defines the User struct and Role enum for user management.

use std::fmt;
use std::str::FromStr;

/// User role for permission management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Role {
    /// Guest user (not registered).
    Guest = 0,
    /// Regular member.
    #[default]
    Member = 1,
    /// Sub-operator (moderator).
    SubOp = 2,
    /// System operator (administrator).
    SysOp = 3,
}

impl Role {
    /// Convert role to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Guest => "guest",
            Role::Member => "member",
            Role::SubOp => "subop",
            Role::SysOp => "sysop",
        }
    }

    /// Get display name for the role.
    pub fn display_name(&self) -> &'static str {
        match self {
            Role::Guest => "ゲスト",
            Role::Member => "メンバー",
            Role::SubOp => "副管理者",
            Role::SysOp => "管理者",
        }
    }

    /// Check if this role has at least the required permission level.
    ///
    /// # Examples
    ///
    /// ```
    /// use hobbs::db::Role;
    ///
    /// assert!(Role::SysOp.can_access(Role::Member));
    /// assert!(Role::Member.can_access(Role::Member));
    /// assert!(!Role::Guest.can_access(Role::Member));
    /// ```
    pub fn can_access(&self, required: Role) -> bool {
        *self >= required
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "guest" => Ok(Role::Guest),
            "member" => Ok(Role::Member),
            "subop" => Ok(Role::SubOp),
            "sysop" => Ok(Role::SysOp),
            _ => Err(format!("unknown role: {s}")),
        }
    }
}

/// User entity representing a registered user.
#[derive(Debug, Clone)]
pub struct User {
    /// Unique user ID.
    pub id: i64,
    /// Login username (unique).
    pub username: String,
    /// Password hash (Argon2).
    pub password: String,
    /// Display name / handle.
    pub nickname: String,
    /// Email address (optional).
    pub email: Option<String>,
    /// User role for permissions.
    pub role: Role,
    /// Self-introduction text (optional).
    pub profile: Option<String>,
    /// Terminal profile preference.
    pub terminal: String,
    /// Account creation timestamp.
    pub created_at: String,
    /// Last login timestamp (optional).
    pub last_login: Option<String>,
    /// Whether the account is active.
    pub is_active: bool,
}

impl User {
    /// Check if this user has at least the required role level.
    pub fn has_role(&self, required: Role) -> bool {
        self.role >= required
    }

    /// Check if this user is a system operator.
    pub fn is_sysop(&self) -> bool {
        self.role == Role::SysOp
    }

    /// Check if this user is a sub-operator or higher.
    pub fn is_operator(&self) -> bool {
        self.role >= Role::SubOp
    }
}

/// Data for creating a new user.
#[derive(Debug, Clone)]
pub struct NewUser {
    /// Login username.
    pub username: String,
    /// Password hash (should be pre-hashed with Argon2).
    pub password: String,
    /// Display name / handle.
    pub nickname: String,
    /// Email address (optional).
    pub email: Option<String>,
    /// User role (defaults to Member).
    pub role: Role,
    /// Terminal profile preference (defaults to "standard").
    pub terminal: String,
}

impl NewUser {
    /// Create a new user with minimal required fields.
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        nickname: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            nickname: nickname.into(),
            email: None,
            role: Role::Member,
            terminal: "standard".to_string(),
        }
    }

    /// Set the email address.
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the role.
    pub fn with_role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Set the terminal profile.
    pub fn with_terminal(mut self, terminal: impl Into<String>) -> Self {
        self.terminal = terminal.into();
        self
    }
}

/// Data for updating an existing user.
#[derive(Debug, Clone, Default)]
pub struct UserUpdate {
    /// New password hash (if changing password).
    pub password: Option<String>,
    /// New nickname.
    pub nickname: Option<String>,
    /// New email address.
    pub email: Option<Option<String>>,
    /// New role.
    pub role: Option<Role>,
    /// New profile text.
    pub profile: Option<Option<String>>,
    /// New terminal preference.
    pub terminal: Option<String>,
    /// New active status.
    pub is_active: Option<bool>,
}

impl UserUpdate {
    /// Create an empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set new password.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set new nickname.
    pub fn nickname(mut self, nickname: impl Into<String>) -> Self {
        self.nickname = Some(nickname.into());
        self
    }

    /// Set new email.
    pub fn email(mut self, email: Option<String>) -> Self {
        self.email = Some(email);
        self
    }

    /// Set new role.
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// Set new profile.
    pub fn profile(mut self, profile: Option<String>) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Set new terminal preference.
    pub fn terminal(mut self, terminal: impl Into<String>) -> Self {
        self.terminal = Some(terminal.into());
        self
    }

    /// Set active status.
    pub fn is_active(mut self, is_active: bool) -> Self {
        self.is_active = Some(is_active);
        self
    }

    /// Check if any fields are set.
    pub fn is_empty(&self) -> bool {
        self.password.is_none()
            && self.nickname.is_none()
            && self.email.is_none()
            && self.role.is_none()
            && self.profile.is_none()
            && self.terminal.is_none()
            && self.is_active.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_ordering() {
        assert!(Role::Guest < Role::Member);
        assert!(Role::Member < Role::SubOp);
        assert!(Role::SubOp < Role::SysOp);
    }

    #[test]
    fn test_role_from_str() {
        assert_eq!(Role::from_str("guest").unwrap(), Role::Guest);
        assert_eq!(Role::from_str("member").unwrap(), Role::Member);
        assert_eq!(Role::from_str("subop").unwrap(), Role::SubOp);
        assert_eq!(Role::from_str("sysop").unwrap(), Role::SysOp);
        assert_eq!(Role::from_str("SYSOP").unwrap(), Role::SysOp);
        assert!(Role::from_str("invalid").is_err());
    }

    #[test]
    fn test_role_as_str() {
        assert_eq!(Role::Guest.as_str(), "guest");
        assert_eq!(Role::Member.as_str(), "member");
        assert_eq!(Role::SubOp.as_str(), "subop");
        assert_eq!(Role::SysOp.as_str(), "sysop");
    }

    #[test]
    fn test_role_display() {
        assert_eq!(format!("{}", Role::SysOp), "sysop");
    }

    #[test]
    fn test_role_default() {
        assert_eq!(Role::default(), Role::Member);
    }

    #[test]
    fn test_new_user_builder() {
        let user = NewUser::new("testuser", "hash", "Test User")
            .with_email("test@example.com")
            .with_role(Role::SubOp)
            .with_terminal("c64");

        assert_eq!(user.username, "testuser");
        assert_eq!(user.password, "hash");
        assert_eq!(user.nickname, "Test User");
        assert_eq!(user.email, Some("test@example.com".to_string()));
        assert_eq!(user.role, Role::SubOp);
        assert_eq!(user.terminal, "c64");
    }

    #[test]
    fn test_user_update_builder() {
        let update = UserUpdate::new()
            .nickname("New Name")
            .role(Role::SubOp)
            .is_active(false);

        assert!(update.nickname.is_some());
        assert!(update.role.is_some());
        assert!(update.is_active.is_some());
        assert!(update.password.is_none());
        assert!(!update.is_empty());
    }

    #[test]
    fn test_user_update_empty() {
        let update = UserUpdate::new();
        assert!(update.is_empty());
    }

    #[test]
    fn test_user_has_role() {
        let user = User {
            id: 1,
            username: "test".to_string(),
            password: "hash".to_string(),
            nickname: "Test".to_string(),
            email: None,
            role: Role::SubOp,
            profile: None,
            terminal: "standard".to_string(),
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        };

        assert!(user.has_role(Role::Guest));
        assert!(user.has_role(Role::Member));
        assert!(user.has_role(Role::SubOp));
        assert!(!user.has_role(Role::SysOp));
    }

    #[test]
    fn test_user_is_operator() {
        let member = User {
            id: 1,
            username: "member".to_string(),
            password: "hash".to_string(),
            nickname: "Member".to_string(),
            email: None,
            role: Role::Member,
            profile: None,
            terminal: "standard".to_string(),
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        };

        let subop = User {
            role: Role::SubOp,
            ..member.clone()
        };

        let sysop = User {
            role: Role::SysOp,
            ..member.clone()
        };

        assert!(!member.is_operator());
        assert!(subop.is_operator());
        assert!(sysop.is_operator());
        assert!(!subop.is_sysop());
        assert!(sysop.is_sysop());
    }
}
