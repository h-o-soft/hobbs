//! Mail service for HOBBS.
//!
//! This module provides high-level mail operations with business logic
//! including recipient validation, automatic read marking, and access control.

use crate::db::{Database, UserRepository};
use crate::{HobbsError, Result};

use super::repository::MailRepository;
use super::types::{Mail, NewMail, MAX_BODY_LENGTH, MAX_SUBJECT_LENGTH};

/// Request to send a mail.
#[derive(Debug, Clone)]
pub struct SendMailRequest {
    /// Sender user ID.
    pub sender_id: i64,
    /// Recipient username.
    pub recipient_username: String,
    /// Mail subject.
    pub subject: String,
    /// Mail body.
    pub body: String,
}

impl SendMailRequest {
    /// Create a new send mail request.
    pub fn new(
        sender_id: i64,
        recipient_username: impl Into<String>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            sender_id,
            recipient_username: recipient_username.into(),
            subject: subject.into(),
            body: body.into(),
        }
    }
}

/// Service for mail operations.
pub struct MailService<'a> {
    db: &'a Database,
}

impl<'a> MailService<'a> {
    /// Create a new MailService with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Send a mail.
    ///
    /// Validates the request and creates the mail in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Subject is empty or too long
    /// - Body is empty or too long
    /// - Recipient user doesn't exist
    /// - Sender and recipient are the same user
    pub fn send_mail(&self, request: &SendMailRequest) -> Result<Mail> {
        // Validate subject
        let subject = request.subject.trim();
        if subject.is_empty() {
            return Err(HobbsError::Validation("件名を入力してください".to_string()));
        }
        if subject.chars().count() > MAX_SUBJECT_LENGTH {
            return Err(HobbsError::Validation(format!(
                "件名は{MAX_SUBJECT_LENGTH}文字以内で入力してください"
            )));
        }

        // Validate body
        let body = request.body.trim();
        if body.is_empty() {
            return Err(HobbsError::Validation("本文を入力してください".to_string()));
        }
        if body.chars().count() > MAX_BODY_LENGTH {
            return Err(HobbsError::Validation(format!(
                "本文は{MAX_BODY_LENGTH}文字以内で入力してください"
            )));
        }

        // Find recipient user
        let user_repo = UserRepository::new(self.db);
        let recipient = user_repo
            .get_by_username(&request.recipient_username)?
            .ok_or_else(|| HobbsError::NotFound("宛先ユーザー".to_string()))?;

        // Check if recipient is active
        if !recipient.is_active {
            return Err(HobbsError::Validation(
                "宛先ユーザーは現在利用できません".to_string(),
            ));
        }

        // Prevent sending to self
        if recipient.id == request.sender_id {
            return Err(HobbsError::Validation(
                "自分自身にメールを送ることはできません".to_string(),
            ));
        }

        // Create the mail
        let new_mail = NewMail::new(request.sender_id, recipient.id, subject, body);
        let mail = MailRepository::create(self.db.conn(), &new_mail)?;

        Ok(mail)
    }

    /// List inbox (received mails) for a user.
    ///
    /// Returns mails where the user is the recipient and hasn't deleted them.
    pub fn list_inbox(&self, user_id: i64) -> Result<Vec<Mail>> {
        let mails = MailRepository::list_inbox(self.db.conn(), user_id)?;
        Ok(mails)
    }

    /// List sent mails for a user.
    ///
    /// Returns mails where the user is the sender and hasn't deleted them.
    pub fn list_sent(&self, user_id: i64) -> Result<Vec<Mail>> {
        let mails = MailRepository::list_sent(self.db.conn(), user_id)?;
        Ok(mails)
    }

    /// Get a mail by ID with access control.
    ///
    /// Only the sender or recipient can view the mail.
    /// When the recipient views the mail, it's automatically marked as read.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Mail doesn't exist
    /// - User is neither sender nor recipient
    /// - Mail has been deleted by the user
    pub fn get_mail(&self, mail_id: i64, user_id: i64) -> Result<Mail> {
        let mail = MailRepository::get_by_id(self.db.conn(), mail_id)?
            .ok_or_else(|| HobbsError::NotFound("メール".to_string()))?;

        // Check access permission
        let is_sender = mail.sender_id == user_id;
        let is_recipient = mail.recipient_id == user_id;

        if !is_sender && !is_recipient {
            return Err(HobbsError::Permission(
                "このメールを閲覧する権限がありません".to_string(),
            ));
        }

        // Check if deleted by user
        if is_sender && mail.is_deleted_by_sender {
            return Err(HobbsError::NotFound("メール".to_string()));
        }
        if is_recipient && mail.is_deleted_by_recipient {
            return Err(HobbsError::NotFound("メール".to_string()));
        }

        // Mark as read if recipient is viewing
        if is_recipient && !mail.is_read {
            MailRepository::mark_as_read(self.db.conn(), mail_id)?;
            // Return updated mail
            return MailRepository::get_by_id(self.db.conn(), mail_id)?
                .ok_or_else(|| HobbsError::NotFound("メール".to_string()));
        }

        Ok(mail)
    }

