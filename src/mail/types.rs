//! Mail types for HOBBS.

use chrono::{DateTime, Utc};

/// Maximum length for mail subject.
pub const MAX_SUBJECT_LENGTH: usize = 100;

/// Maximum length for mail body.
pub const MAX_BODY_LENGTH: usize = 10000;

/// A mail message.
#[derive(Debug, Clone)]
pub struct Mail {
    /// Mail ID.
    pub id: i64,
    /// Sender user ID.
    pub sender_id: i64,
    /// Recipient user ID.
    pub recipient_id: i64,
    /// Mail subject.
    pub subject: String,
    /// Mail body.
    pub body: String,
    /// Whether the mail has been read by the recipient.
    pub is_read: bool,
    /// Whether the mail has been deleted by the sender.
    pub is_deleted_by_sender: bool,
    /// Whether the mail has been deleted by the recipient.
    pub is_deleted_by_recipient: bool,
    /// When the mail was created.
    pub created_at: DateTime<Utc>,
}

impl Mail {
    /// Check if the mail is visible to the sender.
    pub fn is_visible_to_sender(&self) -> bool {
        !self.is_deleted_by_sender
    }

    /// Check if the mail is visible to the recipient.
    pub fn is_visible_to_recipient(&self) -> bool {
        !self.is_deleted_by_recipient
    }

    /// Check if the mail can be physically deleted.
    /// A mail can be deleted when both sender and recipient have deleted it.
    pub fn can_be_purged(&self) -> bool {
        self.is_deleted_by_sender && self.is_deleted_by_recipient
    }
}

/// New mail for creation.
#[derive(Debug, Clone)]
pub struct NewMail {
    /// Sender user ID.
    pub sender_id: i64,
    /// Recipient user ID.
    pub recipient_id: i64,
    /// Mail subject.
    pub subject: String,
    /// Mail body.
    pub body: String,
}

impl NewMail {
    /// Create a new mail.
    pub fn new(
        sender_id: i64,
        recipient_id: i64,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            sender_id,
            recipient_id,
            subject: subject.into(),
            body: body.into(),
        }
    }
}

/// Mail update request.
#[derive(Debug, Clone, Default)]
pub struct MailUpdate {
    /// Mark as read.
    pub is_read: Option<bool>,
    /// Mark as deleted by sender.
    pub is_deleted_by_sender: Option<bool>,
    /// Mark as deleted by recipient.
    pub is_deleted_by_recipient: Option<bool>,
}

impl MailUpdate {
    /// Create a new update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark as read.
    pub fn mark_as_read(mut self) -> Self {
        self.is_read = Some(true);
        self
    }

    /// Mark as unread.
    pub fn mark_as_unread(mut self) -> Self {
        self.is_read = Some(false);
        self
    }

    /// Mark as deleted by sender.
    pub fn delete_by_sender(mut self) -> Self {
        self.is_deleted_by_sender = Some(true);
        self
    }

    /// Mark as deleted by recipient.
    pub fn delete_by_recipient(mut self) -> Self {
        self.is_deleted_by_recipient = Some(true);
        self
    }

    /// Check if the update is empty.
    pub fn is_empty(&self) -> bool {
        self.is_read.is_none()
            && self.is_deleted_by_sender.is_none()
            && self.is_deleted_by_recipient.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_mail() {
        let mail = NewMail::new(1, 2, "Hello", "Body text");
        assert_eq!(mail.sender_id, 1);
        assert_eq!(mail.recipient_id, 2);
        assert_eq!(mail.subject, "Hello");
        assert_eq!(mail.body, "Body text");
    }

    #[test]
    fn test_mail_visibility_sender() {
        let mail = Mail {
            id: 1,
            sender_id: 1,
            recipient_id: 2,
            subject: "Test".to_string(),
            body: "Body".to_string(),
            is_read: false,
            is_deleted_by_sender: false,
            is_deleted_by_recipient: false,
            created_at: Utc::now(),
        };
        assert!(mail.is_visible_to_sender());

        let deleted_mail = Mail {
            is_deleted_by_sender: true,
            ..mail.clone()
        };
        assert!(!deleted_mail.is_visible_to_sender());
    }

    #[test]
    fn test_mail_visibility_recipient() {
        let mail = Mail {
            id: 1,
            sender_id: 1,
            recipient_id: 2,
            subject: "Test".to_string(),
            body: "Body".to_string(),
            is_read: false,
            is_deleted_by_sender: false,
            is_deleted_by_recipient: false,
            created_at: Utc::now(),
        };
        assert!(mail.is_visible_to_recipient());

        let deleted_mail = Mail {
            is_deleted_by_recipient: true,
            ..mail.clone()
        };
        assert!(!deleted_mail.is_visible_to_recipient());
    }

    #[test]
    fn test_mail_can_be_purged() {
        let mail = Mail {
            id: 1,
            sender_id: 1,
            recipient_id: 2,
            subject: "Test".to_string(),
            body: "Body".to_string(),
            is_read: false,
            is_deleted_by_sender: false,
            is_deleted_by_recipient: false,
            created_at: Utc::now(),
        };
        assert!(!mail.can_be_purged());

        let sender_deleted = Mail {
            is_deleted_by_sender: true,
            ..mail.clone()
        };
        assert!(!sender_deleted.can_be_purged());

        let both_deleted = Mail {
            is_deleted_by_sender: true,
            is_deleted_by_recipient: true,
            ..mail.clone()
        };
        assert!(both_deleted.can_be_purged());
    }

    #[test]
    fn test_mail_update_empty() {
        let update = MailUpdate::new();
        assert!(update.is_empty());
    }

    #[test]
    fn test_mail_update_mark_as_read() {
        let update = MailUpdate::new().mark_as_read();
        assert_eq!(update.is_read, Some(true));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_mail_update_mark_as_unread() {
        let update = MailUpdate::new().mark_as_unread();
        assert_eq!(update.is_read, Some(false));
    }

    #[test]
    fn test_mail_update_delete_by_sender() {
        let update = MailUpdate::new().delete_by_sender();
        assert_eq!(update.is_deleted_by_sender, Some(true));
    }

    #[test]
    fn test_mail_update_delete_by_recipient() {
        let update = MailUpdate::new().delete_by_recipient();
        assert_eq!(update.is_deleted_by_recipient, Some(true));
    }

    #[test]
    fn test_mail_update_combined() {
        let update = MailUpdate::new().mark_as_read().delete_by_recipient();
        assert_eq!(update.is_read, Some(true));
        assert_eq!(update.is_deleted_by_recipient, Some(true));
        assert!(update.is_deleted_by_sender.is_none());
    }
}
