//! User management for administrators.
//!
//! This module provides administrative functions for managing users:
//! - List users (SubOp and above)
//! - Get user detail (SubOp and above)
//! - Update nickname (SubOp can edit Member, SysOp can edit anyone)
//! - Reset password (SubOp can reset Member, SysOp can reset anyone)
//! - Change role (SysOp only)
//! - Suspend/activate account (SubOp can suspend Member, SysOp can suspend anyone)

use rand::Rng;

use crate::auth::hash_password;
use crate::board::PaginatedResult;
use crate::db::{Database, Role, User, UserRepository, UserUpdate};

use super::{can_edit_user, require_admin, AdminError, AdminService};

/// Default length for generated passwords.
pub const DEFAULT_PASSWORD_LENGTH: usize = 12;

/// Characters used for password generation.
const PASSWORD_CHARS: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// User detail information for admin view.
#[derive(Debug, Clone)]
pub struct UserDetail {
    /// User information.
    pub user: User,
    /// Number of posts by this user.
    pub post_count: i64,
    /// Number of files uploaded by this user.
    pub file_count: i64,
    /// Number of mail messages sent by this user.
    pub mail_sent_count: i64,
    /// Number of mail messages received by this user.
    pub mail_received_count: i64,
}

impl UserDetail {
    /// Create a new UserDetail.
    pub fn new(user: User) -> Self {
        Self {
            user,
            post_count: 0,
            file_count: 0,
            mail_sent_count: 0,
            mail_received_count: 0,
        }
    }

    /// Set post count.
    pub fn with_post_count(mut self, count: i64) -> Self {
        self.post_count = count;
        self
    }

    /// Set file count.
    pub fn with_file_count(mut self, count: i64) -> Self {
        self.file_count = count;
        self
    }

    /// Set mail sent count.
    pub fn with_mail_sent_count(mut self, count: i64) -> Self {
        self.mail_sent_count = count;
        self
    }

    /// Set mail received count.
    pub fn with_mail_received_count(mut self, count: i64) -> Self {
        self.mail_received_count = count;
        self
    }
}

/// Generate a random password.
///
/// The password contains alphanumeric characters (excluding ambiguous ones like 0, O, 1, l, I).
pub fn generate_password(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..PASSWORD_CHARS.len());
            PASSWORD_CHARS[idx] as char
        })
        .collect()
}

/// Admin service for user management.
pub struct UserAdminService<'a> {
    db: &'a Database,
}

impl<'a> UserAdminService<'a> {
    /// Create a new UserAdminService.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// List users with pagination.
    ///
    /// Requires SubOp or higher permission.
    /// Returns all users including inactive ones for admin purposes.
    pub fn list_users(
        &self,
        offset: i64,
        limit: i64,
        admin: &User,
    ) -> Result<PaginatedResult<User>, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();

        // Get total count
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;

        // Get users with pagination
        let mut stmt = conn.prepare(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )?;

        let users = stmt
            .query_map([limit, offset], |row| {
                let role_str: String = row.get(5)?;
                let role = role_str.parse().unwrap_or(Role::Member);
                let encoding_str: String = row.get(8)?;
                let encoding = encoding_str
                    .parse()
                    .unwrap_or(crate::server::CharacterEncoding::default());
                let auto_paging: i64 = row.get(10)?;
                let is_active: i64 = row.get(13)?;

                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password: row.get(2)?,
                    nickname: row.get(3)?,
                    email: row.get(4)?,
                    role,
                    profile: row.get(6)?,
                    terminal: row.get(7)?,
                    encoding,
                    language: row.get(9)?,
                    auto_paging: auto_paging != 0,
                    created_at: row.get(11)?,
                    last_login: row.get(12)?,
                    is_active: is_active != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PaginatedResult {
            items: users,
            total,
            offset,
            limit,
        })
    }

