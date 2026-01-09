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
    pub async fn get_sysop_user(&self) -> Result<Option<User>> {
        let repo = UserRepository::new(self.db.pool());
        let sysops = repo.list_by_role(Role::SysOp).await?;
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
    pub async fn send_welcome_mail(&self, recipient: &User) -> Result<bool> {
        // Don't send welcome mail to SysOp (they're the sender)
        if recipient.role == Role::SysOp {
            return Ok(false);
        }

        // Get the SysOp to use as sender
        let sysop = match self.get_sysop_user().await? {
            Some(u) => u,
            None => return Ok(false), // No SysOp, skip welcome mail
        };

        // Generate the mail body with the recipient's nickname
        let body = self.generate_welcome_body(&recipient.nickname);

        // Create and send the mail
        let new_mail = NewMail::new(sysop.id, recipient.id, WELCOME_MAIL_SUBJECT, body);
        let mail_repo = MailRepository::new(self.db.pool());
        mail_repo.create(&new_mail).await?;

        Ok(true)
    }

    /// Send a welcome mail to a user by ID.
    ///
    /// Convenience method that fetches the user and sends the welcome mail.
    pub async fn send_welcome_mail_by_id(&self, user_id: i64) -> Result<bool> {
        let repo = UserRepository::new(self.db.pool());
        let user = repo
            .get_by_id(user_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("user".to_string()))?;

        self.send_welcome_mail(&user).await
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
    pub async fn send_notification(&self, recipient_id: i64, subject: &str, body: &str) -> Result<bool> {
        // Get the SysOp to use as sender
        let sysop = match self.get_sysop_user().await? {
            Some(u) => u,
            None => return Ok(false),
        };

        // Don't send to self
        if sysop.id == recipient_id {
            return Ok(false);
        }

        // Create and send the mail
        let new_mail = NewMail::new(sysop.id, recipient_id, subject, body);
        let mail_repo = MailRepository::new(self.db.pool());
        mail_repo.create(&new_mail).await?;

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
    pub async fn broadcast_notification(&self, subject: &str, body: &str) -> Result<usize> {
        let sysop = match self.get_sysop_user().await? {
            Some(u) => u,
            None => return Ok(0),
        };

        let user_repo = UserRepository::new(self.db.pool());
        let users = user_repo.list_active().await?;

        let mail_repo = MailRepository::new(self.db.pool());
        let mut count = 0;
        for user in users {
            // Skip the sender (SysOp)
            if user.id == sysop.id {
                continue;
            }

            let new_mail = NewMail::new(sysop.id, user.id, subject, body);
            mail_repo.create(&new_mail).await?;
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewUser;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_sysop(db: &Database) -> User {
        let repo = UserRepository::new(db.pool());
        let mut sysop = NewUser::new("sysop", "password123", "System Operator");
        sysop.role = Role::SysOp;
        repo.create(&sysop).await.unwrap()
    }

    async fn create_member(db: &Database, username: &str, nickname: &str) -> User {
        let repo = UserRepository::new(db.pool());
        let user = NewUser::new(username, "password123", nickname);
        repo.create(&user).await.unwrap()
    }

    #[tokio::test]
    async fn test_get_sysop_user_exists() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;

        let service = SystemMailService::new(&db);
        let result = service.get_sysop_user().await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, sysop.id);
    }

    #[tokio::test]
    async fn test_get_sysop_user_not_exists() {
        let db = setup_db().await;
        create_member(&db, "alice", "Alice").await;

        let service = SystemMailService::new(&db);
        let result = service.get_sysop_user().await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_send_welcome_mail_success() {
        let db = setup_db().await;
        let _sysop = create_sysop(&db).await;
        let member = create_member(&db, "alice", "Alice").await;

        let service = SystemMailService::new(&db);
        let sent = service.send_welcome_mail(&member).await.unwrap();

        assert!(sent);

        // Verify mail was created
        let mail_repo = MailRepository::new(db.pool());
        let inbox = mail_repo.list_inbox(member.id).await.unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].subject, WELCOME_MAIL_SUBJECT);
        assert!(inbox[0].body.contains("Aliceさん"));
    }

    #[tokio::test]
    async fn test_send_welcome_mail_no_sysop() {
        let db = setup_db().await;
        let member = create_member(&db, "alice", "Alice").await;

        let service = SystemMailService::new(&db);
        let sent = service.send_welcome_mail(&member).await.unwrap();

        assert!(!sent);

        // No mail should be created
        let mail_repo = MailRepository::new(db.pool());
        let inbox = mail_repo.list_inbox(member.id).await.unwrap();
        assert!(inbox.is_empty());
    }

    #[tokio::test]
    async fn test_send_welcome_mail_to_sysop_skipped() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;

        let service = SystemMailService::new(&db);
        let sent = service.send_welcome_mail(&sysop).await.unwrap();

        assert!(!sent);
    }

    #[tokio::test]
    async fn test_send_welcome_mail_by_id() {
        let db = setup_db().await;
        let _sysop = create_sysop(&db).await;
        let member = create_member(&db, "bob", "Bob").await;

        let service = SystemMailService::new(&db);
        let sent = service.send_welcome_mail_by_id(member.id).await.unwrap();

        assert!(sent);

        let mail_repo = MailRepository::new(db.pool());
        let inbox = mail_repo.list_inbox(member.id).await.unwrap();
        assert_eq!(inbox.len(), 1);
    }

    #[tokio::test]
    async fn test_send_welcome_mail_by_id_not_found() {
        let db = setup_db().await;
        let _sysop = create_sysop(&db).await;

        let service = SystemMailService::new(&db);
        let result = service.send_welcome_mail_by_id(999).await;

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_generate_welcome_body() {
        let db = setup_db().await;
        let service = SystemMailService::new(&db);

        let body = service.generate_welcome_body("テストユーザー");

        assert!(body.contains("テストユーザーさん、HOBBSへようこそ！"));
        assert!(body.contains("掲示板"));
        assert!(body.contains("チャット"));
        assert!(body.contains("メール"));
    }

    #[tokio::test]
    async fn test_send_notification_success() {
        let db = setup_db().await;
        let _sysop = create_sysop(&db).await;
        let member = create_member(&db, "alice", "Alice").await;

        let service = SystemMailService::new(&db);
        let sent = service
            .send_notification(member.id, "お知らせ", "テスト通知です")
            .await
            .unwrap();

        assert!(sent);

        let mail_repo = MailRepository::new(db.pool());
        let inbox = mail_repo.list_inbox(member.id).await.unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].subject, "お知らせ");
        assert_eq!(inbox[0].body, "テスト通知です");
    }

    #[tokio::test]
    async fn test_send_notification_no_sysop() {
        let db = setup_db().await;
        let member = create_member(&db, "alice", "Alice").await;

        let service = SystemMailService::new(&db);
        let sent = service
            .send_notification(member.id, "お知らせ", "テスト")
            .await
            .unwrap();

        assert!(!sent);
    }

    #[tokio::test]
    async fn test_send_notification_to_sysop_skipped() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;

        let service = SystemMailService::new(&db);
        let sent = service
            .send_notification(sysop.id, "お知らせ", "テスト")
            .await
            .unwrap();

        assert!(!sent);
    }

    #[tokio::test]
    async fn test_broadcast_notification() {
        let db = setup_db().await;
        let _sysop = create_sysop(&db).await;
        let alice = create_member(&db, "alice", "Alice").await;
        let bob = create_member(&db, "bob", "Bob").await;

        let service = SystemMailService::new(&db);
        let count = service
            .broadcast_notification("重要なお知らせ", "全員へのメッセージ")
            .await
            .unwrap();

        assert_eq!(count, 2);

        // Both users should have received the mail
        let mail_repo = MailRepository::new(db.pool());
        let alice_inbox = mail_repo.list_inbox(alice.id).await.unwrap();
        let bob_inbox = mail_repo.list_inbox(bob.id).await.unwrap();

        assert_eq!(alice_inbox.len(), 1);
        assert_eq!(bob_inbox.len(), 1);
        assert_eq!(alice_inbox[0].subject, "重要なお知らせ");
    }

    #[tokio::test]
    async fn test_broadcast_notification_no_sysop() {
        let db = setup_db().await;
        create_member(&db, "alice", "Alice").await;

        let service = SystemMailService::new(&db);
        let count = service
            .broadcast_notification("お知らせ", "テスト")
            .await
            .unwrap();

        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_broadcast_notification_only_sysop() {
        let db = setup_db().await;
        let _sysop = create_sysop(&db).await;

        let service = SystemMailService::new(&db);
        let count = service
            .broadcast_notification("お知らせ", "テスト")
            .await
            .unwrap();

        // SysOp is skipped, so no mails sent
        assert_eq!(count, 0);
    }
}