    /// Delete a mail (logical deletion).
    ///
    /// Marks the mail as deleted by the user. The mail is only physically
    /// deleted when both sender and recipient have deleted it.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Mail doesn't exist
    /// - User is neither sender nor recipient
    pub fn delete_mail(&self, mail_id: i64, user_id: i64) -> Result<()> {
        let mail = MailRepository::get_by_id(self.db.conn(), mail_id)?
            .ok_or_else(|| HobbsError::NotFound("メール".to_string()))?;

        // Check if user is sender or recipient
        let is_sender = mail.sender_id == user_id;
        let is_recipient = mail.recipient_id == user_id;

        if !is_sender && !is_recipient {
            return Err(HobbsError::Permission(
                "このメールを削除する権限がありません".to_string(),
            ));
        }

        // Delete by user
        MailRepository::delete_by_user(self.db.conn(), mail_id, user_id)?;

        // Check if both have deleted - if so, purge
        let updated_mail = MailRepository::get_by_id(self.db.conn(), mail_id)?;
        if let Some(m) = updated_mail {
            if m.can_be_purged() {
                MailRepository::purge(self.db.conn(), mail_id)?;
            }
        }

        Ok(())
    }

    /// Count unread mails for a user.
    pub fn count_unread(&self, user_id: i64) -> Result<i64> {
        let count = MailRepository::count_unread(self.db.conn(), user_id)?;
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

    fn create_test_users(db: &Database) -> (i64, i64) {
        let repo = UserRepository::new(db);
        let user1 = NewUser::new("alice", "password123", "Alice");
        let user2 = NewUser::new("bob", "password123", "Bob");
        let id1 = repo.create(&user1).unwrap().id;
        let id2 = repo.create(&user2).unwrap().id;
        (id1, id2)
    }

    #[test]
    fn test_send_mail_success() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "Hello", "How are you?");
        let mail = service.send_mail(&request).unwrap();

