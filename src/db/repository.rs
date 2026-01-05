//! User repository for HOBBS.
//!
//! This module provides CRUD operations for users in the database.

use rusqlite::{params, Row};

use super::user::{NewUser, Role, User, UserUpdate};
use super::Database;
use crate::server::CharacterEncoding;
use crate::{HobbsError, Result};

/// Repository for user CRUD operations.
pub struct UserRepository<'a> {
    db: &'a Database,
}

impl<'a> UserRepository<'a> {
    /// Create a new UserRepository with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new user in the database.
    ///
    /// Returns the created user with the assigned ID.
    pub fn create(&self, new_user: &NewUser) -> Result<User> {
        self.db.conn().execute(
            "INSERT INTO users (username, password, nickname, email, role, terminal, encoding, language, auto_paging)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                &new_user.username,
                &new_user.password,
                &new_user.nickname,
                &new_user.email,
                new_user.role.as_str(),
                &new_user.terminal,
                new_user.encoding.as_str(),
                &new_user.language,
                if new_user.auto_paging { 1i64 } else { 0i64 },
            ],
        )?;

        let id = self.db.conn().last_insert_rowid();
        self.get_by_id(id)?
            .ok_or_else(|| HobbsError::NotFound("user".to_string()))
    }

    /// Get a user by ID.
    pub fn get_by_id(&self, id: i64) -> Result<Option<User>> {
        let result = self.db.conn().query_row(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users WHERE id = ?",
            [id],
            Self::row_to_user,
        );

        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a user by username.
    pub fn get_by_username(&self, username: &str) -> Result<Option<User>> {
        let result = self.db.conn().query_row(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users WHERE username = ?",
            [username],
            Self::row_to_user,
        );

        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update a user by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated user, or None if not found.
    pub fn update(&self, id: i64, update: &UserUpdate) -> Result<Option<User>> {
        if update.is_empty() {
            return self.get_by_id(id);
        }

        let mut fields = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref password) = update.password {
            fields.push("password = ?");
            values.push(Box::new(password.clone()));
        }
        if let Some(ref nickname) = update.nickname {
            fields.push("nickname = ?");
            values.push(Box::new(nickname.clone()));
        }
        if let Some(ref email) = update.email {
            fields.push("email = ?");
            values.push(Box::new(email.clone()));
        }
        if let Some(role) = update.role {
            fields.push("role = ?");
            values.push(Box::new(role.as_str().to_string()));
        }
        if let Some(ref profile) = update.profile {
            fields.push("profile = ?");
            values.push(Box::new(profile.clone()));
        }
        if let Some(ref terminal) = update.terminal {
            fields.push("terminal = ?");
            values.push(Box::new(terminal.clone()));
        }
        if let Some(encoding) = update.encoding {
            fields.push("encoding = ?");
            values.push(Box::new(encoding.as_str().to_string()));
        }
        if let Some(ref language) = update.language {
            fields.push("language = ?");
            values.push(Box::new(language.clone()));
        }
        if let Some(is_active) = update.is_active {
            fields.push("is_active = ?");
            values.push(Box::new(if is_active { 1i64 } else { 0i64 }));
        }
        if let Some(auto_paging) = update.auto_paging {
            fields.push("auto_paging = ?");
            values.push(Box::new(if auto_paging { 1i64 } else { 0i64 }));
        }

        let sql = format!("UPDATE users SET {} WHERE id = ?", fields.join(", "));
        values.push(Box::new(id));

        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
        let affected = self.db.conn().execute(&sql, params.as_slice())?;

        if affected == 0 {
            return Ok(None);
        }

        self.get_by_id(id)
    }

    /// Update the last login timestamp for a user.
    pub fn update_last_login(&self, id: i64) -> Result<()> {
        self.db.conn().execute(
            "UPDATE users SET last_login = datetime('now') WHERE id = ?",
            [id],
        )?;
        Ok(())
    }

    /// Delete a user by ID.
    ///
    /// Returns true if a user was deleted, false if not found.
    pub fn delete(&self, id: i64) -> Result<bool> {
        let affected = self
            .db
            .conn()
            .execute("DELETE FROM users WHERE id = ?", [id])?;
        Ok(affected > 0)
    }

    /// List all active users.
    pub fn list_active(&self) -> Result<Vec<User>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users WHERE is_active = 1 ORDER BY username",
        )?;

        let users = stmt
            .query_map([], Self::row_to_user)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(users)
    }

    /// List all users (including inactive).
    pub fn list_all(&self) -> Result<Vec<User>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users ORDER BY username",
        )?;

        let users = stmt
            .query_map([], Self::row_to_user)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(users)
    }

    /// List users by role.
    pub fn list_by_role(&self, role: Role) -> Result<Vec<User>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, username, password, nickname, email, role, profile, terminal,
                    encoding, language, auto_paging, created_at, last_login, is_active
             FROM users WHERE role = ? AND is_active = 1 ORDER BY username",
        )?;

        let users = stmt
            .query_map([role.as_str()], Self::row_to_user)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(users)
    }

    /// Count all users.
    pub fn count(&self) -> Result<i64> {
        let count: i64 = self
            .db
            .conn()
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Count active users.
    pub fn count_active(&self) -> Result<i64> {
        let count: i64 = self.db.conn().query_row(
            "SELECT COUNT(*) FROM users WHERE is_active = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Check if a username is already taken.
    pub fn username_exists(&self, username: &str) -> Result<bool> {
        let exists: bool = self.db.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = ?)",
            [username],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Convert a database row to a User struct.
    fn row_to_user(row: &Row<'_>) -> rusqlite::Result<User> {
        let role_str: String = row.get(5)?;
        let role = role_str.parse().unwrap_or(Role::Member);
        let encoding_str: String = row.get(8)?;
        let encoding = encoding_str.parse().unwrap_or(CharacterEncoding::default());
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    #[test]
    fn test_create_user() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).unwrap();

        assert_eq!(user.id, 1);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.nickname, "Test User");
        assert_eq!(user.role, Role::Member);
        assert!(user.is_active);
    }

    #[test]
    fn test_create_user_with_options() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("admin", "hashedpw", "Administrator")
            .with_email("admin@example.com")
            .with_role(Role::SysOp)
            .with_terminal("c64");

        let user = repo.create(&new_user).unwrap();

        assert_eq!(user.username, "admin");
        assert_eq!(user.email, Some("admin@example.com".to_string()));
        assert_eq!(user.role, Role::SysOp);
        assert_eq!(user.terminal, "c64");
    }

    #[test]
    fn test_create_duplicate_username() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        repo.create(&new_user).unwrap();

        let duplicate = NewUser::new("testuser", "otherpw", "Other User");
        let result = repo.create(&duplicate);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_by_id() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let created = repo.create(&new_user).unwrap();

        let found = repo.get_by_id(created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "testuser");

        let not_found = repo.get_by_id(999).unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_by_username() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        repo.create(&new_user).unwrap();

        let found = repo.get_by_username("testuser").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().nickname, "Test User");

        let not_found = repo.get_by_username("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_update_user() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).unwrap();

        let update = UserUpdate::new()
            .nickname("Updated Name")
            .email(Some("new@example.com".to_string()))
            .role(Role::SubOp);

        let updated = repo.update(user.id, &update).unwrap().unwrap();

        assert_eq!(updated.nickname, "Updated Name");
        assert_eq!(updated.email, Some("new@example.com".to_string()));
        assert_eq!(updated.role, Role::SubOp);
        // Unchanged fields
        assert_eq!(updated.username, "testuser");
        assert_eq!(updated.password, "hashedpw");
    }

    #[test]
    fn test_update_nonexistent_user() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let update = UserUpdate::new().nickname("New Name");
        let result = repo.update(999, &update).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_update_empty() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).unwrap();

        let update = UserUpdate::new();
        let result = repo.update(user.id, &update).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().nickname, "Test User");
    }

    #[test]
    fn test_update_is_active() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).unwrap();
        assert!(user.is_active);

        let update = UserUpdate::new().is_active(false);
        let updated = repo.update(user.id, &update).unwrap().unwrap();

        assert!(!updated.is_active);
    }

    #[test]
    fn test_update_last_login() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).unwrap();
        assert!(user.last_login.is_none());

        repo.update_last_login(user.id).unwrap();

        let updated = repo.get_by_id(user.id).unwrap().unwrap();
        assert!(updated.last_login.is_some());
    }

    #[test]
    fn test_delete_user() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hashedpw", "Test User");
        let user = repo.create(&new_user).unwrap();

        let deleted = repo.delete(user.id).unwrap();
        assert!(deleted);

        let found = repo.get_by_id(user.id).unwrap();
        assert!(found.is_none());

        // Deleting again should return false
        let deleted_again = repo.delete(user.id).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_list_active() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        // Create some users
        repo.create(&NewUser::new("user1", "pw", "User 1")).unwrap();
        let user2 = repo.create(&NewUser::new("user2", "pw", "User 2")).unwrap();
        repo.create(&NewUser::new("user3", "pw", "User 3")).unwrap();

        // Deactivate user2
        repo.update(user2.id, &UserUpdate::new().is_active(false))
            .unwrap();

        let active = repo.list_active().unwrap();
        assert_eq!(active.len(), 2);
        assert!(active.iter().all(|u| u.username != "user2"));
    }

    #[test]
    fn test_list_all() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        repo.create(&NewUser::new("user1", "pw", "User 1")).unwrap();
        let user2 = repo.create(&NewUser::new("user2", "pw", "User 2")).unwrap();
        repo.create(&NewUser::new("user3", "pw", "User 3")).unwrap();

        // Deactivate user2
        repo.update(user2.id, &UserUpdate::new().is_active(false))
            .unwrap();

        let all = repo.list_all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_list_by_role() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        repo.create(&NewUser::new("member1", "pw", "Member 1"))
            .unwrap();
        repo.create(&NewUser::new("member2", "pw", "Member 2"))
            .unwrap();
        repo.create(&NewUser::new("subop", "pw", "SubOp").with_role(Role::SubOp))
            .unwrap();
        repo.create(&NewUser::new("sysop", "pw", "SysOp").with_role(Role::SysOp))
            .unwrap();

        let members = repo.list_by_role(Role::Member).unwrap();
        assert_eq!(members.len(), 2);

        let subops = repo.list_by_role(Role::SubOp).unwrap();
        assert_eq!(subops.len(), 1);

        let sysops = repo.list_by_role(Role::SysOp).unwrap();
        assert_eq!(sysops.len(), 1);
    }

    #[test]
    fn test_count() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        assert_eq!(repo.count().unwrap(), 0);
        assert_eq!(repo.count_active().unwrap(), 0);

        repo.create(&NewUser::new("user1", "pw", "User 1")).unwrap();
        let user2 = repo.create(&NewUser::new("user2", "pw", "User 2")).unwrap();

        assert_eq!(repo.count().unwrap(), 2);
        assert_eq!(repo.count_active().unwrap(), 2);

        repo.update(user2.id, &UserUpdate::new().is_active(false))
            .unwrap();

        assert_eq!(repo.count().unwrap(), 2);
        assert_eq!(repo.count_active().unwrap(), 1);
    }

    #[test]
    fn test_username_exists() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        assert!(!repo.username_exists("testuser").unwrap());

        repo.create(&NewUser::new("testuser", "pw", "Test"))
            .unwrap();

        assert!(repo.username_exists("testuser").unwrap());
        assert!(!repo.username_exists("other").unwrap());
    }

    #[test]
    fn test_create_user_with_encoding() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user =
            NewUser::new("testuser", "hash", "Test").with_encoding(CharacterEncoding::Utf8);
        let user = repo.create(&new_user).unwrap();

        assert_eq!(user.encoding, CharacterEncoding::Utf8);
    }

    #[test]
    fn test_create_user_default_encoding() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hash", "Test");
        let user = repo.create(&new_user).unwrap();

        assert_eq!(user.encoding, CharacterEncoding::ShiftJIS);
    }

    #[test]
    fn test_update_encoding() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hash", "Test");
        let user = repo.create(&new_user).unwrap();
        assert_eq!(user.encoding, CharacterEncoding::ShiftJIS);

        let update = UserUpdate::new().encoding(CharacterEncoding::Utf8);
        let updated = repo.update(user.id, &update).unwrap().unwrap();

        assert_eq!(updated.encoding, CharacterEncoding::Utf8);
    }

    #[test]
    fn test_create_user_with_language() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hash", "Test").with_language("ja");
        let user = repo.create(&new_user).unwrap();

        assert_eq!(user.language, "ja");
    }

    #[test]
    fn test_create_user_default_language() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hash", "Test");
        let user = repo.create(&new_user).unwrap();

        assert_eq!(user.language, "en");
    }

    #[test]
    fn test_update_language() {
        let db = setup_db();
        let repo = UserRepository::new(&db);

        let new_user = NewUser::new("testuser", "hash", "Test");
        let user = repo.create(&new_user).unwrap();
        assert_eq!(user.language, "en");

        let update = UserUpdate::new().language("ja");
        let updated = repo.update(user.id, &update).unwrap().unwrap();

        assert_eq!(updated.language, "ja");
    }
}
