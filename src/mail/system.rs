//! System mail service for HOBBS.
//!
//! This module provides system-generated mail functionality including:
//! - Welcome mail for new users
//! - (Future) Password reset notifications
//! - (Future) Admin announcements
//! - (Future) Account suspension notices

use crate::db::{Database, Role, User, UserRepository};
use crate::{HobbsError, Result};

use super::repository::MailRepository;
use super::types::NewMail;

/// Default welcome mail subject.
pub const WELCOME_MAIL_SUBJECT: &str = "ようこそ HOBBSへ！";

/// Default welcome mail body template.
pub const WELCOME_MAIL_BODY: &str = r#"
{nickname}さん、HOBBSへようこそ！

このメールはシステムから自動送信されています。

HOBBSの主な機能をご紹介します：

【掲示板】
  スレッド形式やフラット形式の掲示板で、
  他のユーザーと情報交換ができます。

【チャット】
  リアルタイムでメンバーとチャットができます。
  /help コマンドで使い方を確認してください。

【メール】
  他のユーザーにプライベートメッセージを送れます。

ご不明な点があれば、SysOpまでお問い合わせください。

楽しいBBSライフを！

--
HOBBS System
"#;

/// Service for system-generated mail.
pub struct SystemMailService<'a> {
    db: &'a Database,
}

impl<'a> SystemMailService<'a> {
    /// Create a new SystemMailService with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get the first active SysOp user.
    ///
    /// Returns the first user with SysOp role, or None if no SysOp exists.
    pub fn get_sysop_user(&self) -> Result<Option<User>> {
        let repo = UserRepository::new(self.db);
        let sysops = repo.list_by_role(Role::SysOp)?;
        Ok(sysops.into_iter().next())
    }

    /// Send a welcome mail to a new user.
    ///
    /// The mail is sent from the first SysOp user. If no SysOp exists,
    /// the welcome mail is silently skipped (no error).
    ///
    /// # Arguments
    ///
    /// * `recipient` - The new user to send the welcome mail to
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the mail was sent, `Ok(false)` if skipped
    /// (no SysOp available or recipient is a SysOp).
    pub fn send_welcome_mail(&self, recipient: &User) -> Result<bool> {
        // Don't send welcome mail to SysOp (they're the sender)
        if recipient.role == Role::SysOp {
            return Ok(false);
        }

        // Get the SysOp to use as sender
        let sysop = match self.get_sysop_user()? {
            Some(u) => u,
            None => return Ok(false), // No SysOp, skip welcome mail
        };

        // Generate the mail body with the recipient's nickname
        let body = self.generate_welcome_body(&recipient.nickname);

        // Create and send the mail
        let new_mail = NewMail::new(sysop.id, recipient.id, WELCOME_MAIL_SUBJECT, body);
        MailRepository::create(self.db.conn(), &new_mail)?;

        Ok(true)
    }

    /// Send a welcome mail to a user by ID.
    ///
    /// Convenience method that fetches the user and sends the welcome mail.
    pub fn send_welcome_mail_by_id(&self, user_id: i64) -> Result<bool> {
        let repo = UserRepository::new(self.db);
        let user = repo
            .get_by_id(user_id)?
            .ok_or_else(|| HobbsError::NotFound("user".to_string()))?;

        self.send_welcome_mail(&user)
    }

    /// Generate the welcome mail body with the user's nickname.
    fn generate_welcome_body(&self, nickname: &str) -> String {
        WELCOME_MAIL_BODY
            .replace("{nickname}", nickname)
            .trim()
            .to_string()
    }

    /// Send a system notification mail.
    ///
    /// Generic method for sending system messages from the SysOp.
    ///
    /// # Arguments
    ///
    /// * `recipient_id` - The user ID to send the notification to
    /// * `subject` - The mail subject
    /// * `body` - The mail body
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the mail was sent, `Ok(false)` if no SysOp available.
    pub fn send_notification(&self, recipient_id: i64, subject: &str, body: &str) -> Result<bool> {
        // Get the SysOp to use as sender
        let sysop = match self.get_sysop_user()? {
            Some(u) => u,
            None => return Ok(false),
        };

        // Don't send to self
        if sysop.id == recipient_id {
            return Ok(false);
        }

        // Create and send the mail
        let new_mail = NewMail::new(sysop.id, recipient_id, subject, body);
        MailRepository::create(self.db.conn(), &new_mail)?;

        Ok(true)
    }