    /// Get user detail by ID.
    ///
    /// Requires SubOp or higher permission.
    /// Returns user information with activity statistics.
    pub fn get_user_detail(&self, user_id: i64, admin: &User) -> Result<UserDetail, AdminError> {
        require_admin(Some(admin))?;

        let repo = UserRepository::new(self.db);
        let user = repo
            .get_by_id(user_id)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        let conn = self.db.conn();

        // Count posts
        let post_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM posts WHERE author_id = ?",
                [user_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Count files
        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE uploader_id = ?",
                [user_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Count sent mail
        let mail_sent_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail WHERE sender_id = ?",
                [user_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Count received mail
        let mail_received_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail WHERE recipient_id = ?",
                [user_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(UserDetail::new(user)
            .with_post_count(post_count)
            .with_file_count(file_count)
            .with_mail_sent_count(mail_sent_count)
            .with_mail_received_count(mail_received_count))
    }

    /// Update a user's nickname.
    ///
    /// Requires SubOp or higher permission.
    /// SubOp can only edit Member or lower.
    /// SysOp can edit anyone.
    pub fn update_user_nickname(
        &self,
        user_id: i64,
        nickname: &str,
        admin: &User,
    ) -> Result<User, AdminError> {
        let repo = UserRepository::new(self.db);
        let target = repo
            .get_by_id(user_id)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        can_edit_user(admin, &target)?;

        // Validate nickname (basic validation)
        let nickname = nickname.trim();
        if nickname.is_empty() {
            return Err(AdminError::InvalidOperation(
                "ニックネームは空にできません".to_string(),
            ));
        }
        if nickname.len() > 20 {
            return Err(AdminError::InvalidOperation(
                "ニックネームは20文字以内で入力してください".to_string(),
            ));
        }

        let update = UserUpdate::new().nickname(nickname);
        let updated = repo
            .update(user_id, &update)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        Ok(updated)
    }

    /// Reset a user's password.
    ///
    /// Generates a new random password and returns it.
    /// The password is hashed before storing.
    ///
    /// Requires SubOp or higher permission.
    /// SubOp can only reset Member or lower.
    /// SysOp can reset anyone.
    pub fn reset_user_password(&self, user_id: i64, admin: &User) -> Result<String, AdminError> {
        let repo = UserRepository::new(self.db);
        let target = repo
            .get_by_id(user_id)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        can_edit_user(admin, &target)?;

        // Generate new password
        let new_password = generate_password(DEFAULT_PASSWORD_LENGTH);

        // Hash the password
        let hashed = hash_password(&new_password).map_err(|e| {
            AdminError::InvalidOperation(format!("パスワードのハッシュ化に失敗: {e}"))
        })?;

        // Update password
        let update = UserUpdate::new().password(&hashed);
        repo.update(user_id, &update)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        Ok(new_password)
    }

    /// Change a user's role.
    ///
    /// Requires SysOp permission.
    /// Cannot change own role.
    /// Cannot demote the last SysOp.
    pub fn change_user_role(
        &self,
        user_id: i64,
        new_role: Role,
        admin: &User,
    ) -> Result<User, AdminError> {
        let repo = UserRepository::new(self.db);
        let target = repo
            .get_by_id(user_id)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        // Use AdminService for full validation
        let admin_service = AdminService::new(self.db);
        admin_service.validate_role_change(admin, &target, new_role)?;

        let update = UserUpdate::new().role(new_role);
        let updated = repo
            .update(user_id, &update)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        Ok(updated)
    }

    /// Suspend a user account.
    ///
    /// Requires SubOp or higher permission.
    /// SubOp can only suspend Member or lower.
    /// SysOp can suspend anyone (except themselves).
    /// Cannot suspend the last SysOp.
    pub fn suspend_user(&self, user_id: i64, admin: &User) -> Result<User, AdminError> {
        let repo = UserRepository::new(self.db);
        let target = repo
            .get_by_id(user_id)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        can_edit_user(admin, &target)?;

        // Cannot suspend self
        if admin.id == target.id {
            return Err(AdminError::CannotModifySelf);
        }

        // Cannot suspend the last SysOp
        if target.role == Role::SysOp {
            let admin_service = AdminService::new(self.db);
            if !admin_service.has_multiple_sysops()? {
                return Err(AdminError::LastSysOp);
            }
        }

        let update = UserUpdate::new().is_active(false);
        let updated = repo
            .update(user_id, &update)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        Ok(updated)
    }

    /// Activate a suspended user account.
    ///
    /// Requires SubOp or higher permission.
    /// SubOp can only activate Member or lower.
    /// SysOp can activate anyone.
    pub fn activate_user(&self, user_id: i64, admin: &User) -> Result<User, AdminError> {
        let repo = UserRepository::new(self.db);
        let target = repo
            .get_by_id(user_id)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        can_edit_user(admin, &target)?;

        let update = UserUpdate::new().is_active(true);
        let updated = repo
            .update(user_id, &update)?
            .ok_or_else(|| AdminError::NotFound("ユーザー".to_string()))?;

        Ok(updated)
    }

    /// Search users by username or nickname.
    ///
    /// Requires SubOp or higher permission.
    pub fn search_users(
        &self,
        query: &str,
        offset: i64,
        limit: i64,
        admin: &User,
    ) -> Result<PaginatedResult<User>, AdminError> {
        require_admin(Some(admin))?;

        let conn = self.db.conn();
        let search_pattern = format!("%{query}%");

        // Get total count
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE username LIKE ? OR nickname LIKE ?",
            [&search_pattern, &search_pattern],
            |row| row.get(0),
        )?;

        // Get users with pagination
        let mut stmt = conn.prepare(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users
             WHERE username LIKE ? OR nickname LIKE ?
             ORDER BY username
             LIMIT ? OFFSET ?",
        )?;

        let users = stmt
            .query_map(
                rusqlite::params![&search_pattern, &search_pattern, limit, offset],
                |row| {
                    let role_str: String = row.get(5)?;
                    let role = role_str.parse().unwrap_or(Role::Member);
                    let encoding_str: String = row.get(8)?;
                    let encoding = encoding_str
                        .parse()
                        .unwrap_or(crate::server::CharacterEncoding::default());
                    let auto_paging: i64 = row.get(10)?;
                    let is_active: i64 = row.get(13)?;

                    Ok(User {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        password: row.get(2)?,
                        nickname: row.get(3)?,
                        email: row.get(4)?,
                        role,
                        profile: row.get(6)?,
                        terminal: row.get(7)?,
                        encoding,
                        language: row.get(9)?,
                        auto_paging: auto_paging != 0,
                        created_at: row.get(11)?,
                        last_login: row.get(12)?,
                        is_active: is_active != 0,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PaginatedResult {
            items: users,
            total,
            offset,
            limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewUser;
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

    fn create_db_user(db: &Database, username: &str, nickname: &str, role: Role) -> User {
        let repo = UserRepository::new(db);
        repo.create(&NewUser::new(username, "hash", nickname).with_role(role))
            .unwrap()
    }

    // generate_password tests
    #[test]
    fn test_generate_password_length() {
        let password = generate_password(12);
        assert_eq!(password.len(), 12);

        let password = generate_password(16);
        assert_eq!(password.len(), 16);
    }

    #[test]
    fn test_generate_password_uniqueness() {
        let p1 = generate_password(12);
        let p2 = generate_password(12);
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_generate_password_valid_chars() {
        let password = generate_password(100);
        let valid_chars: Vec<char> = PASSWORD_CHARS.iter().map(|&b| b as char).collect();
        assert!(password.chars().all(|c| valid_chars.contains(&c)));
    }

    // list_users tests
    #[test]
    fn test_list_users_as_subop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);

        create_db_user(&db, "user1", "User 1", Role::Member);
        create_db_user(&db, "user2", "User 2", Role::Member);
        create_db_user(&db, "user3", "User 3", Role::Member);

        let result = service.list_users(0, 10, &subop).unwrap();
        assert_eq!(result.total, 4); // 3 members + 1 subop
        assert_eq!(result.items.len(), 4);
    }

    #[test]
    fn test_list_users_pagination() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);

        for i in 0..10 {
            create_db_user(&db, &format!("user{i}"), &format!("User {i}"), Role::Member);
        }

        let page1 = service.list_users(0, 3, &subop).unwrap();
        assert_eq!(page1.total, 11); // 10 members + 1 subop
        assert_eq!(page1.items.len(), 3);

        let page2 = service.list_users(3, 3, &subop).unwrap();
        assert_eq!(page2.total, 11);
        assert_eq!(page2.items.len(), 3);
    }

    #[test]
    fn test_list_users_as_member_fails() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let member = create_test_user(1, Role::Member);

        let result = service.list_users(0, 10, &member);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    // get_user_detail tests
    #[test]
    fn test_get_user_detail() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        let detail = service.get_user_detail(member.id, &subop).unwrap();
        assert_eq!(detail.user.username, "member");
        assert_eq!(detail.post_count, 0);
        assert_eq!(detail.file_count, 0);
    }

    #[test]
    fn test_get_user_detail_not_found() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);

        let result = service.get_user_detail(999, &subop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    // update_user_nickname tests
    #[test]
    fn test_update_nickname_as_subop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        let updated = service
            .update_user_nickname(member.id, "新しい名前", &subop)
            .unwrap();
        assert_eq!(updated.nickname, "新しい名前");
    }

    #[test]
    fn test_update_nickname_subop_cannot_edit_subop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop1 = create_db_user(&db, "subop1", "SubOp 1", Role::SubOp);
        let subop2 = create_db_user(&db, "subop2", "SubOp 2", Role::SubOp);

        let result = service.update_user_nickname(subop2.id, "新しい名前", &subop1);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_update_nickname_sysop_can_edit_subop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let sysop = create_db_user(&db, "sysop", "SysOp", Role::SysOp);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);

        let updated = service
            .update_user_nickname(subop.id, "新しい名前", &sysop)
            .unwrap();
        assert_eq!(updated.nickname, "新しい名前");
    }

    #[test]
    fn test_update_nickname_empty_fails() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let sysop = create_db_user(&db, "sysop", "SysOp", Role::SysOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        let result = service.update_user_nickname(member.id, "", &sysop);
        assert!(matches!(result, Err(AdminError::InvalidOperation(_))));
    }

    #[test]
    fn test_update_nickname_too_long_fails() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let sysop = create_db_user(&db, "sysop", "SysOp", Role::SysOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        let long_name = "a".repeat(21);
        let result = service.update_user_nickname(member.id, &long_name, &sysop);
        assert!(matches!(result, Err(AdminError::InvalidOperation(_))));
    }

    // reset_user_password tests
    #[test]
    fn test_reset_password_as_subop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        let new_password = service.reset_user_password(member.id, &subop).unwrap();
        assert_eq!(new_password.len(), DEFAULT_PASSWORD_LENGTH);

        // Verify password was changed
        let repo = UserRepository::new(&db);
        let updated = repo.get_by_id(member.id).unwrap().unwrap();
        assert_ne!(updated.password, "hash");
    }

    #[test]
    fn test_reset_password_subop_cannot_reset_subop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop1 = create_db_user(&db, "subop1", "SubOp 1", Role::SubOp);
        let subop2 = create_db_user(&db, "subop2", "SubOp 2", Role::SubOp);

        let result = service.reset_user_password(subop2.id, &subop1);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    // change_user_role tests
    #[test]
    fn test_change_role_as_sysop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let sysop = create_db_user(&db, "sysop", "SysOp", Role::SysOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        let updated = service
            .change_user_role(member.id, Role::SubOp, &sysop)
            .unwrap();
        assert_eq!(updated.role, Role::SubOp);
    }

    #[test]
    fn test_change_role_as_subop_fails() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        let result = service.change_user_role(member.id, Role::SubOp, &subop);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_change_role_self_fails() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let sysop = create_db_user(&db, "sysop", "SysOp", Role::SysOp);

        let result = service.change_user_role(sysop.id, Role::Member, &sysop);
        assert!(matches!(result, Err(AdminError::CannotModifySelf)));
    }

