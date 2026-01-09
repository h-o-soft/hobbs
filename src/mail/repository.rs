//! Mail repository for HOBBS.

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Row};

use super::types::{Mail, MailUpdate, NewMail};
use crate::db::DbPool;
use crate::{HobbsError, Result};

/// Repository for mail operations.
pub struct MailRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> MailRepository<'a> {
    /// Create a new MailRepository with the given database pool reference.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new mail.
    pub async fn create(&self, mail: &NewMail) -> Result<Mail> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO mails (sender_id, recipient_id, subject, body)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(mail.sender_id)
        .bind(mail.recipient_id)
        .bind(&mail.subject)
        .bind(&mail.body)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("mail".to_string()))
    }

    /// Get a mail by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Mail>> {
        let row = sqlx::query(
            r#"
            SELECT id, sender_id, recipient_id, subject, body,
                   is_read, is_deleted_by_sender, is_deleted_by_recipient, created_at
            FROM mails
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(Self::row_to_mail(&row)?)),
            None => Ok(None),
        }
    }

    /// List inbox mails for a user (received mails, not deleted by recipient).
    pub async fn list_inbox(&self, user_id: i64) -> Result<Vec<Mail>> {
        let rows = sqlx::query(
            r#"
            SELECT id, sender_id, recipient_id, subject, body,
                   is_read, is_deleted_by_sender, is_deleted_by_recipient, created_at
            FROM mails
            WHERE recipient_id = ? AND is_deleted_by_recipient = 0
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        rows.iter().map(Self::row_to_mail).collect()
    }

    /// List sent mails for a user (not deleted by sender).
    pub async fn list_sent(&self, user_id: i64) -> Result<Vec<Mail>> {
        let rows = sqlx::query(
            r#"
            SELECT id, sender_id, recipient_id, subject, body,
                   is_read, is_deleted_by_sender, is_deleted_by_recipient, created_at
            FROM mails
            WHERE sender_id = ? AND is_deleted_by_sender = 0
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        rows.iter().map(Self::row_to_mail).collect()
    }

    /// Count unread mails for a user.
    pub async fn count_unread(&self, user_id: i64) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM mails
            WHERE recipient_id = ? AND is_read = 0 AND is_deleted_by_recipient = 0
            "#,
        )
        .bind(user_id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }

    /// Update a mail.
    pub async fn update(&self, id: i64, update: &MailUpdate) -> Result<bool> {
        if update.is_empty() {
            return Ok(false);
        }

        #[cfg(feature = "sqlite")]
        let mut query: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("UPDATE mails SET ");
        #[cfg(feature = "postgres")]
        let mut query: QueryBuilder<sqlx::Postgres> = QueryBuilder::new("UPDATE mails SET ");
        let mut separated = query.separated(", ");

        if let Some(is_read) = update.is_read {
            separated.push("is_read = ");
            separated.push_bind_unseparated(is_read as i32);
        }

        if let Some(deleted) = update.is_deleted_by_sender {
            separated.push("is_deleted_by_sender = ");
            separated.push_bind_unseparated(deleted as i32);
        }

        if let Some(deleted) = update.is_deleted_by_recipient {
            separated.push("is_deleted_by_recipient = ");
            separated.push_bind_unseparated(deleted as i32);
        }

        query.push(" WHERE id = ");
        query.push_bind(id);

        let result = query
            .build()
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Mark a mail as read.
    pub async fn mark_as_read(&self, id: i64) -> Result<bool> {
        self.update(id, &MailUpdate::new().mark_as_read()).await
    }

    /// Delete a mail (logical deletion).
    /// Marks the mail as deleted by the specified user.
    pub async fn delete_by_user(&self, id: i64, user_id: i64) -> Result<bool> {
        // First, get the mail to determine if user is sender or recipient
        let mail = match self.get_by_id(id).await? {
            Some(m) => m,
            None => return Ok(false),
        };

        let update = if mail.sender_id == user_id {
            MailUpdate::new().delete_by_sender()
        } else if mail.recipient_id == user_id {
            MailUpdate::new().delete_by_recipient()
        } else {
            // User is neither sender nor recipient
            return Ok(false);
        };

        self.update(id, &update).await
    }

    /// Physically delete a mail.
    /// Should only be used for mails that have been deleted by both sender and recipient.
    pub async fn purge(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM mails WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Purge all mails that have been deleted by both sender and recipient.
    pub async fn purge_all_deleted(&self) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM mails WHERE is_deleted_by_sender = 1 AND is_deleted_by_recipient = 1",
        )
        .execute(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Count total mails in the database.
    pub async fn count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mails")
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }

    /// Convert a database row to a Mail.
    #[cfg(feature = "sqlite")]
    fn row_to_mail(row: &sqlx::sqlite::SqliteRow) -> Result<Mail> {
        let created_at_str: String = row
            .try_get("created_at")
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Mail {
            id: row
                .try_get("id")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            sender_id: row
                .try_get("sender_id")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            recipient_id: row
                .try_get("recipient_id")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            subject: row
                .try_get("subject")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            body: row
                .try_get("body")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            is_read: row
                .try_get::<i32, _>("is_read")
                .map_err(|e| HobbsError::Database(e.to_string()))?
                != 0,
            is_deleted_by_sender: row
                .try_get::<i32, _>("is_deleted_by_sender")
                .map_err(|e| HobbsError::Database(e.to_string()))?
                != 0,
            is_deleted_by_recipient: row
                .try_get::<i32, _>("is_deleted_by_recipient")
                .map_err(|e| HobbsError::Database(e.to_string()))?
                != 0,
            created_at,
        })
    }

    /// Convert a database row to a Mail.
    #[cfg(feature = "postgres")]
    fn row_to_mail(row: &sqlx::postgres::PgRow) -> Result<Mail> {
        let created_at_str: String = row
            .try_get("created_at")
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Mail {
            id: row
                .try_get("id")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            sender_id: row
                .try_get("sender_id")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            recipient_id: row
                .try_get("recipient_id")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            subject: row
                .try_get("subject")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            body: row
                .try_get("body")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            is_read: row
                .try_get::<bool, _>("is_read")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            is_deleted_by_sender: row
                .try_get::<bool, _>("is_deleted_by_sender")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            is_deleted_by_recipient: row
                .try_get::<bool, _>("is_deleted_by_recipient")
                .map_err(|e| HobbsError::Database(e.to_string()))?,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{NewUser, UserRepository};
    use crate::Database;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_test_users(db: &Database) -> (i64, i64) {
        let repo = UserRepository::new(db.pool());
        let user1 = NewUser::new("alice", "password123", "Alice");
        let user2 = NewUser::new("bob", "password123", "Bob");
        let id1 = repo.create(&user1).await.unwrap().id;
        let id2 = repo.create(&user2).await.unwrap().id;
        (id1, id2)
    }

    #[tokio::test]
    async fn test_create_mail() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());
        let new_mail = NewMail::new(sender_id, recipient_id, "Hello", "How are you?");
        let mail = repo.create(&new_mail).await.unwrap();

        assert!(mail.id > 0);
        assert_eq!(mail.sender_id, sender_id);
        assert_eq!(mail.recipient_id, recipient_id);
        assert_eq!(mail.subject, "Hello");
        assert_eq!(mail.body, "How are you?");
        assert!(!mail.is_read);
        assert!(!mail.is_deleted_by_sender);
        assert!(!mail.is_deleted_by_recipient);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());
        let new_mail = NewMail::new(sender_id, recipient_id, "Test", "Body");
        let created = repo.create(&new_mail).await.unwrap();

        let retrieved = repo.get_by_id(created.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.subject, "Test");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let db = setup_db().await;
        let repo = MailRepository::new(db.pool());
        let result = repo.get_by_id(999).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_inbox() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        // Create some mails
        repo.create(&NewMail::new(sender_id, recipient_id, "Mail 1", "Body 1"))
            .await
            .unwrap();
        repo.create(&NewMail::new(sender_id, recipient_id, "Mail 2", "Body 2"))
            .await
            .unwrap();

        let inbox = repo.list_inbox(recipient_id).await.unwrap();
        assert_eq!(inbox.len(), 2);
        // Should be ordered by created_at DESC (most recent first)
        assert_eq!(inbox[0].subject, "Mail 2");
        assert_eq!(inbox[1].subject, "Mail 1");
    }

    #[tokio::test]
    async fn test_list_inbox_excludes_deleted() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        let mail = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        // Delete by recipient
        repo.update(mail.id, &MailUpdate::new().delete_by_recipient())
            .await
            .unwrap();

        let inbox = repo.list_inbox(recipient_id).await.unwrap();
        assert!(inbox.is_empty());
    }

    #[tokio::test]
    async fn test_list_sent() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        repo.create(&NewMail::new(sender_id, recipient_id, "Sent Mail", "Body"))
            .await
            .unwrap();

        let sent = repo.list_sent(sender_id).await.unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].subject, "Sent Mail");
    }

    #[tokio::test]
    async fn test_list_sent_excludes_deleted() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        let mail = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        repo.update(mail.id, &MailUpdate::new().delete_by_sender())
            .await
            .unwrap();

        let sent = repo.list_sent(sender_id).await.unwrap();
        assert!(sent.is_empty());
    }

    #[tokio::test]
    async fn test_count_unread() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        // Initially no unread
        assert_eq!(repo.count_unread(recipient_id).await.unwrap(), 0);

        // Create two mails
        repo.create(&NewMail::new(sender_id, recipient_id, "Mail 1", "Body"))
            .await
            .unwrap();
        let mail2 = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail 2", "Body"))
            .await
            .unwrap();

        assert_eq!(repo.count_unread(recipient_id).await.unwrap(), 2);

        // Mark one as read
        repo.mark_as_read(mail2.id).await.unwrap();

        assert_eq!(repo.count_unread(recipient_id).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_mark_as_read() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        let mail = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        assert!(!mail.is_read);

        repo.mark_as_read(mail.id).await.unwrap();

        let updated = repo.get_by_id(mail.id).await.unwrap().unwrap();
        assert!(updated.is_read);
    }

    #[tokio::test]
    async fn test_delete_by_user_sender() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        let mail = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        repo.delete_by_user(mail.id, sender_id).await.unwrap();

        let updated = repo.get_by_id(mail.id).await.unwrap().unwrap();
        assert!(updated.is_deleted_by_sender);
        assert!(!updated.is_deleted_by_recipient);
    }

    #[tokio::test]
    async fn test_delete_by_user_recipient() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        let mail = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        repo.delete_by_user(mail.id, recipient_id).await.unwrap();

        let updated = repo.get_by_id(mail.id).await.unwrap().unwrap();
        assert!(!updated.is_deleted_by_sender);
        assert!(updated.is_deleted_by_recipient);
    }

    #[tokio::test]
    async fn test_delete_by_user_not_involved() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        // Create a third user
        let user_repo = UserRepository::new(db.pool());
        let user3 = NewUser::new("charlie", "password123", "Charlie");
        let user3_id = user_repo.create(&user3).await.unwrap().id;

        let repo = MailRepository::new(db.pool());

        let mail = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        // User3 tries to delete - should fail
        let result = repo.delete_by_user(mail.id, user3_id).await.unwrap();
        assert!(!result);

        // Mail should be unchanged
        let unchanged = repo.get_by_id(mail.id).await.unwrap().unwrap();
        assert!(!unchanged.is_deleted_by_sender);
        assert!(!unchanged.is_deleted_by_recipient);
    }

    #[tokio::test]
    async fn test_purge() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        let mail = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        repo.purge(mail.id).await.unwrap();

        let result = repo.get_by_id(mail.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_purge_all_deleted() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        // Create three mails
        let mail1 = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail 1", "Body"))
            .await
            .unwrap();
        let mail2 = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail 2", "Body"))
            .await
            .unwrap();
        let _mail3 = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail 3", "Body"))
            .await
            .unwrap();

        // Delete mail1 by both users
        repo.delete_by_user(mail1.id, sender_id).await.unwrap();
        repo.delete_by_user(mail1.id, recipient_id).await.unwrap();

        // Delete mail2 only by sender
        repo.delete_by_user(mail2.id, sender_id).await.unwrap();

        // Purge should only remove mail1
        let purged = repo.purge_all_deleted().await.unwrap();
        assert_eq!(purged, 1);

        // mail1 should be gone
        assert!(repo.get_by_id(mail1.id).await.unwrap().is_none());

        // mail2 and mail3 should still exist
        assert!(repo.get_by_id(mail2.id).await.unwrap().is_some());

        assert_eq!(repo.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_count() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        assert_eq!(repo.count().await.unwrap(), 0);

        repo.create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        assert_eq!(repo.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_update_empty() {
        let db = setup_db().await;
        let (sender_id, recipient_id) = create_test_users(&db).await;

        let repo = MailRepository::new(db.pool());

        let mail = repo
            .create(&NewMail::new(sender_id, recipient_id, "Mail", "Body"))
            .await
            .unwrap();

        let result = repo.update(mail.id, &MailUpdate::new()).await.unwrap();
        assert!(!result);
    }
}