        assert_eq!(mail.sender_id, sender_id);
        assert_eq!(mail.subject, "Hello");
        assert_eq!(mail.body, "How are you?");
        assert!(!mail.is_read);
    }

    #[test]
    fn test_send_mail_empty_subject() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "", "Body");
        let result = service.send_mail(&request);

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[test]
    fn test_send_mail_empty_body() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "Subject", "   ");
        let result = service.send_mail(&request);

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[test]
    fn test_send_mail_subject_too_long() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let long_subject = "あ".repeat(MAX_SUBJECT_LENGTH + 1);
        let request = SendMailRequest::new(sender_id, "bob", long_subject, "Body");
        let result = service.send_mail(&request);

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[test]
    fn test_send_mail_recipient_not_found() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "nonexistent", "Subject", "Body");
        let result = service.send_mail(&request);

        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_send_mail_to_self() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "alice", "Subject", "Body");
        let result = service.send_mail(&request);

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[test]
    fn test_send_mail_to_inactive_user() {
        let db = setup_db();
        let repo = UserRepository::new(&db);
        let user1 = NewUser::new("alice", "password123", "Alice");
        let user2 = NewUser::new("bob", "password123", "Bob");
        let sender_id = repo.create(&user1).unwrap().id;
        let bob = repo.create(&user2).unwrap();

        // Deactivate bob
        use crate::db::UserUpdate;
        repo.update(bob.id, &UserUpdate::new().is_active(false))
            .unwrap();

        let service = MailService::new(&db);
        let request = SendMailRequest::new(sender_id, "bob", "Subject", "Body");
        let result = service.send_mail(&request);

        assert!(matches!(result, Err(HobbsError::Validation(_))));
    }

    #[test]
    fn test_list_inbox() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);
        let service = MailService::new(&db);

        // Send two mails
        let request1 = SendMailRequest::new(sender_id, "bob", "Mail 1", "Body 1");
        let request2 = SendMailRequest::new(sender_id, "bob", "Mail 2", "Body 2");
        service.send_mail(&request1).unwrap();
        service.send_mail(&request2).unwrap();

        let inbox = service.list_inbox(recipient_id).unwrap();
        assert_eq!(inbox.len(), 2);
        // Most recent first
        assert_eq!(inbox[0].subject, "Mail 2");
    }

    #[test]
    fn test_list_sent() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "Sent Mail", "Body");
        service.send_mail(&request).unwrap();

        let sent = service.list_sent(sender_id).unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].subject, "Sent Mail");
    }

    #[test]
    fn test_get_mail_by_recipient() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "Test", "Body");
        let sent_mail = service.send_mail(&request).unwrap();
        assert!(!sent_mail.is_read);

        // Recipient views the mail - should be marked as read
        let mail = service.get_mail(sent_mail.id, recipient_id).unwrap();
        assert!(mail.is_read);
    }

    #[test]
    fn test_get_mail_by_sender() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "Test", "Body");
        let sent_mail = service.send_mail(&request).unwrap();

        // Sender views the mail - should NOT be marked as read
        let mail = service.get_mail(sent_mail.id, sender_id).unwrap();
        assert!(!mail.is_read);
    }

    #[test]
    fn test_get_mail_not_found() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let result = service.get_mail(999, sender_id);
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_get_mail_no_permission() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let repo = UserRepository::new(&db);
        let user3 = NewUser::new("charlie", "password123", "Charlie");
        let user3_id = repo.create(&user3).unwrap().id;

        let service = MailService::new(&db);
        let request = SendMailRequest::new(sender_id, "bob", "Test", "Body");
        let mail = service.send_mail(&request).unwrap();

        // Third user tries to view
        let result = service.get_mail(mail.id, user3_id);
        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[test]
    fn test_get_mail_deleted_by_recipient() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "Test", "Body");
        let mail = service.send_mail(&request).unwrap();

        // Recipient deletes the mail
        service.delete_mail(mail.id, recipient_id).unwrap();

        // Recipient tries to view - should fail
        let result = service.get_mail(mail.id, recipient_id);
        assert!(matches!(result, Err(HobbsError::NotFound(_))));

        // Sender can still view
        let sender_view = service.get_mail(mail.id, sender_id);
        assert!(sender_view.is_ok());
    }

    #[test]
    fn test_delete_mail_by_sender() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "Test", "Body");
        let mail = service.send_mail(&request).unwrap();

        service.delete_mail(mail.id, sender_id).unwrap();

        // Sender can't see it
        let result = service.get_mail(mail.id, sender_id);
        assert!(matches!(result, Err(HobbsError::NotFound(_))));

        // Recipient can still see it
        let inbox = service.list_inbox(recipient_id).unwrap();
        assert_eq!(inbox.len(), 1);
    }

    #[test]
    fn test_delete_mail_by_both_purges() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);
        let service = MailService::new(&db);

        let request = SendMailRequest::new(sender_id, "bob", "Test", "Body");
        let mail = service.send_mail(&request).unwrap();

        // Both delete
        service.delete_mail(mail.id, sender_id).unwrap();
        service.delete_mail(mail.id, recipient_id).unwrap();

        // Mail should be physically deleted
        let db_mail = MailRepository::get_by_id(db.conn(), mail.id).unwrap();
        assert!(db_mail.is_none());
    }

    #[test]
    fn test_delete_mail_not_found() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let service = MailService::new(&db);

        let result = service.delete_mail(999, sender_id);
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_delete_mail_no_permission() {
        let db = setup_db();
        let (sender_id, _) = create_test_users(&db);
        let repo = UserRepository::new(&db);
        let user3 = NewUser::new("charlie", "password123", "Charlie");
        let user3_id = repo.create(&user3).unwrap().id;

        let service = MailService::new(&db);
        let request = SendMailRequest::new(sender_id, "bob", "Test", "Body");
        let mail = service.send_mail(&request).unwrap();

        let result = service.delete_mail(mail.id, user3_id);
        assert!(matches!(result, Err(HobbsError::Permission(_))));
    }

    #[test]
    fn test_count_unread() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);
        let service = MailService::new(&db);

        assert_eq!(service.count_unread(recipient_id).unwrap(), 0);

        // Send two mails
        service
            .send_mail(&SendMailRequest::new(sender_id, "bob", "Mail 1", "Body"))
            .unwrap();
        let mail2 = service
            .send_mail(&SendMailRequest::new(sender_id, "bob", "Mail 2", "Body"))
            .unwrap();

        assert_eq!(service.count_unread(recipient_id).unwrap(), 2);

        // Read one mail
        service.get_mail(mail2.id, recipient_id).unwrap();

        assert_eq!(service.count_unread(recipient_id).unwrap(), 1);
    }
}