    /// Broadcast a notification to all active users (except SysOp).
    ///
    /// # Arguments
    ///
    /// * `subject` - The mail subject
    /// * `body` - The mail body
    ///
    /// # Returns
    ///
    /// Returns the number of mails sent.
    pub fn broadcast_notification(&self, subject: &str, body: &str) -> Result<usize> {
        let sysop = match self.get_sysop_user()? {
            Some(u) => u,
            None => return Ok(0),
        };

        let repo = UserRepository::new(self.db);
        let users = repo.list_active()?;

        let mut count = 0;
        for user in users {
            // Skip the sender (SysOp)
            if user.id == sysop.id {
                continue;
            }

            let new_mail = NewMail::new(sysop.id, user.id, subject, body);
            MailRepository::create(self.db.conn(), &new_mail)?;
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewUser;

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_sysop(db: &Database) -> User {
        let repo = UserRepository::new(db);
        let mut sysop = NewUser::new("sysop", "password123", "System Operator");
        sysop.role = Role::SysOp;
        repo.create(&sysop).unwrap()
    }

    fn create_member(db: &Database, username: &str, nickname: &str) -> User {
        let repo = UserRepository::new(db);
        let user = NewUser::new(username, "password123", nickname);
        repo.create(&user).unwrap()
    }

    #[test]
    fn test_get_sysop_user_exists() {
        let db = setup_db();
        let sysop = create_sysop(&db);

        let service = SystemMailService::new(&db);
        let result = service.get_sysop_user().unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, sysop.id);
    }

    #[test]
    fn test_get_sysop_user_not_exists() {
        let db = setup_db();
        create_member(&db, "alice", "Alice");

        let service = SystemMailService::new(&db);
        let result = service.get_sysop_user().unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_send_welcome_mail_success() {
        let db = setup_db();
        let _sysop = create_sysop(&db);
        let member = create_member(&db, "alice", "Alice");

        let service = SystemMailService::new(&db);
        let sent = service.send_welcome_mail(&member).unwrap();

        assert!(sent);

        // Verify mail was created
        let inbox = MailRepository::list_inbox(db.conn(), member.id).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].subject, WELCOME_MAIL_SUBJECT);
        assert!(inbox[0].body.contains("Aliceさん"));
    }

    #[test]
    fn test_send_welcome_mail_no_sysop() {
        let db = setup_db();
        let member = create_member(&db, "alice", "Alice");

        let service = SystemMailService::new(&db);
        let sent = service.send_welcome_mail(&member).unwrap();

        assert!(!sent);

        // No mail should be created
        let inbox = MailRepository::list_inbox(db.conn(), member.id).unwrap();
        assert!(inbox.is_empty());
    }

    #[test]
    fn test_send_welcome_mail_to_sysop_skipped() {
        let db = setup_db();
        let sysop = create_sysop(&db);

        let service = SystemMailService::new(&db);
        let sent = service.send_welcome_mail(&sysop).unwrap();

        assert!(!sent);
    }

    #[test]
    fn test_send_welcome_mail_by_id() {
        let db = setup_db();
        let _sysop = create_sysop(&db);
        let member = create_member(&db, "bob", "Bob");

        let service = SystemMailService::new(&db);
        let sent = service.send_welcome_mail_by_id(member.id).unwrap();

        assert!(sent);

        let inbox = MailRepository::list_inbox(db.conn(), member.id).unwrap();
        assert_eq!(inbox.len(), 1);
    }

    #[test]
    fn test_send_welcome_mail_by_id_not_found() {
        let db = setup_db();
        let _sysop = create_sysop(&db);

        let service = SystemMailService::new(&db);
        let result = service.send_welcome_mail_by_id(999);

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_generate_welcome_body() {
        let db = setup_db();
        let service = SystemMailService::new(&db);

        let body = service.generate_welcome_body("テストユーザー");

        assert!(body.contains("テストユーザーさん、HOBBSへようこそ！"));
        assert!(body.contains("掲示板"));
        assert!(body.contains("チャット"));
        assert!(body.contains("メール"));
    }

    #[test]
    fn test_send_notification_success() {
        let db = setup_db();
        let _sysop = create_sysop(&db);
        let member = create_member(&db, "alice", "Alice");

        let service = SystemMailService::new(&db);
        let sent = service
            .send_notification(member.id, "お知らせ", "テスト通知です")
            .unwrap();

        assert!(sent);

        let inbox = MailRepository::list_inbox(db.conn(), member.id).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].subject, "お知らせ");
        assert_eq!(inbox[0].body, "テスト通知です");
    }

    #[test]
    fn test_send_notification_no_sysop() {
        let db = setup_db();
        let member = create_member(&db, "alice", "Alice");

        let service = SystemMailService::new(&db);
        let sent = service
            .send_notification(member.id, "お知らせ", "テスト")
            .unwrap();

        assert!(!sent);
    }

    #[test]
    fn test_send_notification_to_sysop_skipped() {
        let db = setup_db();
        let sysop = create_sysop(&db);

        let service = SystemMailService::new(&db);
        let sent = service
            .send_notification(sysop.id, "お知らせ", "テスト")
            .unwrap();

        assert!(!sent);
    }

    #[test]
    fn test_broadcast_notification() {
        let db = setup_db();
        let _sysop = create_sysop(&db);
        let alice = create_member(&db, "alice", "Alice");
        let bob = create_member(&db, "bob", "Bob");

        let service = SystemMailService::new(&db);
        let count = service
            .broadcast_notification("重要なお知らせ", "全員へのメッセージ")
            .unwrap();

        assert_eq!(count, 2);

        // Both users should have received the mail
        let alice_inbox = MailRepository::list_inbox(db.conn(), alice.id).unwrap();
        let bob_inbox = MailRepository::list_inbox(db.conn(), bob.id).unwrap();

        assert_eq!(alice_inbox.len(), 1);
        assert_eq!(bob_inbox.len(), 1);
        assert_eq!(alice_inbox[0].subject, "重要なお知らせ");
    }

    #[test]
    fn test_broadcast_notification_no_sysop() {
        let db = setup_db();
        create_member(&db, "alice", "Alice");

        let service = SystemMailService::new(&db);
        let count = service
            .broadcast_notification("お知らせ", "テスト")
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_broadcast_notification_only_sysop() {
        let db = setup_db();
        let _sysop = create_sysop(&db);

        let service = SystemMailService::new(&db);
        let count = service
            .broadcast_notification("お知らせ", "テスト")
            .unwrap();

        // SysOp is skipped, so no mails sent
        assert_eq!(count, 0);
    }
}
