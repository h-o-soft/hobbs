//! Mail repository for HOBBS.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::types::{Mail, MailUpdate, NewMail};

/// Repository for mail operations.
pub struct MailRepository;

impl MailRepository {
    /// Create a new mail.
    pub fn create(conn: &Connection, mail: &NewMail) -> rusqlite::Result<Mail> {
        conn.execute(
            r#"
            INSERT INTO mails (sender_id, recipient_id, subject, body)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![mail.sender_id, mail.recipient_id, mail.subject, mail.body],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
    }

    /// Get a mail by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> rusqlite::Result<Option<Mail>> {
        conn.query_row(
            r#"
            SELECT id, sender_id, recipient_id, subject, body,
                   is_read, is_deleted_by_sender, is_deleted_by_recipient, created_at
            FROM mails
            WHERE id = ?1
            "#,
            [id],
            Self::map_row,
        )
        .optional()
    }

    /// List inbox mails for a user (received mails, not deleted by recipient).
    pub fn list_inbox(conn: &Connection, user_id: i64) -> rusqlite::Result<Vec<Mail>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, sender_id, recipient_id, subject, body,
                   is_read, is_deleted_by_sender, is_deleted_by_recipient, created_at
            FROM mails
            WHERE recipient_id = ?1 AND is_deleted_by_recipient = 0
            ORDER BY created_at DESC, id DESC
            "#,
        )?;

        let mails: Vec<Mail> = stmt
            .query_map([user_id], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(mails)
    }

    /// List sent mails for a user (not deleted by sender).
    pub fn list_sent(conn: &Connection, user_id: i64) -> rusqlite::Result<Vec<Mail>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, sender_id, recipient_id, subject, body,
                   is_read, is_deleted_by_sender, is_deleted_by_recipient, created_at
            FROM mails
            WHERE sender_id = ?1 AND is_deleted_by_sender = 0
            ORDER BY created_at DESC, id DESC
            "#,
        )?;

        let mails: Vec<Mail> = stmt
            .query_map([user_id], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(mails)
    }

    /// Count unread mails for a user.
    pub fn count_unread(conn: &Connection, user_id: i64) -> rusqlite::Result<i64> {
        conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM mails
            WHERE recipient_id = ?1 AND is_read = 0 AND is_deleted_by_recipient = 0
            "#,
            [user_id],
            |row| row.get(0),
        )
    }

    /// Update a mail.
    pub fn update(conn: &Connection, id: i64, update: &MailUpdate) -> rusqlite::Result<bool> {
        if update.is_empty() {
            return Ok(false);
        }

        let mut sets = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(is_read) = update.is_read {
            sets.push("is_read = ?");
            values.push(Box::new(is_read as i32));
        }

        if let Some(deleted) = update.is_deleted_by_sender {
            sets.push("is_deleted_by_sender = ?");
            values.push(Box::new(deleted as i32));
        }

        if let Some(deleted) = update.is_deleted_by_recipient {
            sets.push("is_deleted_by_recipient = ?");
            values.push(Box::new(deleted as i32));
        }

        values.push(Box::new(id));

        let sql = format!("UPDATE mails SET {} WHERE id = ?", sets.join(", "));

        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
        let rows = conn.execute(&sql, params.as_slice())?;
        Ok(rows > 0)
    }

    /// Mark a mail as read.
    pub fn mark_as_read(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
        Self::update(conn, id, &MailUpdate::new().mark_as_read())
    }

    /// Delete a mail (logical deletion).
    /// Marks the mail as deleted by the specified user.
    pub fn delete_by_user(conn: &Connection, id: i64, user_id: i64) -> rusqlite::Result<bool> {
        // First, get the mail to determine if user is sender or recipient
        let mail = match Self::get_by_id(conn, id)? {
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

        Self::update(conn, id, &update)
    }

    /// Physically delete a mail.
    /// Should only be used for mails that have been deleted by both sender and recipient.
    pub fn purge(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
        let rows = conn.execute("DELETE FROM mails WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Purge all mails that have been deleted by both sender and recipient.
    pub fn purge_all_deleted(conn: &Connection) -> rusqlite::Result<usize> {
        conn.execute(
            "DELETE FROM mails WHERE is_deleted_by_sender = 1 AND is_deleted_by_recipient = 1",
            [],
        )
    }

    /// Count total mails in the database.
    pub fn count(conn: &Connection) -> rusqlite::Result<i64> {
        conn.query_row("SELECT COUNT(*) FROM mails", [], |row| row.get(0))
    }

    /// Map a database row to a Mail.
    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<Mail> {
        let created_at_str: String = row.get(8)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Mail {
            id: row.get(0)?,
            sender_id: row.get(1)?,
            recipient_id: row.get(2)?,
            subject: row.get(3)?,
            body: row.get(4)?,
            is_read: row.get::<_, i32>(5)? != 0,
            is_deleted_by_sender: row.get::<_, i32>(6)? != 0,
            is_deleted_by_recipient: row.get::<_, i32>(7)? != 0,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, NewUser, UserRepository};

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
    fn test_create_mail() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let new_mail = NewMail::new(sender_id, recipient_id, "Hello", "How are you?");
        let mail = MailRepository::create(db.conn(), &new_mail).unwrap();

        assert!(mail.id > 0);
        assert_eq!(mail.sender_id, sender_id);
        assert_eq!(mail.recipient_id, recipient_id);
        assert_eq!(mail.subject, "Hello");
        assert_eq!(mail.body, "How are you?");
        assert!(!mail.is_read);
        assert!(!mail.is_deleted_by_sender);
        assert!(!mail.is_deleted_by_recipient);
    }

    #[test]
    fn test_get_by_id() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let new_mail = NewMail::new(sender_id, recipient_id, "Test", "Body");
        let created = MailRepository::create(db.conn(), &new_mail).unwrap();

        let retrieved = MailRepository::get_by_id(db.conn(), created.id)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.subject, "Test");
    }

    #[test]
    fn test_get_by_id_not_found() {
        let db = setup_db();
        let result = MailRepository::get_by_id(db.conn(), 999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_inbox() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        // Create some mails
        MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail 1", "Body 1"),
        )
        .unwrap();
        MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail 2", "Body 2"),
        )
        .unwrap();

        let inbox = MailRepository::list_inbox(db.conn(), recipient_id).unwrap();
        assert_eq!(inbox.len(), 2);
        // Should be ordered by created_at DESC (most recent first)
        assert_eq!(inbox[0].subject, "Mail 2");
        assert_eq!(inbox[1].subject, "Mail 1");
    }

    #[test]
    fn test_list_inbox_excludes_deleted() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let mail = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        // Delete by recipient
        MailRepository::update(db.conn(), mail.id, &MailUpdate::new().delete_by_recipient())
            .unwrap();

        let inbox = MailRepository::list_inbox(db.conn(), recipient_id).unwrap();
        assert!(inbox.is_empty());
    }

    #[test]
    fn test_list_sent() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Sent Mail", "Body"),
        )
        .unwrap();

        let sent = MailRepository::list_sent(db.conn(), sender_id).unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].subject, "Sent Mail");
    }

    #[test]
    fn test_list_sent_excludes_deleted() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let mail = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        MailRepository::update(db.conn(), mail.id, &MailUpdate::new().delete_by_sender()).unwrap();

        let sent = MailRepository::list_sent(db.conn(), sender_id).unwrap();
        assert!(sent.is_empty());
    }

    #[test]
    fn test_count_unread() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        // Initially no unread
        assert_eq!(
            MailRepository::count_unread(db.conn(), recipient_id).unwrap(),
            0
        );

        // Create two mails
        MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail 1", "Body"),
        )
        .unwrap();
        let mail2 = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail 2", "Body"),
        )
        .unwrap();

        assert_eq!(
            MailRepository::count_unread(db.conn(), recipient_id).unwrap(),
            2
        );

        // Mark one as read
        MailRepository::mark_as_read(db.conn(), mail2.id).unwrap();

        assert_eq!(
            MailRepository::count_unread(db.conn(), recipient_id).unwrap(),
            1
        );
    }

    #[test]
    fn test_mark_as_read() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let mail = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        assert!(!mail.is_read);

        MailRepository::mark_as_read(db.conn(), mail.id).unwrap();

        let updated = MailRepository::get_by_id(db.conn(), mail.id)
            .unwrap()
            .unwrap();
        assert!(updated.is_read);
    }

    #[test]
    fn test_delete_by_user_sender() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let mail = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        MailRepository::delete_by_user(db.conn(), mail.id, sender_id).unwrap();

        let updated = MailRepository::get_by_id(db.conn(), mail.id)
            .unwrap()
            .unwrap();
        assert!(updated.is_deleted_by_sender);
        assert!(!updated.is_deleted_by_recipient);
    }

    #[test]
    fn test_delete_by_user_recipient() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let mail = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        MailRepository::delete_by_user(db.conn(), mail.id, recipient_id).unwrap();

        let updated = MailRepository::get_by_id(db.conn(), mail.id)
            .unwrap()
            .unwrap();
        assert!(!updated.is_deleted_by_sender);
        assert!(updated.is_deleted_by_recipient);
    }

    #[test]
    fn test_delete_by_user_not_involved() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        // Create a third user
        let repo = UserRepository::new(&db);
        let user3 = NewUser::new("charlie", "password123", "Charlie");
        let user3_id = repo.create(&user3).unwrap().id;

        let mail = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        // User3 tries to delete - should fail
        let result = MailRepository::delete_by_user(db.conn(), mail.id, user3_id).unwrap();
        assert!(!result);

        // Mail should be unchanged
        let unchanged = MailRepository::get_by_id(db.conn(), mail.id)
            .unwrap()
            .unwrap();
        assert!(!unchanged.is_deleted_by_sender);
        assert!(!unchanged.is_deleted_by_recipient);
    }

    #[test]
    fn test_purge() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let mail = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        MailRepository::purge(db.conn(), mail.id).unwrap();

        let result = MailRepository::get_by_id(db.conn(), mail.id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_purge_all_deleted() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        // Create three mails
        let mail1 = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail 1", "Body"),
        )
        .unwrap();
        let mail2 = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail 2", "Body"),
        )
        .unwrap();
        let _mail3 = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail 3", "Body"),
        )
        .unwrap();

        // Delete mail1 by both users
        MailRepository::delete_by_user(db.conn(), mail1.id, sender_id).unwrap();
        MailRepository::delete_by_user(db.conn(), mail1.id, recipient_id).unwrap();

        // Delete mail2 only by sender
        MailRepository::delete_by_user(db.conn(), mail2.id, sender_id).unwrap();

        // Purge should only remove mail1
        let purged = MailRepository::purge_all_deleted(db.conn()).unwrap();
        assert_eq!(purged, 1);

        // mail1 should be gone
        assert!(MailRepository::get_by_id(db.conn(), mail1.id)
            .unwrap()
            .is_none());

        // mail2 and mail3 should still exist
        assert!(MailRepository::get_by_id(db.conn(), mail2.id)
            .unwrap()
            .is_some());

        assert_eq!(MailRepository::count(db.conn()).unwrap(), 2);
    }

    #[test]
    fn test_count() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        assert_eq!(MailRepository::count(db.conn()).unwrap(), 0);

        MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        assert_eq!(MailRepository::count(db.conn()).unwrap(), 1);
    }

    #[test]
    fn test_update_empty() {
        let db = setup_db();
        let (sender_id, recipient_id) = create_test_users(&db);

        let mail = MailRepository::create(
            db.conn(),
            &NewMail::new(sender_id, recipient_id, "Mail", "Body"),
        )
        .unwrap();

        let result = MailRepository::update(db.conn(), mail.id, &MailUpdate::new()).unwrap();
        assert!(!result);
    }
}