    #[test]
    fn test_change_role_demote_last_sysop_fails() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let sysop1 = create_db_user(&db, "sysop1", "SysOp 1", Role::SysOp);
        let sysop2 = create_db_user(&db, "sysop2", "SysOp 2", Role::SysOp);

        // Deactivate sysop2 to make sysop1 the "last" active sysop
        let repo = UserRepository::new(&db);
        repo.update(sysop2.id, &UserUpdate::new().is_active(false))
            .unwrap();

        // Create a new active sysop for testing (sysop1 can't demote self)
        let sysop3 = create_db_user(&db, "sysop3", "SysOp 3", Role::SysOp);

        // Now sysop1 and sysop3 are active, try to demote sysop3
        // First deactivate sysop1 to make sysop3 the last
        repo.update(sysop1.id, &UserUpdate::new().is_active(false))
            .unwrap();

        // Create another admin to do the demotion
        let admin = User {
            id: 999,
            username: "admin".to_string(),
            password: "hash".to_string(),
            nickname: "Admin".to_string(),
            email: None,
            role: Role::SysOp,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            language: "en".to_string(),
            auto_paging: true,
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        };

        let result = service.change_user_role(sysop3.id, Role::Member, &admin);
        assert!(matches!(result, Err(AdminError::LastSysOp)));
    }

    // suspend_user tests
    #[test]
    fn test_suspend_user_as_subop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        let suspended = service.suspend_user(member.id, &subop).unwrap();
        assert!(!suspended.is_active);
    }

    #[test]
    fn test_suspend_user_subop_cannot_suspend_subop() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop1 = create_db_user(&db, "subop1", "SubOp 1", Role::SubOp);
        let subop2 = create_db_user(&db, "subop2", "SubOp 2", Role::SubOp);

        let result = service.suspend_user(subop2.id, &subop1);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_suspend_user_self_fails() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let sysop = create_db_user(&db, "sysop", "SysOp", Role::SysOp);

        let result = service.suspend_user(sysop.id, &sysop);
        assert!(matches!(result, Err(AdminError::CannotModifySelf)));
    }

    #[test]
    fn test_suspend_last_sysop_fails() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let sysop1 = create_db_user(&db, "sysop1", "SysOp 1", Role::SysOp);
        let sysop2 = create_db_user(&db, "sysop2", "SysOp 2", Role::SysOp);

        // Deactivate sysop2 to make sysop1 the last active sysop
        let repo = UserRepository::new(&db);
        repo.update(sysop2.id, &UserUpdate::new().is_active(false))
            .unwrap();

        // Create another admin who is not the last sysop
        let admin = User {
            id: 999,
            username: "admin".to_string(),
            password: "hash".to_string(),
            nickname: "Admin".to_string(),
            email: None,
            role: Role::SysOp,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            language: "en".to_string(),
            auto_paging: true,
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        };

        let result = service.suspend_user(sysop1.id, &admin);
        assert!(matches!(result, Err(AdminError::LastSysOp)));
    }

    // activate_user tests
    #[test]
    fn test_activate_user() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);
        let member = create_db_user(&db, "member", "Member", Role::Member);

        // First suspend the user
        service.suspend_user(member.id, &subop).unwrap();

        // Then activate
        let activated = service.activate_user(member.id, &subop).unwrap();
        assert!(activated.is_active);
    }

    // search_users tests
    #[test]
    fn test_search_users() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);

        create_db_user(&db, "john_doe", "John Doe", Role::Member);
        create_db_user(&db, "jane_doe", "Jane Doe", Role::Member);
        create_db_user(&db, "bob_smith", "Bob Smith", Role::Member);

        let result = service.search_users("doe", 0, 10, &subop).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
    }

    #[test]
    fn test_search_users_by_nickname() {
        let db = setup_db();
        let service = UserAdminService::new(&db);
        let subop = create_db_user(&db, "subop", "SubOp", Role::SubOp);

        create_db_user(&db, "user1", "田中太郎", Role::Member);
        create_db_user(&db, "user2", "田中花子", Role::Member);
        create_db_user(&db, "user3", "山田太郎", Role::Member);

        let result = service.search_users("田中", 0, 10, &subop).unwrap();
        assert_eq!(result.total, 2);
    }

    // UserDetail tests
    #[test]
    fn test_user_detail_builder() {
        let user = create_test_user(1, Role::Member);
        let detail = UserDetail::new(user)
            .with_post_count(10)
            .with_file_count(5)
            .with_mail_sent_count(3)
            .with_mail_received_count(7);

        assert_eq!(detail.post_count, 10);
        assert_eq!(detail.file_count, 5);
        assert_eq!(detail.mail_sent_count, 3);
        assert_eq!(detail.mail_received_count, 7);
    }
}
